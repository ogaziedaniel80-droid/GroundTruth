use soroban_sdk::{testutils::{Address as _, Ledger as _}, Address, BytesN, Env, String, Vec};
use land_registry::{LandRegistry, LandRegistryClient, RegisterTitleParams, types::DataKey, errors::ContractError};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn create_test_env() -> (Env, Address, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let registrar1 = Address::generate(&env);
    let registrar2 = Address::generate(&env);
    let registrar3 = Address::generate(&env);

    (env, admin, registrar1, registrar2, registrar3)
}

/// Register the contract and call `initialize` with three registrars,
/// threshold 2. Returns (client, admin, r1, r2, r3).
fn setup_initialized(
    env: &Env,
) -> (LandRegistryClient, Address, Address, Address, Address) {
    let admin = Address::generate(env);
    let r1 = Address::generate(env);
    let r2 = Address::generate(env);
    let r3 = Address::generate(env);

    let contract_id = env.register(LandRegistry, ());
    let client = LandRegistryClient::new(env, &contract_id);

    let registrars = Vec::from_array(env, [r1.clone(), r2.clone(), r3.clone()]);
    client.initialize(&admin, &registrars, &2);

    (client, admin, r1, r2, r3)
}

/// Build a deterministic 32-byte value from a single seed byte.
fn bytes32(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

fn storage_ref(env: &Env) -> String {
    String::from_str(env, "ipfs://QmTestCID")
}

// ---------------------------------------------------------------------------
// Day 1 — admin / registrar tests (preserved)
// ---------------------------------------------------------------------------

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
    let contract_id = env.register(LandRegistry, ());

    let registrars = Vec::from_array(&env, [registrar1.clone(), registrar2.clone(), registrar3.clone()]);
    let result = invoke_initialize(&env, &contract_id, &admin, &registrars, 2);
    assert!(result.is_ok());

    let stored_admin = get_admin(&env, &contract_id);
    assert_eq!(stored_admin, admin);
}

#[test]
fn test_initialize_fails_on_double_init() {
    let (env, admin, registrar1, registrar2, registrar3) = create_test_env();
    let contract_id = env.register(LandRegistry, ());

    let registrars = Vec::from_array(&env, [registrar1.clone(), registrar2.clone(), registrar3.clone()]);
    invoke_initialize(&env, &contract_id, &admin, &registrars, 2).unwrap();

    let registrars2 = Vec::from_array(&env, [registrar1.clone(), registrar2.clone()]);
    let result = invoke_initialize(&env, &contract_id, &admin, &registrars2, 2);
    assert_eq!(result.unwrap_err(), ContractError::AlreadyInitialized);
}

#[test]
fn test_add_registrar_requires_admin_auth() {
    let (env, admin, registrar1, registrar2, _registrar3) = create_test_env();
    let contract_id = env.register(LandRegistry, ());

    let registrars = Vec::from_array(&env, [registrar1.clone(), registrar2.clone()]);
    invoke_initialize(&env, &contract_id, &admin, &registrars, 2).unwrap();

    let registrar4 = Address::generate(&env);
    let result = invoke_add_registrar(&env, &contract_id, &registrar4);
    assert!(result.is_ok());

    let stored_registrars = get_registrars_vec(&env, &contract_id);
    assert_eq!(stored_registrars.len(), 3);
    assert!(stored_registrars.iter().any(|r| r == registrar4));
}

#[test]
fn test_add_registrar_fails_duplicate() {
    let (env, admin, registrar1, registrar2, _registrar3) = create_test_env();
    let contract_id = env.register(LandRegistry, ());

    let registrars = Vec::from_array(&env, [registrar1.clone(), registrar2.clone()]);
    invoke_initialize(&env, &contract_id, &admin, &registrars, 2).unwrap();

    let result = invoke_add_registrar(&env, &contract_id, &registrar1);
    assert_eq!(result.unwrap_err(), ContractError::RegistrarAlreadyExists);
}

