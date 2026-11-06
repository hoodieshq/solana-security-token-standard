# Security Policy

## Reporting a Vulnerability

For security disclosures, use the contact methods listed in:

- `https://ssts.org/.well-known/security.txt`

Do not publish exploit details in public issues before coordinated disclosure.

## Scope

This policy covers this repository and the deployed core programs:

- Security Token Program
- Security Token Transfer Hook

## Program-owned mint authorities

SSTS Core assigns the mint, freeze, pause, permanent delegate, transfer hook
and, when enabled, Permissioned Burn authorities of each mint it initializes to
PDAs derived from the Core program ID. SSTS exposes no instruction that moves
these authorities to another address.

Each mint therefore stays bound to the Core program that created it. Upgrades
keep the program ID, so an upgraded Core can still sign for existing mints; a
Core deployed under a different program ID cannot. If the original Core
program is closed, or made immutable with a defect in an operation that needs
these authorities, that operation cannot be restored for existing mints, and
the remaining option is migrating holders to a new mint.

## Disclosure Process

1. Send a report using the contact path from `/.well-known/security.txt`.
2. Include reproduction steps, impact, and affected cluster/program IDs if known.
3. We will triage and coordinate remediation before public disclosure.

## Bug Bounty

No standing public bug bounty is currently guaranteed by this repository policy.
Any reward decisions are discretionary.
