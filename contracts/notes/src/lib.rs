#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, BytesN, Env,
    String, Symbol,
};

#[contract]
pub struct ReliefChain;

// ============================================================
// Storage Keys
// ============================================================

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    Verifier(Address),

    NextDisasterId,
    NextBeneficiaryId,
    NextRedemptionId,

    Disaster(u64),
    Beneficiary(u64),
    BeneficiaryByWallet(u64, Address),

    Vendor(Address),

    VoucherBalance(u64, Address, Symbol),

    VaultBalance(u64),

    Redemption(u64),
}

// ============================================================
// Errors
// ============================================================

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,

    DisasterNotFound = 10,
    DisasterNotActive = 11,

    BeneficiaryNotFound = 20,
    BeneficiaryAlreadyRegistered = 21,
    BeneficiaryNotVerified = 22,

    VendorNotFound = 30,
    VendorNotActive = 31,

    InsufficientVoucher = 40,
    InsufficientVaultBalance = 41,

    RedemptionNotFound = 50,

    InvalidStatus = 60,
    InvalidAmount = 61,
    ExpiredVoucher = 62,
}

// ============================================================
// Domain Types
// ============================================================

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisasterStatus {
    Draft,
    Active,
    Closed,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisasterEvent {
    pub id: u64,
    pub name: String,
    pub location_hash: BytesN<32>,
    pub disaster_type: Symbol,
    pub start_time: u64,
    pub status: DisasterStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BeneficiaryStatus {
    Pending,
    Verified,
    Rejected,
    Assisted,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Beneficiary {
    pub id: u64,
    pub wallet: Address,
    pub disaster_id: u64,
    pub credential_hash: BytesN<32>,
    pub status: BeneficiaryStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VendorStatus {
    Pending,
    Active,
    Suspended,
    Rejected,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Vendor {
    pub wallet: Address,
    pub metadata_hash: BytesN<32>,
    pub category: Symbol,
    pub status: VendorStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VoucherBalance {
    pub amount: i128,
    pub expiry: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RedemptionStatus {
    Pending,
    Paid,
    Disputed,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Redemption {
    pub id: u64,
    pub disaster_id: u64,
    pub vendor: Address,
    pub beneficiary: Address,
    pub category: Symbol,
    pub voucher_amount: i128,
    pub payout_amount: i128,
    pub status: RedemptionStatus,
    pub created_at: u64,
}

// ============================================================
// Internal Helpers
// ============================================================

impl ReliefChain {
    fn read_admin(env: &Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)
    }

    fn require_admin(env: &Env, admin: &Address) -> Result<(), Error> {
        admin.require_auth();

        let stored_admin = Self::read_admin(env)?;

        if stored_admin != admin.clone() {
            return Err(Error::Unauthorized);
        }

        Ok(())
    }

    fn require_verifier(env: &Env, verifier: &Address) -> Result<(), Error> {
        verifier.require_auth();

        let is_verifier: bool = env
            .storage()
            .persistent()
            .get(&DataKey::Verifier(verifier.clone()))
            .unwrap_or(false);

        if !is_verifier {
            return Err(Error::Unauthorized);
        }

        Ok(())
    }

    fn next_disaster_id(env: &Env) -> u64 {
        let current: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextDisasterId)
            .unwrap_or(0);

        let next = current + 1;

        env.storage()
            .instance()
            .set(&DataKey::NextDisasterId, &next);

        next
    }

    fn next_beneficiary_id(env: &Env) -> u64 {
        let current: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextBeneficiaryId)
            .unwrap_or(0);

        let next = current + 1;

        env.storage()
            .instance()
            .set(&DataKey::NextBeneficiaryId, &next);

        next
    }

    fn next_redemption_id(env: &Env) -> u64 {
        let current: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextRedemptionId)
            .unwrap_or(0);

        let next = current + 1;

        env.storage()
            .instance()
            .set(&DataKey::NextRedemptionId, &next);

        next
    }

    fn require_disaster_exists(env: &Env, disaster_id: u64) -> Result<DisasterEvent, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Disaster(disaster_id))
            .ok_or(Error::DisasterNotFound)
    }

    fn require_active_disaster(env: &Env, disaster_id: u64) -> Result<DisasterEvent, Error> {
        let disaster = Self::require_disaster_exists(env, disaster_id)?;

        if disaster.status != DisasterStatus::Active {
            return Err(Error::DisasterNotActive);
        }

        Ok(disaster)
    }

    fn require_beneficiary_exists(
        env: &Env,
        beneficiary_id: u64,
    ) -> Result<Beneficiary, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Beneficiary(beneficiary_id))
            .ok_or(Error::BeneficiaryNotFound)
    }

    fn require_beneficiary_by_wallet(
        env: &Env,
        disaster_id: u64,
        wallet: &Address,
    ) -> Result<Beneficiary, Error> {
        let beneficiary_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::BeneficiaryByWallet(disaster_id, wallet.clone()))
            .ok_or(Error::BeneficiaryNotFound)?;

        Self::require_beneficiary_exists(env, beneficiary_id)
    }

    fn require_verified_beneficiary(
        env: &Env,
        disaster_id: u64,
        wallet: &Address,
    ) -> Result<Beneficiary, Error> {
        let beneficiary = Self::require_beneficiary_by_wallet(env, disaster_id, wallet)?;

        if beneficiary.status != BeneficiaryStatus::Verified {
            return Err(Error::BeneficiaryNotVerified);
        }

        Ok(beneficiary)
    }

    fn require_vendor_exists(env: &Env, vendor: &Address) -> Result<Vendor, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Vendor(vendor.clone()))
            .ok_or(Error::VendorNotFound)
    }

    fn require_active_vendor(env: &Env, vendor: &Address) -> Result<Vendor, Error> {
        let vendor_data = Self::require_vendor_exists(env, vendor)?;

        if vendor_data.status != VendorStatus::Active {
            return Err(Error::VendorNotActive);
        }

        Ok(vendor_data)
    }

    fn get_vault_balance_internal(env: &Env, disaster_id: u64) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::VaultBalance(disaster_id))
            .unwrap_or(0)
    }

    fn get_voucher_record_internal(
        env: &Env,
        disaster_id: u64,
        beneficiary: &Address,
        category: &Symbol,
    ) -> VoucherBalance {
        env.storage()
            .persistent()
            .get(&DataKey::VoucherBalance(
                disaster_id,
                beneficiary.clone(),
                category.clone(),
            ))
            .unwrap_or(VoucherBalance {
                amount: 0,
                expiry: 0,
            })
    }

    fn spend_voucher_internal(
        env: &Env,
        disaster_id: u64,
        beneficiary: &Address,
        vendor: &Address,
        category: &Symbol,
        amount: i128,
    ) -> Result<(), Error> {
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let key = DataKey::VoucherBalance(
            disaster_id,
            beneficiary.clone(),
            category.clone(),
        );

        let mut record: VoucherBalance = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::InsufficientVoucher)?;

        if record.expiry <= env.ledger().timestamp() {
            return Err(Error::ExpiredVoucher);
        }

        if record.amount < amount {
            return Err(Error::InsufficientVoucher);
        }

        record.amount -= amount;

        env.storage().persistent().set(&key, &record);

        env.events().publish(
            (symbol_short!("v_spent"), disaster_id),
            (
                beneficiary.clone(),
                vendor.clone(),
                category.clone(),
                amount,
            ),
        );

        Ok(())
    }

    fn release_to_vendor_internal(
        env: &Env,
        disaster_id: u64,
        vendor: &Address,
        amount: i128,
    ) -> Result<(), Error> {
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let key = DataKey::VaultBalance(disaster_id);
        let balance = Self::get_vault_balance_internal(env, disaster_id);

        if balance < amount {
            return Err(Error::InsufficientVaultBalance);
        }

        let new_balance = balance - amount;

        env.storage().persistent().set(&key, &new_balance);

        env.events().publish(
            (symbol_short!("vault_rel"), disaster_id),
            (vendor.clone(), amount),
        );

        Ok(())
    }
}