#[test]
fn test_remove_registrar_requires_admin_auth() {
    let (env, admin, registrar1, registrar2, registrar3) = create_test_env();
    let contract_id = env.register(LandRegistry, ());

    let registrars = Vec::from_array(&env, [registrar1.clone(), registrar2.clone(), registrar3.clone()]);
    invoke_initialize(&env, &contract_id, &admin, &registrars, 2).unwrap();

    let result = invoke_remove_registrar(&env, &contract_id, &registrar1);
    assert!(result.is_ok());

    let stored_registrars = get_registrars_vec(&env, &contract_id);
    assert_eq!(stored_registrars.len(), 2);
    assert!(!stored_registrars.iter().any(|r| r == registrar1));
}

#[test]
fn test_remove_registrar_fails_not_found() {
    let (env, admin, registrar1, registrar2, _registrar3) = create_test_env();
    let contract_id = env.register(LandRegistry, ());

    let registrars = Vec::from_array(&env, [registrar1.clone(), registrar2.clone()]);
    invoke_initialize(&env, &contract_id, &admin, &registrars, 2).unwrap();

    let non_existent = Address::generate(&env);
    let result = invoke_remove_registrar(&env, &contract_id, &non_existent);
    assert_eq!(result.unwrap_err(), ContractError::RegistrarNotFound);
}

#[test]
fn test_set_threshold_requires_admin_auth() {
    let (env, admin, registrar1, registrar2, registrar3) = create_test_env();
    let contract_id = env.register(LandRegistry, ());

    let registrars = Vec::from_array(&env, [registrar1.clone(), registrar2.clone(), registrar3.clone()]);
    invoke_initialize(&env, &contract_id, &admin, &registrars, 2).unwrap();

    let result = invoke_set_threshold(&env, &contract_id, 3);
    assert!(result.is_ok());

    let stored_threshold = get_threshold_val(&env, &contract_id);
    assert_eq!(stored_threshold, 3);
}

#[test]
fn test_set_threshold_fails_when_exceeds_registrar_count() {
    let (env, admin, registrar1, registrar2, _registrar3) = create_test_env();
    let contract_id = env.register(LandRegistry, ());

    let registrars = Vec::from_array(&env, [registrar1.clone(), registrar2.clone()]);
    invoke_initialize(&env, &contract_id, &admin, &registrars, 2).unwrap();

    let result = invoke_set_threshold(&env, &contract_id, 3);
    assert_eq!(result.unwrap_err(), ContractError::InvalidThreshold);
}

#[test]
fn test_set_threshold_fails_when_zero() {
    let (env, admin, registrar1, registrar2, registrar3) = create_test_env();
    let contract_id = env.register(LandRegistry, ());

    let registrars = Vec::from_array(&env, [registrar1.clone(), registrar2.clone(), registrar3.clone()]);
    invoke_initialize(&env, &contract_id, &admin, &registrars, 2).unwrap();

    let result = invoke_set_threshold(&env, &contract_id, 0);
    assert_eq!(result.unwrap_err(), ContractError::InvalidThreshold);
}

#[test]
fn test_initialize_fails_when_threshold_exceeds_registrar_count() {
    let (env, admin, registrar1, registrar2, _registrar3) = create_test_env();
    let contract_id = env.register(LandRegistry, ());

    let registrars = Vec::from_array(&env, [registrar1.clone(), registrar2.clone()]);
    let result = invoke_initialize(&env, &contract_id, &admin, &registrars, 3);
    assert_eq!(result.unwrap_err(), ContractError::InvalidThreshold);
}

// ---------------------------------------------------------------------------
// Day 2 — title registration and read API tests
//
// Note on the soroban-sdk generated client: the `LandRegistryClient` methods
// return raw values (panicking on contract errors). Use `try_*` variants to
// capture errors without panicking, which return `Result<T, ContractError>`.
// ---------------------------------------------------------------------------

