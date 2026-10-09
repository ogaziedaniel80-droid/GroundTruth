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

use crate::types::{DataKey, TitleRecord, TitleStatus, TransferProposal};
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

    // -------------------------------------------------------------------------
    // Read-only
    // -------------------------------------------------------------------------

    /// Return the `TitleRecord` for `title_id`, or `TitleNotFound` if absent.
    pub fn get_title(env: Env, title_id: BytesN<32>) -> Result<TitleRecord, ContractError> {
        env.storage()
            .persistent()
            .get(&DataKey::TitleRecord(title_id))
            .ok_or(ContractError::TitleNotFound)
    }

    /// Return the pending `TransferProposal` for `title_id`, if one exists.
    /// Returns `None` when no proposal is in flight (normal state).
    pub fn get_pending_transfer(
        env: Env,
        title_id: BytesN<32>,
    ) -> Option<crate::types::TransferProposal> {
        env.storage()
            .persistent()
            .get(&DataKey::TransferProposal(title_id))
    }

    /// Return `true` iff `candidate_hash` matches the `doc_hash` stored for
    /// `title_id`. Returns `TitleNotFound` if the title does not exist.
    ///
    /// This is the primary on-chain verification primitive: a caller hashes the
    /// document they hold off-chain and passes it here; the contract confirms
    /// whether it matches what was anchored at registration time.
    pub fn verify_hash(
        env: Env,
        title_id: BytesN<32>,
        candidate_hash: BytesN<32>,
    ) -> Result<bool, ContractError> {
        let record: TitleRecord = env
            .storage()
            .persistent()
            .get(&DataKey::TitleRecord(title_id))
            .ok_or(ContractError::TitleNotFound)?;

        Ok(record.doc_hash == candidate_hash)
    }

    // -------------------------------------------------------------------------
    // Transfer flow (registrar M-of-N multisig)
    // -------------------------------------------------------------------------

    /// Proposal expiry: 7 days of ledger time (seconds).
    /// 7 days gives registrars a practical window to co-sign without leaving
    /// titles in PendingTransfer state indefinitely. The proposer can cancel
    /// at any time before expiry if circumstances change.
    const TRANSFER_EXPIRY_SECS: u64 = 7 * 24 * 60 * 60;

    /// Initiate an ownership-transfer proposal.
    ///
    /// Only the current owner may propose a transfer. The title must be
    /// `Active` — a title that is already `PendingTransfer`, `Disputed`, or
    /// `Revoked` cannot have a new proposal opened. The proposal is stored
    /// on-chain keyed by `title_id`; registrars then co-sign via
    /// `co_sign_transfer`. Once enough approvals are collected the transfer
    /// executes automatically inside `co_sign_transfer`.
    ///
    /// Emits event `("transfer", "initiate")`.
    pub fn initiate_transfer(
        env: Env,
        owner: Address,
        title_id: BytesN<32>,
        new_owner: Address,
    ) -> Result<TransferProposal, ContractError> {
        owner.require_auth();

        let mut record: TitleRecord = env
            .storage()
            .persistent()
            .get(&DataKey::TitleRecord(title_id.clone()))
            .ok_or(ContractError::TitleNotFound)?;

        // Only the current owner may initiate
        if record.owner != owner {
            return Err(ContractError::NotAuthorized);
        }

        // Title must be Active to accept a transfer proposal
        if record.status != TitleStatus::Active {
            return Err(ContractError::InvalidStatusTransition);
        }

        // Reject a second concurrent proposal on the same title
        if env
            .storage()
            .persistent()
            .has(&DataKey::TransferProposal(title_id.clone()))
        {
            return Err(ContractError::InvalidStatusTransition);
        }

        let threshold = get_threshold(&env);
        let expires_at = env.ledger().timestamp() + Self::TRANSFER_EXPIRY_SECS;

        let proposal = TransferProposal {
            title_id: title_id.clone(),
            new_owner,
            proposer: owner,
            approvals: Vec::new(&env),
            threshold,
            expires_at,
        };

        record.status = TitleStatus::PendingTransfer;
        record.updated_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&DataKey::TitleRecord(title_id.clone()), &record);
        env.storage()
            .persistent()
            .set(&DataKey::TransferProposal(title_id.clone()), &proposal);

        env.events().publish(
            (symbol_short!("transfer"), symbol_short!("initiate")),
            title_id,
        );

        Ok(proposal)
    }

    /// Co-sign a pending transfer proposal as a registrar.
    ///
    /// Idempotent per registrar: a registrar cannot double-count their own
    /// approval (`DuplicateApproval`). Rejects if the proposal has expired.
    /// Once `approvals.len() >= threshold` the transfer executes automatically,
    /// matching the README's note "(auto-fires once threshold met)."
    ///
    /// Emits event `("transfer", "cosign")` on each successful co-sign.
    pub fn co_sign_transfer(
        env: Env,
        registrar: Address,
        title_id: BytesN<32>,
    ) -> Result<(), ContractError> {
        if !is_registrar(&env, &registrar) {
            return Err(ContractError::NotAuthorized);
        }
        registrar.require_auth();

        let mut proposal: TransferProposal = env
            .storage()
            .persistent()
            .get(&DataKey::TransferProposal(title_id.clone()))
            .ok_or(ContractError::ProposalNotFound)?;

        // Reject expired proposals
        if env.ledger().timestamp() > proposal.expires_at {
            return Err(ContractError::ProposalExpired);
        }

        // Idempotency guard — each registrar counts once
        if proposal.approvals.iter().any(|a| a == registrar) {
            return Err(ContractError::DuplicateApproval);
        }

        proposal.approvals.push_back(registrar.clone());

        env.storage()
            .persistent()
            .set(&DataKey::TransferProposal(title_id.clone()), &proposal);

        env.events().publish(
            (symbol_short!("transfer"), symbol_short!("cosign")),
            title_id.clone(),
        );

        // Auto-execute once threshold is met
        if proposal.approvals.len() >= proposal.threshold {
            Self::execute_transfer(env, title_id)?;
        }

        Ok(())
    }

    /// Finalise an approved transfer, flipping ownership to `new_owner`.
    ///
    /// Called automatically by `co_sign_transfer` when the approval threshold
    /// is reached, but also exposed as a standalone entry-point so external
    /// callers (e.g. a backend cron) can trigger execution after the threshold
    /// has already been met (e.g. if a network issue interrupted `co_sign`).
    ///
    /// Emits event `("transfer", "execute")`.
    pub fn execute_transfer(
        env: Env,
        title_id: BytesN<32>,
    ) -> Result<(), ContractError> {
        let proposal: TransferProposal = env
            .storage()
            .persistent()
            .get(&DataKey::TransferProposal(title_id.clone()))
            .ok_or(ContractError::ProposalNotFound)?;

        if proposal.approvals.len() < proposal.threshold {
            return Err(ContractError::NotAuthorized);
        }

        let mut record: TitleRecord = env
            .storage()
            .persistent()
            .get(&DataKey::TitleRecord(title_id.clone()))
            .ok_or(ContractError::TitleNotFound)?;

        record.owner = proposal.new_owner.clone();
        record.status = TitleStatus::Active;
        record.updated_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&DataKey::TitleRecord(title_id.clone()), &record);
        env.storage()
            .persistent()
            .remove(&DataKey::TransferProposal(title_id.clone()));

        env.events().publish(
            (symbol_short!("transfer"), symbol_short!("execute")),
            title_id,
        );

        Ok(())
    }

    /// Cancel a pending transfer proposal.
    ///
    /// May be called by the original proposer (owner who initiated) or by the
    /// contract admin. Clears the proposal and reverts the title status to
    /// `Active`. Anyone else receives `NotAuthorized`.
    ///
    /// Emits event `("transfer", "cancel")`.
    pub fn cancel_transfer(
        env: Env,
        caller: Address,
        title_id: BytesN<32>,
    ) -> Result<(), ContractError> {
        caller.require_auth();

        let proposal: TransferProposal = env
            .storage()
            .persistent()
            .get(&DataKey::TransferProposal(title_id.clone()))
            .ok_or(ContractError::ProposalNotFound)?;

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(ContractError::NotInitialized)?;

        // Only the original proposer or the admin may cancel
        if caller != proposal.proposer && caller != admin {
            return Err(ContractError::NotAuthorized);
        }

        let mut record: TitleRecord = env
            .storage()
            .persistent()
            .get(&DataKey::TitleRecord(title_id.clone()))
            .ok_or(ContractError::TitleNotFound)?;

        record.status = TitleStatus::Active;
        record.updated_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&DataKey::TitleRecord(title_id.clone()), &record);
        env.storage()
            .persistent()
            .remove(&DataKey::TransferProposal(title_id.clone()));

        env.events().publish(
            (symbol_short!("transfer"), symbol_short!("cancel")),
            title_id,
        );

        Ok(())
    }
}
