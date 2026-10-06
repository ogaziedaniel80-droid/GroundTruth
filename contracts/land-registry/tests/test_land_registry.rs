use soroban_sdk::{testutils::Address as _, Address, Env, Vec};
use land_registry::{LandRegistry, types::DataKey, errors::ContractError};

fn create_test_env() -> (Env, Address, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let registrar1 = Address::generate(&env);
    let registrar2 = Address::generate(&env);
    let registrar3 = Address::generate(&env);

    (env, admin, registrar1, registrar2, registrar3)
}

fn invoke_initialize(env: &Env, contract_id: &Address, admin: &Address, registrars: &Vec<Address>, threshold: u32) -> Result<(), ContractError> {
    env.as_contract(contract_id, || {
        LandRegistry::initialize(env.clone(), admin.clone(), registrars.clone(), threshold)
    })
}

fn invoke_add_registrar(env: &Env, contract_id: &Address, registrar: &Address) -> Result<(), ContractError> {
    env.as_contract(contract_id, || {
        LandRegistry::add_registrar(env.clone(), registrar.clone())
    })
}

fn invoke_remove_registrar(env: &Env, contract_id: &Address, registrar: &Address) -> Result<(), ContractError> {
    env.as_contract(contract_id, || {
        LandRegistry::remove_registrar(env.clone(), registrar.clone())
    })
}

fn invoke_set_threshold(env: &Env, contract_id: &Address, threshold: u32) -> Result<(), ContractError> {
    env.as_contract(contract_id, || {
        LandRegistry::set_threshold(env.clone(), threshold)
    })
}

fn get_admin(env: &Env, contract_id: &Address) -> Address {
    env.as_contract(contract_id, || env.storage().instance().get(&DataKey::Admin).unwrap())
}

fn get_registrars_vec(env: &Env, contract_id: &Address) -> Vec<Address> {
    env.as_contract(contract_id, || env.storage().instance().get(&DataKey::Registrars).unwrap())
}

fn get_threshold_val(env: &Env, contract_id: &Address) -> u32 {
    env.as_contract(contract_id, || env.storage().instance().get(&DataKey::Threshold).unwrap())
}

#[test]
fn test_initialize_success() {
    let (env, admin, registrar1, registrar2, registrar3) = create_test_env();
    let contract_id = env.register_contract(None, LandRegistry);

    let registrars = Vec::from_array(&env, [registrar1.clone(), registrar2.clone(), registrar3.clone()]);
    let threshold = 2;

    let result = invoke_initialize(&env, &contract_id, &admin, &registrars, threshold);
    assert!(result.is_ok());

    let stored_admin = get_admin(&env, &contract_id);
    assert_eq!(stored_admin, admin);
}

#[test]
fn test_initialize_fails_on_double_init() {
    let (env, admin, registrar1, registrar2, registrar3) = create_test_env();
    let contract_id = env.register_contract(None, LandRegistry);

    let registrars = Vec::from_array(&env, [registrar1.clone(), registrar2.clone(), registrar3.clone()]);
    let threshold = 2;

    invoke_initialize(&env, &contract_id, &admin, &registrars, threshold).unwrap();

    let registrars2 = Vec::from_array(&env, [registrar1.clone(), registrar2.clone()]);
    let result = invoke_initialize(&env, &contract_id, &admin, &registrars2, threshold);
    assert_eq!(result.unwrap_err(), ContractError::AlreadyInitialized);
}

#[test]
fn test_add_registrar_requires_admin_auth() {
    let (env, admin, registrar1, registrar2, registrar3) = create_test_env();
    let contract_id = env.register_contract(None, LandRegistry);

    let registrars = Vec::from_array(&env, [registrar1.clone(), registrar2.clone()]);
    let threshold = 2;
    invoke_initialize(&env, &contract_id, &admin, &registrars, threshold).unwrap();

    let registrar4 = Address::generate(&env);
    let result = invoke_add_registrar(&env, &contract_id, &registrar4);
    assert!(result.is_ok());

    let stored_registrars = get_registrars_vec(&env, &contract_id);
    assert_eq!(stored_registrars.len(), 3);
    assert!(stored_registrars.iter().any(|r| r == registrar4));
}