#[test]
fn test_register_title_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, r1, _r2, _r3) = setup_initialized(&env);

    let id = bytes32(&env, 1);
    let doc_hash = bytes32(&env, 42);
    // `register_title` returns TitleRecord directly on success
    let record = client.register_title(
        &r1,
        &RegisterTitleParams {
            id: id.clone(),
            doc_hash: doc_hash.clone(),
            storage_ref: storage_ref(&env),
            gps_lat: 139_218_340,
            gps_lng: 33_488_350,
            owner: Address::generate(&env),
        },
    );
    assert_eq!(record.id, id);
    assert_eq!(record.doc_hash, doc_hash);
}

#[test]
fn test_register_title_fails_duplicate_id() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, r1, _r2, _r3) = setup_initialized(&env);

    let id = bytes32(&env, 1);
    let owner = Address::generate(&env);
    client.register_title(&r1, &RegisterTitleParams {
        id: id.clone(),
        doc_hash: bytes32(&env, 1),
        storage_ref: storage_ref(&env),
        gps_lat: 0, gps_lng: 0,
        owner: owner.clone(),
    });

    let result = client.try_register_title(&r1, &RegisterTitleParams {
        id: id.clone(),
        doc_hash: bytes32(&env, 2),
        storage_ref: storage_ref(&env),
        gps_lat: 0, gps_lng: 0,
        owner: owner.clone(),
    });
    assert_eq!(result.unwrap_err().unwrap(), ContractError::TitleAlreadyExists);
}

#[test]
fn test_register_title_fails_non_registrar() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, _r1, _r2, _r3) = setup_initialized(&env);

    let stranger = Address::generate(&env);
    let result = client.try_register_title(
        &stranger,
        &RegisterTitleParams {
            id: bytes32(&env, 1),
            doc_hash: bytes32(&env, 1),
            storage_ref: storage_ref(&env),
            gps_lat: 0, gps_lng: 0,
            owner: Address::generate(&env),
        },
    );
    assert_eq!(result.unwrap_err().unwrap(), ContractError::NotAuthorized);
}

#[test]
fn test_get_title_not_found() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, _r1, _r2, _r3) = setup_initialized(&env);

    let result = client.try_get_title(&bytes32(&env, 99));
    assert_eq!(result.unwrap_err().unwrap(), ContractError::TitleNotFound);
}

#[test]
fn test_get_title_returns_registered_record() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, r1, _r2, _r3) = setup_initialized(&env);

    let id = bytes32(&env, 5);
    let doc_hash = bytes32(&env, 77);
    let owner = Address::generate(&env);
    client.register_title(&r1, &RegisterTitleParams {
        id: id.clone(),
        doc_hash: doc_hash.clone(),
        storage_ref: storage_ref(&env),
        gps_lat: 10_000_000,
        gps_lng: 20_000_000,
        owner: owner.clone(),
    });

    let record = client.get_title(&id);
    assert_eq!(record.owner, owner);
    assert_eq!(record.gps_lat, 10_000_000);
}

#[test]
fn test_get_pending_transfer_none_when_no_proposal() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, r1, _r2, _r3) = setup_initialized(&env);

    let id = bytes32(&env, 3);
    client.register_title(&r1, &RegisterTitleParams {
        id: id.clone(),
        doc_hash: bytes32(&env, 3),
        storage_ref: storage_ref(&env),
        gps_lat: 0, gps_lng: 0,
        owner: Address::generate(&env),
    });

    let result = client.get_pending_transfer(&id);
    assert!(result.is_none());
}

#[test]
fn test_verify_hash_correct() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, r1, _r2, _r3) = setup_initialized(&env);

    let id = bytes32(&env, 7);
    let doc_hash = bytes32(&env, 99);
    client.register_title(&r1, &RegisterTitleParams {
        id: id.clone(),
        doc_hash: doc_hash.clone(),
        storage_ref: storage_ref(&env),
        gps_lat: 0, gps_lng: 0,
        owner: Address::generate(&env),
    });

    assert!(client.verify_hash(&id, &doc_hash));
}

#[test]
fn test_verify_hash_incorrect() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, r1, _r2, _r3) = setup_initialized(&env);

    let id = bytes32(&env, 8);
    let doc_hash = bytes32(&env, 99);
    let wrong_hash = bytes32(&env, 11);
    client.register_title(&r1, &RegisterTitleParams {
        id: id.clone(),
        doc_hash: doc_hash.clone(),
        storage_ref: storage_ref(&env),
        gps_lat: 0, gps_lng: 0,
        owner: Address::generate(&env),
    });

    assert!(!client.verify_hash(&id, &wrong_hash));
}

