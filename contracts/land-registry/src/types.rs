use soroban_sdk::{Address, BytesN, String, Vec, contracttype};

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