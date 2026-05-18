# ReliefChain Soroban

## Description

**ReliefChain Soroban** is an early-stage smart contract MVP built with the **Soroban SDK** for disaster relief coordination.

The project demonstrates how smart contracts can be used to manage disaster events, verify beneficiaries, register aid vendors, issue non-transferable aid vouchers, and track simulated relief fund distribution.

This MVP is designed for learning and experimentation. It is not production-ready and does not yet include real token transfers, DAO approval, oracle integration, or advanced governance.

---

## Testnet Smart Contract ID

```txt
CCGHLSVRIF26IPYSNYBJX76BHKOMTG5CJR5YLFLTQDB3K7MBPZAN7K65
```

---

## Features

### 1. Contract Initialization

The contract can be initialized once with an admin address.

- Stores the admin address
- Prevents repeated initialization
- Uses Soroban address authorization

---

### 2. Role Management

The admin can manage verifier accounts.

Supported functions:

- `add_verifier`
- `remove_verifier`
- `is_verifier`

Verifiers are responsible for registering and verifying disaster relief beneficiaries.

---

### 3. Disaster Event Registry

The admin can create and manage disaster events.

Supported disaster lifecycle:

- `Draft`
- `Active`
- `Closed`
- `Cancelled`

Supported functions:

- `create_disaster`
- `activate_disaster`
- `close_disaster`
- `get_disaster`

Only active disasters can be used for beneficiary registration, voucher issuance, and redemption.

---

### 4. Beneficiary Registry

Verifiers can register and verify beneficiaries for a disaster event.

Supported beneficiary lifecycle:

- `Pending`
- `Verified`
- `Rejected`
- `Assisted`

Supported functions:

- `register_beneficiary`
- `verify_beneficiary`
- `reject_beneficiary`
- `get_beneficiary`
- `get_beneficiary_by_wallet`

The contract prevents the same wallet from being registered more than once for the same disaster.

Sensitive beneficiary data is not stored directly on-chain. Instead, the contract stores a `credential_hash`.

---

### 5. Vendor Registry

Vendors can register themselves, and the admin can approve or suspend them.

Supported vendor lifecycle:

- `Pending`
- `Active`
- `Suspended`
- `Rejected`

Supported functions:

- `register_vendor`
- `approve_vendor`
- `suspend_vendor`
- `get_vendor`

Only active vendors can redeem aid vouchers.

---

### 6. Non-Transferable Aid Voucher

Verifiers can issue internal aid vouchers to verified beneficiaries.

Supported functions:

- `issue_voucher`
- `get_voucher_balance`
- `get_voucher_record`

Voucher properties:

- Linked to a specific disaster
- Linked to a specific beneficiary wallet
- Categorized by aid type, such as `FOOD`, `MED`, or `WATER`
- Has an expiry timestamp
- Cannot be transferred freely between users

---

### 7. Simulated Relief Vault

The MVP includes simulated vault accounting for testing relief fund distribution.

Supported functions:

- `deposit_simulated`
- `get_vault_balance`

This does not transfer real tokens yet. It only tracks internal contract balances for learning and testing purposes.

---

### 8. Vendor Redemption Flow

Approved vendors can redeem vouchers from verified beneficiaries.

Supported functions:

- `redeem_voucher`
- `get_redemption`

Redemption flow:

1. Beneficiary receives voucher.
2. Vendor provides aid.
3. Beneficiary authorizes voucher spending.
4. Vendor redeems the voucher.
5. Voucher balance decreases.
6. Simulated vault balance decreases.
7. Redemption record is stored as `Paid`.

---

## Main Contract Functions

```txt
initialize
get_admin

add_verifier
remove_verifier
is_verifier

create_disaster
activate_disaster
close_disaster
get_disaster

register_beneficiary
verify_beneficiary
reject_beneficiary
get_beneficiary
get_beneficiary_by_wallet

register_vendor
approve_vendor
suspend_vendor
get_vendor

issue_voucher
get_voucher_balance
get_voucher_record

deposit_simulated
get_vault_balance

redeem_voucher
get_redemption
```

---

## Example Manual Invocation

Replace `satar` with your Stellar CLI identity.

```bash
stellar contract invoke \
  --id CCGHLSVRIF26IPYSNYBJX76BHKOMTG5CJR5YLFLTQDB3K7MBPZAN7K65 \
  --source-account satar \
  -- \
  get_admin
```

---

## MVP Scope

This version focuses on the basic disaster relief workflow:

- Admin setup
- Verifier role
- Disaster event creation
- Beneficiary verification
- Vendor approval
- Voucher issuance
- Simulated vault deposit
- Voucher redemption

---

## Out of Scope

The following features are intentionally excluded from the current MVP:

- DAO approval
- Multisig treasury
- Real token transfer
- Oracle-based disaster triggers
- Parametric insurance
- Cross-contract architecture
- Upgradeability
- Frontend application
- Production-grade compliance

---

## Privacy Note

This contract should not store sensitive personal data directly on-chain.

Do not store:

- National ID numbers
- Phone numbers
- Full home addresses
- Medical records
- Raw documents

Use off-chain storage and only store hashes or references on-chain.

---

## Project Status

Current status:

```txt
MVP deployed on testnet
```

Smart contract ID:

```txt
CCGHLSVRIF26IPYSNYBJX76BHKOMTG5CJR5YLFLTQDB3K7MBPZAN7K65
```

---

## Disclaimer

This project is for educational and experimental purposes only. It is not ready for real-world disaster relief operations without further security review, legal review, real asset integration, and production-grade infrastructure.