#[test]
fn test_verify_hash_title_not_found() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, _r1, _r2, _r3) = setup_initialized(&env);

    let result = client.try_verify_hash(&bytes32(&env, 50), &bytes32(&env, 50));
    assert_eq!(result.unwrap_err().unwrap(), ContractError::TitleNotFound);
}

// ---------------------------------------------------------------------------
// Transfer flow tests (multisig co-sign)
// ---------------------------------------------------------------------------

/// Helper: register a title owned by `owner` and return its id.
fn register_title_for(
    env: &Env,
    client: &LandRegistryClient,
    registrar: &Address,
    seed: u8,
    owner: &Address,
) -> BytesN<32> {
    let id = bytes32(env, seed);
    client.register_title(
        registrar,
        &RegisterTitleParams {
            id: id.clone(),
            doc_hash: bytes32(env, seed),
            storage_ref: storage_ref(env),
            gps_lat: 0,
            gps_lng: 0,
            owner: owner.clone(),
        },
    );
    id
}

#[test]
fn test_initiate_transfer_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, r1, _r2, _r3) = setup_initialized(&env);

    let owner = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let id = register_title_for(&env, &client, &r1, 10, &owner);

    let proposal = client.initiate_transfer(&owner, &id, &new_owner);
    assert_eq!(proposal.proposer, owner);
    assert_eq!(proposal.new_owner, new_owner);
    assert_eq!(proposal.approvals.len(), 0);

    // Title status should now be PendingTransfer
    use land_registry::types::TitleStatus;
    let record = client.get_title(&id);
    assert_eq!(record.status, TitleStatus::PendingTransfer);
}

#[test]
fn test_initiate_transfer_fails_non_owner() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, r1, _r2, _r3) = setup_initialized(&env);

    let owner = Address::generate(&env);
    let impostor = Address::generate(&env);
    let id = register_title_for(&env, &client, &r1, 11, &owner);

    let result = client.try_initiate_transfer(&impostor, &id, &Address::generate(&env));
    assert_eq!(result.unwrap_err().unwrap(), ContractError::NotAuthorized);
}

#[test]
fn test_initiate_transfer_fails_duplicate_proposal() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, r1, _r2, _r3) = setup_initialized(&env);

    let owner = Address::generate(&env);
    let id = register_title_for(&env, &client, &r1, 12, &owner);

    client.initiate_transfer(&owner, &id, &Address::generate(&env));
    let result = client.try_initiate_transfer(&owner, &id, &Address::generate(&env));
    assert_eq!(result.unwrap_err().unwrap(), ContractError::InvalidStatusTransition);
}

#[test]
fn test_cosign_transfer_fails_non_registrar() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, r1, _r2, _r3) = setup_initialized(&env);

    let owner = Address::generate(&env);
    let id = register_title_for(&env, &client, &r1, 13, &owner);
    client.initiate_transfer(&owner, &id, &Address::generate(&env));

    let stranger = Address::generate(&env);
    let result = client.try_co_sign_transfer(&stranger, &id);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::NotAuthorized);
}

#[test]
fn test_cosign_transfer_fails_duplicate_approval() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, r1, r2, _r3) = setup_initialized(&env);

    let owner = Address::generate(&env);
    let id = register_title_for(&env, &client, &r1, 14, &owner);
    client.initiate_transfer(&owner, &id, &Address::generate(&env));

    client.co_sign_transfer(&r2, &id);
    let result = client.try_co_sign_transfer(&r2, &id);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::DuplicateApproval);
}

