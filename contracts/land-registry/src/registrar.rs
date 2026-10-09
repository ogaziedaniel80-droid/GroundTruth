use soroban_sdk::{Address, Env, Vec};

use crate::types::DataKey;
use crate::errors::ContractError;

pub fn get_registrars(env: &Env) -> Vec<Address> {
    env.storage().instance().get(&DataKey::Registrars).unwrap_or(Vec::new(env))
}

pub fn set_registrars(env: &Env, registrars: Vec<Address>) {
    env.storage().instance().set(&DataKey::Registrars, &registrars);
}

pub fn get_threshold(env: &Env) -> u32 {
    env.storage().instance().get(&DataKey::Threshold).unwrap_or(0)
}

pub fn set_threshold(env: &Env, threshold: u32) {
    env.storage().instance().set(&DataKey::Threshold, &threshold);
}

pub fn is_registrar(env: &Env, address: &Address) -> bool {
    let registrars = get_registrars(env);
    registrars.iter().any(|r| r == *address)
}

pub fn add_registrar_internal(env: &Env, registrar: Address) -> Result<(), ContractError> {
    let mut registrars = get_registrars(env);
    if registrars.iter().any(|r| r == registrar) {
        return Err(ContractError::RegistrarAlreadyExists);
    }
    registrars.push_back(registrar);
    set_registrars(env, registrars);
    Ok(())
}

pub fn remove_registrar_internal(env: &Env, registrar: &Address) -> Result<(), ContractError> {
    let mut registrars = get_registrars(env);
    let index = registrars.iter().position(|r| r == *registrar);
    match index {
        Some(i) => {
            registrars.remove(i as u32);
            set_registrars(env, registrars);
            Ok(())
        }
        None => Err(ContractError::RegistrarNotFound),
    }
}

pub fn validate_threshold(env: &Env, threshold: u32) -> Result<(), ContractError> {
    let registrars = get_registrars(env);
    if threshold == 0 || threshold > registrars.len() {
        return Err(ContractError::InvalidThreshold);
    }
    Ok(())
}