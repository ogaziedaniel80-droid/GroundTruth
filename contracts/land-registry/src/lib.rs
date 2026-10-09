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

use soroban_sdk::{Address, BytesN, Env, String, Vec, contract, contractimpl, contracttype};

use crate::types::{
    DataKey, DisputeFlagged, DisputeResolved, TitleRecord, TitleRegistered, TitleStatus,
    TransferCancelled, TransferCoSigned, TransferExecuted, TransferInitiated, TransferProposal,
};
use crate::errors::ContractError;
use crate::registrar::{
    add_registrar_internal, get_threshold, is_registrar, remove_registrar_internal,
    validate_threshold, set_threshold,
};

/// Input parameters for `register_title`. Bundled into a struct so the
/// function stays within clippy's 7-argument limit while keeping all fields
/// from the README's Data model.
#[contracttype]
pub struct RegisterTitleParams {
    pub id: BytesN<32>,
    pub doc_hash: BytesN<32>,
    pub storage_ref: String,
    pub gps_lat: i64,
    pub gps_lng: i64,
    pub owner: Address,
}

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

        if threshold == 0 || threshold > registrars.len() {
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
    /// Only a current registrar may anchor a title. `params.id` must be
    /// unique; a duplicate returns `TitleAlreadyExists`. `params.doc_hash` is
    /// the SHA-256 of the off-chain document. `params.storage_ref` is an IPFS
    /// CID or encrypted object-store URL. GPS coordinates are fixed-point
    /// integers (value × 10⁷).
    ///
    /// Emits `TitleRegistered`.
    pub fn register_title(
        env: Env,
        registrar: Address,
        params: RegisterTitleParams,
    ) -> Result<TitleRecord, ContractError> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(ContractError::NotInitialized);
        }

        if !is_registrar(&env, &registrar) {
            return Err(ContractError::NotAuthorized);
        }
        registrar.require_auth();

        if env
            .storage()
            .persistent()
            .has(&DataKey::TitleRecord(params.id.clone()))
        {
            return Err(ContractError::TitleAlreadyExists);
        }

        let now = env.ledger().timestamp();
        let record = TitleRecord {
            id: params.id.clone(),
            doc_hash: params.doc_hash,
            storage_ref: params.storage_ref,
            gps_lat: params.gps_lat,
            gps_lng: params.gps_lng,
            owner: params.owner.clone(),
            status: TitleStatus::Active,
            created_at: now,
            updated_at: now,
        };

        env.storage()
            .persistent()
            .set(&DataKey::TitleRecord(params.id.clone()), &record);

        env.events().publish_event(&TitleRegistered {
            title_id: params.id,
            owner: params.owner,
        });

        Ok(record)
    }

    /// Flag a title as disputed.
    ///
    /// A registrar may flag any title regardless of current status. Emits
    /// `DisputeFlagged`.
    pub fn flag_dispute(
        env: Env,
        registrar: Address,
        title_id: BytesN<32>,
    ) -> Result<(), ContractError> {
        if !is_registrar(&env, &registrar) {
            return Err(ContractError::NotAuthorized);
        }
        registrar.require_auth();

        let mut record: TitleRecord = env
            .storage()
            .persistent()
            .get(&DataKey::TitleRecord(title_id.clone()))
            .ok_or(ContractError::TitleNotFound)?;

        if record.status == TitleStatus::Disputed {
            return Ok(());
        }

        record.status = TitleStatus::Disputed;
        record.updated_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&DataKey::TitleRecord(title_id.clone()), &record);

        env.events().publish_event(&DisputeFlagged {
            title_id,
            registrar,
        });

        Ok(())
    }

    /// Resolve a disputed title by setting it to an explicit new status.
    ///
    /// Only `Active` and `Revoked` are valid target statuses — passing
    /// `Disputed` or `PendingTransfer` returns `InvalidStatusTransition`.
    /// Emits `DisputeResolved`.
    pub fn resolve_dispute(
        env: Env,
        registrar: Address,
        title_id: BytesN<32>,
        new_status: TitleStatus,
    ) -> Result<(), ContractError> {
        if !is_registrar(&env, &registrar) {
            return Err(ContractError::NotAuthorized);
        }
        registrar.require_auth();

        if new_status == TitleStatus::Disputed || new_status == TitleStatus::PendingTransfer {
            return Err(ContractError::InvalidStatusTransition);
        }

        let mut record: TitleRecord = env
            .storage()
            .persistent()
            .get(&DataKey::TitleRecord(title_id.clone()))
            .ok_or(ContractError::TitleNotFound)?;

        record.status = new_status;
        record.updated_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&DataKey::TitleRecord(title_id.clone()), &record);

        env.events().publish_event(&DisputeResolved {
            title_id,
            registrar,
        });

        Ok(())
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
    pub fn get_pending_transfer(
        env: Env,
        title_id: BytesN<32>,
    ) -> Option<TransferProposal> {
        env.storage()
            .persistent()
            .get(&DataKey::TransferProposal(title_id))
    }

    /// Return `true` iff `candidate_hash` matches the `doc_hash` stored for
    /// `title_id`. Returns `TitleNotFound` if the title does not exist.
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
    /// Gives registrars a practical window to co-sign without leaving titles
    /// in PendingTransfer state indefinitely. The proposer can cancel at any
    /// time before expiry.
    const TRANSFER_EXPIRY_SECS: u64 = 7 * 24 * 60 * 60;

    /// Initiate an ownership-transfer proposal.
    ///
    /// Only the current owner may propose. Title must be `Active`. Emits
    /// `TransferInitiated`.
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

        if record.owner != owner {
            return Err(ContractError::NotAuthorized);
        }

        if record.status != TitleStatus::Active {
            return Err(ContractError::InvalidStatusTransition);
        }

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
            new_owner: new_owner.clone(),
            proposer: owner.clone(),
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

        env.events().publish_event(&TransferInitiated {
            title_id,
            proposer: owner,
            new_owner,
        });

        Ok(proposal)
    }

    /// Co-sign a pending transfer proposal as a registrar.
    ///
    /// Idempotent per registrar (`DuplicateApproval` on repeat). Rejects
    /// expired proposals. Auto-executes once `approvals.len() >= threshold`.
    /// Emits `TransferCoSigned`, and `TransferExecuted` if threshold is met.
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

        if env.ledger().timestamp() > proposal.expires_at {
            return Err(ContractError::ProposalExpired);
        }

        if proposal.approvals.iter().any(|a| a == registrar) {
            return Err(ContractError::DuplicateApproval);
        }

        proposal.approvals.push_back(registrar.clone());
        let approvals_count = proposal.approvals.len();

        env.storage()
            .persistent()
            .set(&DataKey::TransferProposal(title_id.clone()), &proposal);

        env.events().publish_event(&TransferCoSigned {
            title_id: title_id.clone(),
            registrar,
            approvals_so_far: approvals_count,
        });

        if approvals_count >= proposal.threshold {
            Self::execute_transfer(env, title_id)?;
        }

        Ok(())
    }

    /// Finalise an approved transfer.
    ///
    /// Called automatically by `co_sign_transfer` when threshold is met, but
    /// also exposed as a standalone entry-point. Emits `TransferExecuted`.
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

        env.events().publish_event(&TransferExecuted {
            title_id,
            new_owner: proposal.new_owner,
        });

        Ok(())
    }

    /// Cancel a pending transfer proposal.
    ///
    /// Callable by the original proposer or the admin. Emits
    /// `TransferCancelled`.
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

        env.events().publish_event(&TransferCancelled {
            title_id,
            cancelled_by: caller,
        });

        Ok(())
    }
}