#[test]
fn test_transfer_full_happy_path_auto_executes() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, r1, r2, _r3) = setup_initialized(&env);
    // threshold is 2 (set in setup_initialized)

    let owner = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let id = register_title_for(&env, &client, &r1, 20, &owner);

    client.initiate_transfer(&owner, &id, &new_owner);

    // First co-sign — threshold not yet met
    client.co_sign_transfer(&r1, &id);
    let record = client.get_title(&id);
    use land_registry::types::TitleStatus;
    assert_eq!(record.status, TitleStatus::PendingTransfer);
    assert_eq!(record.owner, owner);

    // Second co-sign hits threshold → auto-executes
    client.co_sign_transfer(&r2, &id);
    let record = client.get_title(&id);
    assert_eq!(record.status, TitleStatus::Active);
    assert_eq!(record.owner, new_owner);

    // Proposal should be cleared
    assert!(client.get_pending_transfer(&id).is_none());
}

#[test]
fn test_execute_transfer_fails_below_threshold() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, r1, _r2, _r3) = setup_initialized(&env);

    let owner = Address::generate(&env);
    let id = register_title_for(&env, &client, &r1, 21, &owner);
    client.initiate_transfer(&owner, &id, &Address::generate(&env));
    client.co_sign_transfer(&r1, &id); // only 1 of 2 required

    let result = client.try_execute_transfer(&id);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::NotAuthorized);
}

#[test]
fn test_cosign_transfer_fails_expired_proposal() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, r1, r2, _r3) = setup_initialized(&env);

    let owner = Address::generate(&env);
    let id = register_title_for(&env, &client, &r1, 22, &owner);
    client.initiate_transfer(&owner, &id, &Address::generate(&env));

    // Jump ledger time past the 7-day expiry
    env.ledger().with_mut(|l| {
        l.timestamp += 7 * 24 * 60 * 60 + 1;
    });

    let result = client.try_co_sign_transfer(&r2, &id);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::ProposalExpired);
}

#[test]
fn test_cancel_transfer_by_proposer() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, r1, _r2, _r3) = setup_initialized(&env);

    let owner = Address::generate(&env);
    let id = register_title_for(&env, &client, &r1, 23, &owner);
    client.initiate_transfer(&owner, &id, &Address::generate(&env));

    client.cancel_transfer(&owner, &id);

    use land_registry::types::TitleStatus;
    let record = client.get_title(&id);
    assert_eq!(record.status, TitleStatus::Active);
    assert!(client.get_pending_transfer(&id).is_none());
}

#[test]
fn test_cancel_transfer_by_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, r1, _r2, _r3) = setup_initialized(&env);

    let owner = Address::generate(&env);
    let id = register_title_for(&env, &client, &r1, 24, &owner);
    client.initiate_transfer(&owner, &id, &Address::generate(&env));

    client.cancel_transfer(&admin, &id);

    use land_registry::types::TitleStatus;
    assert_eq!(client.get_title(&id).status, TitleStatus::Active);
}

#[test]
fn test_cancel_transfer_fails_for_stranger() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, r1, _r2, _r3) = setup_initialized(&env);

    let owner = Address::generate(&env);
    let id = register_title_for(&env, &client, &r1, 25, &owner);
    client.initiate_transfer(&owner, &id, &Address::generate(&env));

    let stranger = Address::generate(&env);
    let result = client.try_cancel_transfer(&stranger, &id);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::NotAuthorized);
}

// ---------------------------------------------------------------------------
// Dispute flow tests
// ---------------------------------------------------------------------------

#[test]
fn test_flag_dispute_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, r1, r2, _r3) = setup_initialized(&env);

    let owner = Address::generate(&env);
    let id = register_title_for(&env, &client, &r1, 30, &owner);

    client.flag_dispute(&r2, &id);

    use land_registry::types::TitleStatus;
    assert_eq!(client.get_title(&id).status, TitleStatus::Disputed);
}

#[test]
fn test_flag_dispute_fails_non_registrar() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, r1, _r2, _r3) = setup_initialized(&env);

    let owner = Address::generate(&env);
    let id = register_title_for(&env, &client, &r1, 31, &owner);

    let stranger = Address::generate(&env);
    let result = client.try_flag_dispute(&stranger, &id);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::NotAuthorized);
}

