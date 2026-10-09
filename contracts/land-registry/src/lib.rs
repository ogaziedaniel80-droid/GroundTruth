//! # GroundTruth — Land Registry Contract
//!
//! Tamper-evident anchoring layer for land title records on Stellar/Soroban.
//!
//! This contract stores cryptographic hashes of title documents (never the
//! documents themselves), GPS parcel coordinates, and a full on-chain history
//! of every ownership change. The government registry remains the legal source
//! of truth; this contract is the tamper-evident witness standing next to it.
//!
//! **Full specification:** see `README.md` in the repository root.
//!
//! ## Public interface (matches README "Smart contract interface" section)
//! - Admin/setup: `initialize`, `add_registrar`, `remove_registrar`, `set_threshold`
//! - Title lifecycle: `register_title`, `flag_dispute`, `resolve_dispute`
//! - Transfer flow: `initiate_transfer`, `co_sign_transfer`, `execute_transfer`, `cancel_transfer`
//! - Read-only: `get_title`, `get_pending_transfer`, `verify_hash`

pub mod types;
pub mod errors;
pub mod registrar;

use soroban_sdk::{Address, BytesN, Env, String, Vec, contract, contractimpl, symbol_short};

use crate::types::{DataKey, TitleRecord, TitleStatus};
use crate::errors::ContractError;
use crate::registrar::{
    add_registrar_internal, get_threshold, is_registrar, remove_registrar_internal,
    validate_threshold, set_threshold,
};

#[contract]
pub struct LandRegistry;

#[contractimpl]
impl LandRegistry {
    // -------------------------------------------------------------------------
    // Admin / setup
    // -------------------------------------------------------------------------

    pub fn initialize(
        env: Env,
        admin: Address,
        registrars: Vec<Address>,
        threshold: u32,
    ) -> Result<(), ContractError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(ContractError::AlreadyInitialized);
        }

        if registrars.is_empty() {
            return Err(ContractError::InvalidThreshold);
        }

        if threshold == 0 || threshold > registrars.len() as u32 {
            return Err(ContractError::InvalidThreshold);
        }

        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Registrars, &registrars);
        env.storage().instance().set(&DataKey::Threshold, &threshold);

        Ok(())
    }

    pub fn add_registrar(env: Env, registrar: Address) -> Result<(), ContractError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(ContractError::NotInitialized)?;
        admin.require_auth();

        add_registrar_internal(&env, registrar)
    }

    pub fn remove_registrar(env: Env, registrar: Address) -> Result<(), ContractError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(ContractError::NotInitialized)?;
        admin.require_auth();

        remove_registrar_internal(&env, &registrar)
    }

    pub fn set_threshold(env: Env, threshold: u32) -> Result<(), ContractError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(ContractError::NotInitialized)?;
        admin.require_auth();

        validate_threshold(&env, threshold)?;
        set_threshold(&env, threshold);

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Title lifecycle
    // -------------------------------------------------------------------------

    /// Register a new title document on-chain.
    ///
    /// Only a current registrar may anchor a title. The `id` must be unique;
    /// passing a duplicate returns `TitleAlreadyExists`. `doc_hash` is the
    /// SHA-256 of the off-chain document. `storage_ref` is an IPFS CID or
    /// encrypted object-store URL pointing to the actual document. GPS
    /// coordinates are stored as fixed-point integers (value × 10⁷).
    ///
    /// Emits event `("title", "registered")` with the title id as data.
    pub fn register_title(
        env: Env,
        registrar: Address,
        id: BytesN<32>,
        doc_hash: BytesN<32>,
        storage_ref: String,
        gps_lat: i64,
        gps_lng: i64,
        owner: Address,
    ) -> Result<TitleRecord, ContractError> {
        // Contract must already be initialized
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(ContractError::NotInitialized);
        }

        // Caller must be a registered registrar
        if !is_registrar(&env, &registrar) {
            return Err(ContractError::NotAuthorized);
        }
        registrar.require_auth();

        // Reject duplicate title ids
        if env
            .storage()
            .persistent()
            .has(&DataKey::TitleRecord(id.clone()))
        {
            return Err(ContractError::TitleAlreadyExists);
        }

        let now = env.ledger().timestamp();
        let record = TitleRecord {
            id: id.clone(),
            doc_hash,
            storage_ref,
            gps_lat,
            gps_lng,
            owner,
            status: TitleStatus::Active,
            created_at: now,
            updated_at: now,
        };

        env.storage()
            .persistent()
            .set(&DataKey::TitleRecord(id.clone()), &record);

        // Every state transition emits a Soroban event so the Day-6 indexer
        // can replay the full history without trusting any off-chain cache.
        env.events().publish(
            (symbol_short!("title"), symbol_short!("register")),
            id,
        );

        Ok(record)
    }
}
