use crate::{
    helpers::{
        assert_security_token_error, assert_transaction_success,
        create_minimal_security_token_mint, create_spl_account, find_verification_config_pda,
        get_mint_state, get_token_account_state, send_tx,
    },
    verification_tests::verification_helpers::dynamic_initialize_config_instruction_with_mode,
};
use rstest::rstest;
use security_token_client::{
    errors::SecurityTokenProgramError,
    instructions::{MintBuilder, MINT_DISCRIMINATOR},
    programs::SECURITY_TOKEN_PROGRAM_ID,
};
use security_token_program::instructions::{VerificationAccountMeta, VerificationProgramConfig};
use solana_program_test::{processor, ProgramTest};
use solana_sdk::{
    account::Account,
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::invoke,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    sysvar,
};

fn counting_verifier(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    assert_eq!(instruction_data[0], MINT_DISCRIMINATOR);
    assert_eq!(accounts.len(), 5);
    let counter = &accounts[4];
    assert_eq!(counter.owner, program_id);
    let mut data = counter.try_borrow_mut_data()?;
    let count = u64::from_le_bytes(data[..8].try_into().unwrap());
    data.copy_from_slice(&(count + 1).to_le_bytes());
    Ok(())
}

fn repeated_mint_wrapper(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    assert_eq!(accounts[0].key, &SECURITY_TOKEN_PROGRAM_ID);
    let mint = Instruction {
        program_id: SECURITY_TOKEN_PROGRAM_ID,
        accounts: accounts[1..]
            .iter()
            .map(|account| AccountMeta {
                pubkey: *account.key,
                is_signer: account.is_signer,
                is_writable: account.is_writable,
            })
            .collect(),
        data: instruction_data[1..].to_vec(),
    };
    for _ in 0..instruction_data[0] {
        invoke(&mint, accounts)?;
    }
    Ok(())
}

async fn run_mint(cpi_mode: bool, wrapped_calls: Option<u8>) {
    let verifier = Pubkey::new_unique();
    let wrapper = Pubkey::new_unique();
    let counter = Pubkey::new_unique();
    // Core always runs from the SBF artifact, so the guard uses the real stack-height syscall.
    let mut pt = ProgramTest::new("security_token_program", SECURITY_TOKEN_PROGRAM_ID, None);
    pt.prefer_bpf(false);
    pt.add_program("counting_verifier", verifier, processor!(counting_verifier));
    pt.add_program("mint_wrapper", wrapper, processor!(repeated_mint_wrapper));
    pt.add_account(
        counter,
        Account {
            lamports: 1_000_000,
            data: vec![0; 8],
            owner: verifier,
            executable: false,
            rent_epoch: 0,
        },
    );
    let mut context = pt.start_with_context().await;
    let mint = Keypair::new();
    let owner = Keypair::new();
    let (mint_authority, _) =
        create_minimal_security_token_mint(&mut context, &mint, None, 6).await;
    let config = find_verification_config_pda(mint.pubkey(), MINT_DISCRIMINATOR).0;
    let initialize = dynamic_initialize_config_instruction_with_mode(
        context.payer.pubkey(),
        mint.pubkey(),
        mint_authority,
        config,
        MINT_DISCRIMINATOR,
        cpi_mode,
        vec![VerificationProgramConfig {
            program_id: verifier.to_bytes(),
            extra_accounts: vec![VerificationAccountMeta {
                discriminator: 0,
                address_config: counter.to_bytes(),
                is_signer: false,
                is_writable: true,
            }],
        }],
    );
    assert_transaction_success(
        send_tx(
            &context.banks_client,
            vec![initialize],
            &context.payer.pubkey(),
            vec![&context.payer],
        )
        .await,
    );
    let destination = create_spl_account(&mut context, &mint, &owner).await;
    let amount = 1_000;
    let mint_instruction = MintBuilder::new()
        .mint(mint.pubkey())
        .verification_config(config)
        .instructions_sysvar(sysvar::instructions::ID)
        .mint_authority(mint_authority)
        .mint_account(mint.pubkey())
        .destination(destination)
        .amount(amount)
        .add_remaining_accounts(&[
            AccountMeta::new_readonly(verifier, false),
            AccountMeta::new(counter, false),
        ])
        .instruction();
    let mut instructions = Vec::new();
    if !cpi_mode {
        // Four canonical Mint accounts followed by the verifier's own counter extra.
        let mut accounts = mint_instruction.accounts[3..7].to_vec();
        accounts.push(AccountMeta::new(counter, false));
        instructions.push(Instruction {
            program_id: verifier,
            accounts,
            data: mint_instruction.data.clone(),
        });
    }
    instructions.push(if let Some(calls) = wrapped_calls {
        let mut accounts = vec![AccountMeta::new_readonly(SECURITY_TOKEN_PROGRAM_ID, false)];
        accounts.extend(mint_instruction.accounts);
        Instruction {
            program_id: wrapper,
            accounts,
            data: [vec![calls], mint_instruction.data].concat(),
        }
    } else {
        mint_instruction
    });
    let result = send_tx(
        &context.banks_client,
        instructions,
        &context.payer.pubkey(),
        vec![&context.payer],
    )
    .await;

    let rejected = !cpi_mode && wrapped_calls.is_some();
    let expected_calls = if rejected {
        0
    } else {
        wrapped_calls.unwrap_or(1) as u64
    };
    let supply = get_mint_state(&mut context.banks_client, mint.pubkey())
        .await
        .base
        .supply;
    let balance = get_token_account_state(&mut context.banks_client, destination)
        .await
        .base
        .amount;
    let counter_account = context
        .banks_client
        .get_account(counter)
        .await
        .unwrap()
        .unwrap();
    let verifier_calls = u64::from_le_bytes(counter_account.data.try_into().unwrap());
    assert_eq!(supply, expected_calls * amount, "unexpected mint supply");
    assert_eq!(
        balance,
        expected_calls * amount,
        "unexpected destination balance"
    );
    assert_eq!(
        verifier_calls, expected_calls,
        "each successful Mint needs its own verification"
    );
    if rejected {
        assert_security_token_error(
            result,
            SecurityTokenProgramError::VerificationProgramNotFound,
        );
    } else {
        assert_transaction_success(result);
    }
}

#[rstest]
#[case(1)]
#[case(2)]
#[tokio::test]
async fn introspection_rejects_wrapped_mint(#[case] calls: u8) {
    run_mint(false, Some(calls)).await;
}

#[tokio::test]
async fn introspection_accepts_top_level_mint() {
    run_mint(false, None).await;
}

#[tokio::test]
async fn cpi_mode_verifies_each_wrapped_mint() {
    run_mint(true, Some(2)).await;
}