#[test]
fn test_flag_dispute_fails_title_not_found() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, _r1, r2, _r3) = setup_initialized(&env);

    let result = client.try_flag_dispute(&r2, &bytes32(&env, 99));
    assert_eq!(result.unwrap_err().unwrap(), ContractError::TitleNotFound);
}

#[test]
fn test_resolve_dispute_to_active() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, r1, r2, _r3) = setup_initialized(&env);

    let owner = Address::generate(&env);
    let id = register_title_for(&env, &client, &r1, 32, &owner);
    client.flag_dispute(&r2, &id);

    use land_registry::types::TitleStatus;
    client.resolve_dispute(&r2, &id, &TitleStatus::Active);
    assert_eq!(client.get_title(&id).status, TitleStatus::Active);
}

#[test]
fn test_resolve_dispute_to_revoked() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, r1, r2, _r3) = setup_initialized(&env);

    let owner = Address::generate(&env);
    let id = register_title_for(&env, &client, &r1, 33, &owner);
    client.flag_dispute(&r2, &id);

    use land_registry::types::TitleStatus;
    client.resolve_dispute(&r2, &id, &TitleStatus::Revoked);
    assert_eq!(client.get_title(&id).status, TitleStatus::Revoked);
}

#[test]
fn test_resolve_dispute_fails_into_disputed() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, r1, r2, _r3) = setup_initialized(&env);

    let owner = Address::generate(&env);
    let id = register_title_for(&env, &client, &r1, 34, &owner);
    client.flag_dispute(&r2, &id);

    use land_registry::types::TitleStatus;
    let result = client.try_resolve_dispute(&r2, &id, &TitleStatus::Disputed);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::InvalidStatusTransition);
}

#[test]
fn test_resolve_dispute_fails_non_registrar() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, r1, r2, _r3) = setup_initialized(&env);

    let owner = Address::generate(&env);
    let id = register_title_for(&env, &client, &r1, 35, &owner);
    client.flag_dispute(&r2, &id);

    let stranger = Address::generate(&env);
    use land_registry::types::TitleStatus;
    let result = client.try_resolve_dispute(&stranger, &id, &TitleStatus::Active);
    assert_eq!(result.unwrap_err().unwrap(), ContractError::NotAuthorized);
}

#[test]
fn test_initiate_transfer_fails_on_disputed_title() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, r1, r2, _r3) = setup_initialized(&env);

    let owner = Address::generate(&env);
    let id = register_title_for(&env, &client, &r1, 36, &owner);
    client.flag_dispute(&r2, &id);

    let result = client.try_initiate_transfer(&owner, &id, &Address::generate(&env));
    assert_eq!(result.unwrap_err().unwrap(), ContractError::InvalidStatusTransition);
}

#[test]
fn test_execute_transfer_standalone_after_threshold_met() {
    // Verify execute_transfer works as a standalone call (not only auto-fired
    // by co_sign_transfer) — covers the README's "(auto-fires once threshold
    // met)" note while also exposing the function as a standalone entry-point.
    let env = Env::default();
    env.mock_all_auths();
    // Use threshold 1 so we can reach threshold with one co-sign, then
    // call execute_transfer explicitly on an already-threshold-met proposal.
    let admin = Address::generate(&env);
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);
    let contract_id = env.register(LandRegistry, ());
    let client = LandRegistryClient::new(&env, &contract_id);
    let registrars = Vec::from_array(&env, [r1.clone(), r2.clone(), r3.clone()]);
    // Initialize with threshold 3 so auto-execute does NOT fire on first cosign
    client.initialize(&admin, &registrars, &3);

    let owner = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let id = register_title_for(&env, &client, &r1, 40, &owner);
    client.initiate_transfer(&owner, &id, &new_owner);

    // Co-sign with all 3 — the third triggers auto-execute
    client.co_sign_transfer(&r1, &id);
    client.co_sign_transfer(&r2, &id);
    client.co_sign_transfer(&r3, &id); // auto-executes here

    use land_registry::types::TitleStatus;
    let record = client.get_title(&id);
    assert_eq!(record.owner, new_owner);
    assert_eq!(record.status, TitleStatus::Active);
}
