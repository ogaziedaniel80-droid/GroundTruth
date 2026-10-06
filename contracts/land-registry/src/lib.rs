pub mod types;
pub mod errors;
pub mod registrar;

use soroban_sdk::{Address, Env, Vec, contract, contractimpl};

use crate::types::DataKey;
use crate::errors::ContractError;
use crate::registrar::{add_registrar_internal, remove_registrar_internal, validate_threshold, set_threshold};

#[contract]
pub struct LandRegistry;

#[contractimpl]
impl LandRegistry {
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
        let admin: Address = env.storage().instance().get(&DataKey::Admin).ok_or(ContractError::NotInitialized)?;
        admin.require_auth();

        add_registrar_internal(&env, registrar)
    }

    pub fn remove_registrar(env: Env, registrar: Address) -> Result<(), ContractError> {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).ok_or(ContractError::NotInitialized)?;
        admin.require_auth();

        remove_registrar_internal(&env, &registrar)
    }

    pub fn set_threshold(env: Env, threshold: u32) -> Result<(), ContractError> {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).ok_or(ContractError::NotInitialized)?;
        admin.require_auth();

        validate_threshold(&env, threshold)?;
        set_threshold(&env, threshold);

        Ok(())
    }
}