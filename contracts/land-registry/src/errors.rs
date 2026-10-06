use soroban_sdk::{contracterror, Env};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    NotAuthorized = 1,
    AlreadyInitialized = 2,
    NotInitialized = 3,
    TitleNotFound = 4,
    TitleAlreadyExists = 5,
    InvalidThreshold = 6,
    RegistrarAlreadyExists = 7,
    RegistrarNotFound = 8,
    DuplicateApproval = 9,
    ProposalExpired = 10,
    ProposalNotFound = 11,
    InvalidStatusTransition = 12,
}

impl ContractError {
    pub fn check_authorized(_env: &Env, address: &soroban_sdk::Address) -> Result<(), ContractError> {
        address.require_auth();
        Ok(())
    }
}