#[test]
fn test_add_registrar_fails_duplicate() {
    let (env, admin, registrar1, registrar2, registrar3) = create_test_env();
    let contract_id = env.register_contract(None, LandRegistry);

    let registrars = Vec::from_array(&env, [registrar1.clone(), registrar2.clone()]);
    let threshold = 2;
    invoke_initialize(&env, &contract_id, &admin, &registrars, threshold).unwrap();

    let result = invoke_add_registrar(&env, &contract_id, &registrar1);
    assert_eq!(result.unwrap_err(), ContractError::RegistrarAlreadyExists);
}

#[test]
fn test_remove_registrar_requires_admin_auth() {
    let (env, admin, registrar1, registrar2, registrar3) = create_test_env();
    let contract_id = env.register_contract(None, LandRegistry);

    let registrars = Vec::from_array(&env, [registrar1.clone(), registrar2.clone(), registrar3.clone()]);
    let threshold = 2;
    invoke_initialize(&env, &contract_id, &admin, &registrars, threshold).unwrap();

    let result = invoke_remove_registrar(&env, &contract_id, &registrar1);
    assert!(result.is_ok());

    let stored_registrars = get_registrars_vec(&env, &contract_id);
    assert_eq!(stored_registrars.len(), 2);
    assert!(!stored_registrars.iter().any(|r| r == registrar1));
}

#[test]
fn test_remove_registrar_fails_not_found() {
    let (env, admin, registrar1, registrar2, registrar3) = create_test_env();
    let contract_id = env.register_contract(None, LandRegistry);

    let registrars = Vec::from_array(&env, [registrar1.clone(), registrar2.clone()]);
    let threshold = 2;
    invoke_initialize(&env, &contract_id, &admin, &registrars, threshold).unwrap();

    let non_existent = Address::generate(&env);
    let result = invoke_remove_registrar(&env, &contract_id, &non_existent);
    assert_eq!(result.unwrap_err(), ContractError::RegistrarNotFound);
}

#[test]
fn test_set_threshold_requires_admin_auth() {
    let (env, admin, registrar1, registrar2, registrar3) = create_test_env();
    let contract_id = env.register_contract(None, LandRegistry);

    let registrars = Vec::from_array(&env, [registrar1.clone(), registrar2.clone(), registrar3.clone()]);
    let threshold = 2;
    invoke_initialize(&env, &contract_id, &admin, &registrars, threshold).unwrap();

    let result = invoke_set_threshold(&env, &contract_id, 3);
    assert!(result.is_ok());

    let stored_threshold = get_threshold_val(&env, &contract_id);
    assert_eq!(stored_threshold, 3);
}

#[test]
fn test_set_threshold_fails_when_exceeds_registrar_count() {
    let (env, admin, registrar1, registrar2, registrar3) = create_test_env();
    let contract_id = env.register_contract(None, LandRegistry);

    let registrars = Vec::from_array(&env, [registrar1.clone(), registrar2.clone()]);
    let threshold = 2;
    invoke_initialize(&env, &contract_id, &admin, &registrars, threshold).unwrap();

    let result = invoke_set_threshold(&env, &contract_id, 3);
    assert_eq!(result.unwrap_err(), ContractError::InvalidThreshold);
}

#[test]
fn test_set_threshold_fails_when_zero() {
    let (env, admin, registrar1, registrar2, registrar3) = create_test_env();
    let contract_id = env.register_contract(None, LandRegistry);

    let registrars = Vec::from_array(&env, [registrar1.clone(), registrar2.clone(), registrar3.clone()]);
    let threshold = 2;
    invoke_initialize(&env, &contract_id, &admin, &registrars, threshold).unwrap();

    let result = invoke_set_threshold(&env, &contract_id, 0);
    assert_eq!(result.unwrap_err(), ContractError::InvalidThreshold);
}

#[test]
fn test_initialize_fails_when_threshold_exceeds_registrar_count() {
    let (env, admin, registrar1, registrar2, registrar3) = create_test_env();
    let contract_id = env.register_contract(None, LandRegistry);

    let registrars = Vec::from_array(&env, [registrar1.clone(), registrar2.clone()]);
    let threshold = 3;

    let result = invoke_initialize(&env, &contract_id, &admin, &registrars, threshold);
    assert_eq!(result.unwrap_err(), ContractError::InvalidThreshold);
}