// ============================================================
// Public Contract API
// ============================================================

#[contractimpl]
impl ReliefChain {
    // ------------------------------------------------------------
    // Initialization & Roles
    // ------------------------------------------------------------

    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }

        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);

        env.events()
            .publish((symbol_short!("init"),), admin.clone());

        Ok(())
    }

    pub fn get_admin(env: Env) -> Result<Address, Error> {
        Self::read_admin(&env)
    }

    pub fn add_verifier(
        env: Env,
        admin: Address,
        verifier: Address,
    ) -> Result<(), Error> {
        Self::require_admin(&env, &admin)?;

        env.storage()
            .persistent()
            .set(&DataKey::Verifier(verifier.clone()), &true);

        env.events()
            .publish((symbol_short!("v_add"),), verifier.clone());

        Ok(())
    }

    pub fn remove_verifier(
        env: Env,
        admin: Address,
        verifier: Address,
    ) -> Result<(), Error> {
        Self::require_admin(&env, &admin)?;

        env.storage()
            .persistent()
            .set(&DataKey::Verifier(verifier.clone()), &false);

        env.events()
            .publish((symbol_short!("v_remove"),), verifier.clone());

        Ok(())
    }

    pub fn is_verifier(env: Env, verifier: Address) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::Verifier(verifier))
            .unwrap_or(false)
    }

    // ------------------------------------------------------------
    // Disaster Registry
    // ------------------------------------------------------------

    pub fn create_disaster(
        env: Env,
        admin: Address,
        name: String,
        location_hash: BytesN<32>,
        disaster_type: Symbol,
        start_time: u64,
    ) -> Result<u64, Error> {
        Self::require_admin(&env, &admin)?;

        let disaster_id = Self::next_disaster_id(&env);

        let disaster = DisasterEvent {
            id: disaster_id,
            name,
            location_hash,
            disaster_type,
            start_time,
            status: DisasterStatus::Draft,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Disaster(disaster_id), &disaster);

        env.events()
            .publish((symbol_short!("d_new"),), disaster_id);

        Ok(disaster_id)
    }

    pub fn activate_disaster(
        env: Env,
        admin: Address,
        disaster_id: u64,
    ) -> Result<(), Error> {
        Self::require_admin(&env, &admin)?;

        let mut disaster = Self::require_disaster_exists(&env, disaster_id)?;

        disaster.status = DisasterStatus::Active;

        env.storage()
            .persistent()
            .set(&DataKey::Disaster(disaster_id), &disaster);

        env.events()
            .publish((symbol_short!("d_active"),), disaster_id);

        Ok(())
    }

    pub fn close_disaster(
        env: Env,
        admin: Address,
        disaster_id: u64,
    ) -> Result<(), Error> {
        Self::require_admin(&env, &admin)?;

        let mut disaster = Self::require_disaster_exists(&env, disaster_id)?;

        disaster.status = DisasterStatus::Closed;

        env.storage()
            .persistent()
            .set(&DataKey::Disaster(disaster_id), &disaster);

        env.events()
            .publish((symbol_short!("d_close"),), disaster_id);

        Ok(())
    }

    pub fn get_disaster(
        env: Env,
        disaster_id: u64,
    ) -> Result<DisasterEvent, Error> {
        Self::require_disaster_exists(&env, disaster_id)
    }

    // ------------------------------------------------------------
    // Beneficiary Registry
    // ------------------------------------------------------------

    pub fn register_beneficiary(
        env: Env,
        verifier: Address,
        disaster_id: u64,
        wallet: Address,
        credential_hash: BytesN<32>,
    ) -> Result<u64, Error> {
        Self::require_verifier(&env, &verifier)?;
        Self::require_active_disaster(&env, disaster_id)?;

        let duplicate_key =
            DataKey::BeneficiaryByWallet(disaster_id, wallet.clone());

        if env.storage().persistent().has(&duplicate_key) {
            return Err(Error::BeneficiaryAlreadyRegistered);
        }

        let beneficiary_id = Self::next_beneficiary_id(&env);

        let beneficiary = Beneficiary {
            id: beneficiary_id,
            wallet: wallet.clone(),
            disaster_id,
            credential_hash,
            status: BeneficiaryStatus::Pending,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Beneficiary(beneficiary_id), &beneficiary);

        env.storage()
            .persistent()
            .set(&duplicate_key, &beneficiary_id);

        env.events().publish(
            (symbol_short!("ben_reg"), disaster_id),
            (beneficiary_id, wallet.clone()),
        );

        Ok(beneficiary_id)
    }

    pub fn verify_beneficiary(
        env: Env,
        verifier: Address,
        beneficiary_id: u64,
    ) -> Result<(), Error> {
        Self::require_verifier(&env, &verifier)?;

        let mut beneficiary =
            Self::require_beneficiary_exists(&env, beneficiary_id)?;

        if beneficiary.status != BeneficiaryStatus::Pending {
            return Err(Error::InvalidStatus);
        }

        beneficiary.status = BeneficiaryStatus::Verified;

        env.storage()
            .persistent()
            .set(&DataKey::Beneficiary(beneficiary_id), &beneficiary);

        env.events()
            .publish((symbol_short!("ben_ok"),), beneficiary_id);

        Ok(())
    }

    pub fn reject_beneficiary(
        env: Env,
        verifier: Address,
        beneficiary_id: u64,
    ) -> Result<(), Error> {
        Self::require_verifier(&env, &verifier)?;

        let mut beneficiary =
            Self::require_beneficiary_exists(&env, beneficiary_id)?;

        if beneficiary.status != BeneficiaryStatus::Pending {
            return Err(Error::InvalidStatus);
        }

        beneficiary.status = BeneficiaryStatus::Rejected;

        env.storage()
            .persistent()
            .set(&DataKey::Beneficiary(beneficiary_id), &beneficiary);

        env.events()
            .publish((symbol_short!("ben_rej"),), beneficiary_id);

        Ok(())
    }

    pub fn get_beneficiary(
        env: Env,
        beneficiary_id: u64,
    ) -> Result<Beneficiary, Error> {
        Self::require_beneficiary_exists(&env, beneficiary_id)
    }

    pub fn get_beneficiary_by_wallet(
        env: Env,
        disaster_id: u64,
        wallet: Address,
    ) -> Result<Beneficiary, Error> {
        Self::require_beneficiary_by_wallet(&env, disaster_id, &wallet)
    }

    // ------------------------------------------------------------
    // Vendor Registry
    // ------------------------------------------------------------

    pub fn register_vendor(
        env: Env,
        vendor: Address,
        metadata_hash: BytesN<32>,
        category: Symbol,
    ) -> Result<(), Error> {
        vendor.require_auth();

        let key = DataKey::Vendor(vendor.clone());

        if env.storage().persistent().has(&key) {
            return Err(Error::InvalidStatus);
        }

        let vendor_data = Vendor {
            wallet: vendor.clone(),
            metadata_hash,
            category,
            status: VendorStatus::Pending,
        };

        env.storage().persistent().set(&key, &vendor_data);

        env.events()
            .publish((symbol_short!("ven_reg"),), vendor.clone());

        Ok(())
    }

    pub fn approve_vendor(
        env: Env,
        admin: Address,
        vendor: Address,
    ) -> Result<(), Error> {
        Self::require_admin(&env, &admin)?;

        let mut vendor_data = Self::require_vendor_exists(&env, &vendor)?;

        vendor_data.status = VendorStatus::Active;

        env.storage()
            .persistent()
            .set(&DataKey::Vendor(vendor.clone()), &vendor_data);

        env.events()
            .publish((symbol_short!("ven_ok"),), vendor.clone());

        Ok(())
    }

    pub fn suspend_vendor(
        env: Env,
        admin: Address,
        vendor: Address,
    ) -> Result<(), Error> {
        Self::require_admin(&env, &admin)?;

        let mut vendor_data = Self::require_vendor_exists(&env, &vendor)?;

        vendor_data.status = VendorStatus::Suspended;

        env.storage()
            .persistent()
            .set(&DataKey::Vendor(vendor.clone()), &vendor_data);

        env.events()
            .publish((symbol_short!("ven_stop"),), vendor.clone());

        Ok(())
    }

    pub fn get_vendor(env: Env, vendor: Address) -> Result<Vendor, Error> {
        Self::require_vendor_exists(&env, &vendor)
    }

    // ------------------------------------------------------------
    // Aid Voucher
    // ------------------------------------------------------------

    pub fn issue_voucher(
        env: Env,
        verifier: Address,
        beneficiary_id: u64,
        category: Symbol,
        amount: i128,
        expiry: u64,
    ) -> Result<(), Error> {
        Self::require_verifier(&env, &verifier)?;

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        if expiry <= env.ledger().timestamp() {
            return Err(Error::ExpiredVoucher);
        }

        let beneficiary =
            Self::require_beneficiary_exists(&env, beneficiary_id)?;

        if beneficiary.status != BeneficiaryStatus::Verified {
            return Err(Error::BeneficiaryNotVerified);
        }

        Self::require_active_disaster(&env, beneficiary.disaster_id)?;

        let key = DataKey::VoucherBalance(
            beneficiary.disaster_id,
            beneficiary.wallet.clone(),
            category.clone(),
        );

        let current: VoucherBalance = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(VoucherBalance {
                amount: 0,
                expiry,
            });

        let new_expiry = if expiry > current.expiry {
            expiry
        } else {
            current.expiry
        };

        let updated = VoucherBalance {
            amount: current.amount + amount,
            expiry: new_expiry,
        };

        env.storage().persistent().set(&key, &updated);

        env.events().publish(
            (symbol_short!("v_issue"), beneficiary.disaster_id),
            (beneficiary.wallet.clone(), category.clone(), amount),
        );

        Ok(())
    }

    pub fn get_voucher_balance(
        env: Env,
        disaster_id: u64,
        beneficiary: Address,
        category: Symbol,
    ) -> i128 {
        let record = Self::get_voucher_record_internal(
            &env,
            disaster_id,
            &beneficiary,
            &category,
        );

        if record.expiry <= env.ledger().timestamp() {
            return 0;
        }

        record.amount
    }

    pub fn get_voucher_record(
        env: Env,
        disaster_id: u64,
        beneficiary: Address,
        category: Symbol,
    ) -> VoucherBalance {
        Self::get_voucher_record_internal(
            &env,
            disaster_id,
            &beneficiary,
            &category,
        )
    }

    // ------------------------------------------------------------
    // Relief Vault - Simulated Accounting
    // ------------------------------------------------------------

    pub fn deposit_simulated(
        env: Env,
        admin: Address,
        disaster_id: u64,
        amount: i128,
    ) -> Result<(), Error> {
        Self::require_admin(&env, &admin)?;

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        Self::require_active_disaster(&env, disaster_id)?;

        let current = Self::get_vault_balance_internal(&env, disaster_id);
        let updated = current + amount;

        env.storage()
            .persistent()
            .set(&DataKey::VaultBalance(disaster_id), &updated);

        env.events()
            .publish((symbol_short!("vault_in"), disaster_id), amount);

        Ok(())
    }

    pub fn get_vault_balance(env: Env, disaster_id: u64) -> i128 {
        Self::get_vault_balance_internal(&env, disaster_id)
    }

    // ------------------------------------------------------------
    // Redemption
    // ------------------------------------------------------------

    pub fn redeem_voucher(
        env: Env,
        vendor: Address,
        beneficiary_wallet: Address,
        disaster_id: u64,
        category: Symbol,
        voucher_amount: i128,
        payout_amount: i128,
    ) -> Result<u64, Error> {
        vendor.require_auth();
        beneficiary_wallet.require_auth();

        if voucher_amount <= 0 || payout_amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        Self::require_active_disaster(&env, disaster_id)?;

        let vendor_data = Self::require_active_vendor(&env, &vendor)?;

        if vendor_data.category != category {
            return Err(Error::InvalidStatus);
        }

        let beneficiary = Self::require_verified_beneficiary(
            &env,
            disaster_id,
            &beneficiary_wallet,
        )?;

        let voucher_record = Self::get_voucher_record_internal(
            &env,
            disaster_id,
            &beneficiary_wallet,
            &category,
        );

        if voucher_record.expiry <= env.ledger().timestamp() {
            return Err(Error::ExpiredVoucher);
        }

        if voucher_record.amount < voucher_amount {
            return Err(Error::InsufficientVoucher);
        }

        let vault_balance = Self::get_vault_balance_internal(&env, disaster_id);

        if vault_balance < payout_amount {
            return Err(Error::InsufficientVaultBalance);
        }

        Self::spend_voucher_internal(
            &env,
            disaster_id,
            &beneficiary_wallet,
            &vendor,
            &category,
            voucher_amount,
        )?;

        Self::release_to_vendor_internal(
            &env,
            disaster_id,
            &vendor,
            payout_amount,
        )?;

        let redemption_id = Self::next_redemption_id(&env);

        let redemption = Redemption {
            id: redemption_id,
            disaster_id,
            vendor: vendor.clone(),
            beneficiary: beneficiary.wallet.clone(),
            category: category.clone(),
            voucher_amount,
            payout_amount,
            status: RedemptionStatus::Paid,
            created_at: env.ledger().timestamp(),
        };

        env.storage()
            .persistent()
            .set(&DataKey::Redemption(redemption_id), &redemption);

        env.events().publish(
            (symbol_short!("red_paid"), disaster_id),
            (
                redemption_id,
                vendor.clone(),
                beneficiary.wallet.clone(),
                payout_amount,
            ),
        );

        Ok(redemption_id)
    }

    pub fn get_redemption(
        env: Env,
        redemption_id: u64,
    ) -> Result<Redemption, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Redemption(redemption_id))
            .ok_or(Error::RedemptionNotFound)
    }
}

#[cfg(test)]
mod test;