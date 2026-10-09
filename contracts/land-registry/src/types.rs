use soroban_sdk::{Address, BytesN, String, Vec, contracttype, contractevent};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TitleStatus {
    Active,
    PendingTransfer,
    Disputed,
    Revoked,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TitleRecord {
    pub id: BytesN<32>,
    pub doc_hash: BytesN<32>,
    pub storage_ref: String,
    pub gps_lat: i64,
    pub gps_lng: i64,
    pub owner: Address,
    pub status: TitleStatus,
    pub created_at: u64,
    pub updated_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferProposal {
    pub title_id: BytesN<32>,
    pub new_owner: Address,
    pub proposer: Address,
    pub approvals: Vec<Address>,
    pub threshold: u32,
    pub expires_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    Registrars,
    Threshold,
    TitleRecord(BytesN<32>),
    TransferProposal(BytesN<32>),
}

// ---------------------------------------------------------------------------
// Typed contract events
// Every state transition emits one of the variants below so the Day-6 indexer
// can replay the full history from the chain without trusting any off-chain
// cache. See README.md "Development workflow" section.
// ---------------------------------------------------------------------------

#[contractevent]
pub struct TitleRegistered {
    pub title_id: BytesN<32>,
    pub owner: Address,
}

#[contractevent]
pub struct TransferInitiated {
    pub title_id: BytesN<32>,
    pub proposer: Address,
    pub new_owner: Address,
}

#[contractevent]
pub struct TransferCoSigned {
    pub title_id: BytesN<32>,
    pub registrar: Address,
    pub approvals_so_far: u32,
}

#[contractevent]
pub struct TransferExecuted {
    pub title_id: BytesN<32>,
    pub new_owner: Address,
}

#[contractevent]
pub struct TransferCancelled {
    pub title_id: BytesN<32>,
    pub cancelled_by: Address,
}

#[contractevent]
pub struct DisputeFlagged {
    pub title_id: BytesN<32>,
    pub registrar: Address,
}

#[contractevent]
pub struct DisputeResolved {
    pub title_id: BytesN<32>,
    pub registrar: Address,
}