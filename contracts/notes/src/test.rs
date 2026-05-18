#![cfg(test)]

extern crate std;

use super::*;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    Address, BytesN, Env, String,
};

fn setup() -> (
    Env,
    ReliefChainClient<'static>,
    Address,
    Address,
    Address,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();

    env.ledger().set_timestamp(1_000);

    let contract_id = env.register_contract(None, ReliefChain);
    let client = ReliefChainClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let verifier = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let vendor = Address::generate(&env);

    client.initialize(&admin);
    client.add_verifier(&admin, &verifier);

    (env, client, admin, verifier, beneficiary, vendor)
}

fn hash32(env: &Env, value: u8) -> BytesN<32> {
    BytesN::from_array(env, &[value; 32])
}

#[test]
fn full_mvp_flow_works() {
    let (env, client, admin, verifier, beneficiary, vendor) = setup();

    let disaster_id = client.create_disaster(
        &admin,
        &String::from_str(&env, "Gempa Cianjur"),
        &hash32(&env, 1),
        &symbol_short!("QUAKE"),
        &1_000,
    );

    client.activate_disaster(&admin, &disaster_id);

    let disaster = client.get_disaster(&disaster_id);
    assert_eq!(disaster.status, DisasterStatus::Active);

    let beneficiary_id = client.register_beneficiary(
        &verifier,
        &disaster_id,
        &beneficiary,
        &hash32(&env, 2),
    );

    client.verify_beneficiary(&verifier, &beneficiary_id);

    let beneficiary_data = client.get_beneficiary(&beneficiary_id);
    assert_eq!(beneficiary_data.status, BeneficiaryStatus::Verified);

    client.register_vendor(
        &vendor,
        &hash32(&env, 3),
        &symbol_short!("FOOD"),
    );

    client.approve_vendor(&admin, &vendor);

    let vendor_data = client.get_vendor(&vendor);
    assert_eq!(vendor_data.status, VendorStatus::Active);

    client.deposit_simulated(&admin, &disaster_id, &1_000);

    let vault_before = client.get_vault_balance(&disaster_id);
    assert_eq!(vault_before, 1_000);

    client.issue_voucher(
        &verifier,
        &beneficiary_id,
        &symbol_short!("FOOD"),
        &300,
        &2_000,
    );

    let voucher_before = client.get_voucher_balance(
        &disaster_id,
        &beneficiary,
        &symbol_short!("FOOD"),
    );

    assert_eq!(voucher_before, 300);

    let redemption_id = client.redeem_voucher(
        &vendor,
        &beneficiary,
        &disaster_id,
        &symbol_short!("FOOD"),
        &100,
        &100,
    );

    let redemption = client.get_redemption(&redemption_id);
    assert_eq!(redemption.status, RedemptionStatus::Paid);
    assert_eq!(redemption.payout_amount, 100);

    let voucher_after = client.get_voucher_balance(
        &disaster_id,
        &beneficiary,
        &symbol_short!("FOOD"),
    );

    assert_eq!(voucher_after, 200);

    let vault_after = client.get_vault_balance(&disaster_id);
    assert_eq!(vault_after, 900);
}

#[test]
fn duplicate_beneficiary_registration_should_fail() {
    let (env, client, admin, verifier, beneficiary, _vendor) = setup();

    let disaster_id = client.create_disaster(
        &admin,
        &String::from_str(&env, "Banjir Jakarta"),
        &hash32(&env, 4),
        &symbol_short!("FLOOD"),
        &1_000,
    );

    client.activate_disaster(&admin, &disaster_id);

    client.register_beneficiary(
        &verifier,
        &disaster_id,
        &beneficiary,
        &hash32(&env, 5),
    );

    let result = client.try_register_beneficiary(
        &verifier,
        &disaster_id,
        &beneficiary,
        &hash32(&env, 6),
    );

    assert!(result.is_err());
}

#[test]
fn cannot_issue_voucher_to_unverified_beneficiary() {
    let (env, client, admin, verifier, beneficiary, _vendor) = setup();

    let disaster_id = client.create_disaster(
        &admin,
        &String::from_str(&env, "Erupsi"),
        &hash32(&env, 7),
        &symbol_short!("ERUPT"),
        &1_000,
    );

    client.activate_disaster(&admin, &disaster_id);

    let beneficiary_id = client.register_beneficiary(
        &verifier,
        &disaster_id,
        &beneficiary,
        &hash32(&env, 8),
    );

    let result = client.try_issue_voucher(
        &verifier,
        &beneficiary_id,
        &symbol_short!("FOOD"),
        &100,
        &2_000,
    );

    assert!(result.is_err());
}

#[test]
fn insufficient_voucher_should_fail() {
    let (env, client, admin, verifier, beneficiary, vendor) = setup();

    let disaster_id = client.create_disaster(
        &admin,
        &String::from_str(&env, "Longsor"),
        &hash32(&env, 9),
        &symbol_short!("SLIDE"),
        &1_000,
    );

    client.activate_disaster(&admin, &disaster_id);

    let beneficiary_id = client.register_beneficiary(
        &verifier,
        &disaster_id,
        &beneficiary,
        &hash32(&env, 10),
    );

    client.verify_beneficiary(&verifier, &beneficiary_id);

    client.register_vendor(
        &vendor,
        &hash32(&env, 11),
        &symbol_short!("FOOD"),
    );

    client.approve_vendor(&admin, &vendor);

    client.deposit_simulated(&admin, &disaster_id, &1_000);

    client.issue_voucher(
        &verifier,
        &beneficiary_id,
        &symbol_short!("FOOD"),
        &50,
        &2_000,
    );

    let result = client.try_redeem_voucher(
        &vendor,
        &beneficiary,
        &disaster_id,
        &symbol_short!("FOOD"),
        &100,
        &100,
    );

    assert!(result.is_err());
}

#[test]
fn insufficient_vault_balance_should_fail() {
    let (env, client, admin, verifier, beneficiary, vendor) = setup();

    let disaster_id = client.create_disaster(
        &admin,
        &String::from_str(&env, "Tsunami"),
        &hash32(&env, 12),
        &symbol_short!("TSUNAMI"),
        &1_000,
    );

    client.activate_disaster(&admin, &disaster_id);

    let beneficiary_id = client.register_beneficiary(
        &verifier,
        &disaster_id,
        &beneficiary,
        &hash32(&env, 13),
    );

    client.verify_beneficiary(&verifier, &beneficiary_id);

    client.register_vendor(
        &vendor,
        &hash32(&env, 14),
        &symbol_short!("FOOD"),
    );

    client.approve_vendor(&admin, &vendor);

    client.deposit_simulated(&admin, &disaster_id, &50);

    client.issue_voucher(
        &verifier,
        &beneficiary_id,
        &symbol_short!("FOOD"),
        &500,
        &2_000,
    );

    let result = client.try_redeem_voucher(
        &vendor,
        &beneficiary,
        &disaster_id,
        &symbol_short!("FOOD"),
        &100,
        &100,
    );

    assert!(result.is_err());
}