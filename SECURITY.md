# Security Policy

## Reporting a Vulnerability

For security disclosures, use the contact methods listed in:

- `https://ssts.org/.well-known/security.txt`

Do not publish exploit details in public issues before coordinated disclosure.

## Scope

This policy covers this repository and the deployed core programs:

- Security Token Program
- Security Token Transfer Hook

## Permissioned Burn authority and upgrades

Enabling `ix_permissioned_burn = true` during mint initialization sets the
Token-2022 Permissioned Burn authority to SSTS Core's permanent-delegate PDA.
That address depends on the mint, PDA seeds, and Core program ID.

Token-2022 v11 supports rotating or clearing this authority through
`SetAuthority` (`AuthorityType::PermissionedBurn`, value 17), authorized by the
current authority. See the [pinned v11 implementation](https://github.com/solana-program/token-2022/blob/9bc0275/program/src/processor.rs#L996).
SSTS currently exposes no operation that signs this authority change for its
PDA. An issuer or token holder cannot sign for that PDA directly.

Operators must preserve the ability to sign with existing permanent-delegate
PDAs when upgrading Core. New seeds can coexist with support for the original
derivation. Deploying Core under a different program ID does not transfer
control of existing mint authorities: a rotation would require an authorized
CPI from the original program, for example through an audited upgrade.

If the original program can no longer sign and cannot be upgraded to restore
that ability, SSTS has no built-in recovery path for protected burns. Document
PDA compatibility and a token migration plan before enabling the protection.

SSTS falls back to native `BurnChecked` when the extension is absent or its
authority is zero. Token-2022 permits native burns after the authority is
cleared, so that fallback preserves burn, split, and convert compatibility.
It does not provide an issuer-facing way to disable protection through the
current SSTS API.

## Disclosure Process

1. Send a report using the contact path from `/.well-known/security.txt`.
2. Include reproduction steps, impact, and affected cluster/program IDs if known.
3. We will triage and coordinate remediation before public disclosure.

## Bug Bounty

No standing public bug bounty is currently guaranteed by this repository policy.
Any reward decisions are discretionary.
