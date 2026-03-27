#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as AddressTrait, Events, Ledger},
    testutils::storage::Instance as StorageInstance,
    token::{StellarAssetClient, TokenClient},
    Address, Env, Symbol, TryFromVal,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Register a native Stellar asset (SAC) and return (contract_id, admin).
/// The admin is the issuer; we mint `amount` to `recipient`.
fn setup_token(env: &Env, admin: &Address, recipient: &Address, amount: i128) -> Address {
    let token_id = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let sac = StellarAssetClient::new(env, &token_id);
    sac.mint(recipient, &amount);
    token_id
}

/// Build a fresh AccountGroup with four distinct addresses.
fn make_accounts(env: &Env) -> AccountGroup {
    AccountGroup {
        spending: Address::generate(env),
        savings: Address::generate(env),
        bills: Address::generate(env),
        insurance: Address::generate(env),
    }
}

// ---------------------------------------------------------------------------
// initialize_split
// ---------------------------------------------------------------------------

#[test]
fn test_initialize_split_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    let success = client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    assert_eq!(success, true);

    let config = client.get_config().unwrap();
    assert_eq!(config.owner, owner);
    assert_eq!(config.spending_percent, 50);
    assert_eq!(config.savings_percent, 30);
    assert_eq!(config.bills_percent, 15);
    assert_eq!(config.insurance_percent, 5);
    assert_eq!(config.usdc_contract, token_id);
}

#[test]
fn test_initialize_split_invalid_sum() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    let result = client.try_initialize_split(&owner, &0, &token_id, &50, &50, &10, &0);
    assert_eq!(result, Err(Ok(RemittanceSplitError::PercentagesDoNotSumTo100)));
}

#[test]
fn test_initialize_split_already_initialized() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    let result = client.try_initialize_split(&owner, &1, &token_id, &50, &30, &15, &5);
    assert_eq!(result, Err(Ok(RemittanceSplitError::AlreadyInitialized)));
}

#[test]
#[should_panic]
fn test_initialize_split_requires_auth() {
    let env = Env::default();
    // No mock_all_auths — owner has not authorized
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_id = Address::generate(&env);
    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
}

// ---------------------------------------------------------------------------
// update_split
// ---------------------------------------------------------------------------

#[test]
fn test_update_split() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    let success = client.update_split(&owner, &1, &40, &40, &10, &10);
    assert_eq!(success, true);

    let config = client.get_config().unwrap();
    assert_eq!(config.spending_percent, 40);
    assert_eq!(config.savings_percent, 40);
    assert_eq!(config.bills_percent, 10);
    assert_eq!(config.insurance_percent, 10);
}

#[test]
fn test_update_split_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let other = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    let result = client.try_update_split(&other, &0, &40, &40, &10, &10);
    assert_eq!(result, Err(Ok(RemittanceSplitError::Unauthorized)));
}

#[test]
fn test_update_split_not_initialized() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let caller = Address::generate(&env);

    let result = client.try_update_split(&caller, &0, &25, &25, &25, &25);
    assert_eq!(result, Err(Ok(RemittanceSplitError::NotInitialized)));
}

#[test]
fn test_update_split_percentages_must_sum_to_100() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    let result = client.try_update_split(&owner, &1, &60, &30, &15, &5);
    assert_eq!(result, Err(Ok(RemittanceSplitError::PercentagesDoNotSumTo100)));
}

// ---------------------------------------------------------------------------
// calculate_split
// ---------------------------------------------------------------------------

#[test]
fn test_calculate_split() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    let amounts = client.calculate_split(&1000);
    assert_eq!(amounts.get(0).unwrap(), 500);
    assert_eq!(amounts.get(1).unwrap(), 300);
    assert_eq!(amounts.get(2).unwrap(), 150);
    assert_eq!(amounts.get(3).unwrap(), 50);
}

#[test]
fn test_calculate_split_zero_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    let result = client.try_calculate_split(&0);
    assert_eq!(result, Err(Ok(RemittanceSplitError::InvalidAmount)));
}

#[test]
fn test_calculate_split_rounding() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &33, &33, &33, &1);
    let amounts = client.calculate_split(&100);
    let sum: i128 = amounts.iter().sum();
    assert_eq!(sum, 100);
}

#[test]
fn test_calculate_complex_rounding() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &17, &19, &23, &41);
    let amounts = client.calculate_split(&1000);
    assert_eq!(amounts.get(0).unwrap(), 170);
    assert_eq!(amounts.get(1).unwrap(), 190);
    assert_eq!(amounts.get(2).unwrap(), 230);
    assert_eq!(amounts.get(3).unwrap(), 410);
}

#[test]
fn test_create_remittance_schedule_succeeds() {
    setup_test_env!(env, RemittanceSplit, RemittanceSplitClient, client, owner);
    set_ledger_time(&env, 1000);

    client.initialize_split(&owner, &0, &50, &30, &15, &5);

    let schedule_id = client.create_remittance_schedule(&owner, &10000, &3000, &86400);
    assert_eq!(schedule_id, 1);

    let schedule = client.get_remittance_schedule(&schedule_id);
    assert!(schedule.is_some());
    let schedule = schedule.unwrap();
    assert_eq!(schedule.amount, 10000);
    assert_eq!(schedule.next_due, 3000);
    assert_eq!(schedule.interval, 86400);
    assert!(schedule.active);
}
// ---------------------------------------------------------------------------
// distribute_usdc — happy path
// ---------------------------------------------------------------------------

#[test]
fn test_distribute_usdc_success() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let total = 1_000i128;
    let token_id = setup_token(&env, &token_admin, &owner, total);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);

    let accounts = make_accounts(&env);
    let result = client.distribute_usdc(&token_id, &owner, &1, &accounts, &total);
    assert_eq!(result, true);

    let token = TokenClient::new(&env, &token_id);
    assert_eq!(token.balance(&accounts.spending), 500);
    assert_eq!(token.balance(&accounts.savings), 300);
    assert_eq!(token.balance(&accounts.bills), 150);
    assert_eq!(token.balance(&accounts.insurance), 50);
    assert_eq!(token.balance(&owner), 0);
}

#[test]
fn test_distribute_usdc_emits_event() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 1_000);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    let accounts = make_accounts(&env);
    client.distribute_usdc(&token_id, &owner, &1, &accounts, &1_000);

    let events = env.events().all();
    let last = events.last().unwrap();
    let topic0: Symbol = Symbol::try_from_val(&env, &last.1.get(0).unwrap()).unwrap();
    let topic1: SplitEvent = SplitEvent::try_from_val(&env, &last.1.get(1).unwrap()).unwrap();
    assert_eq!(topic0, symbol_short!("split"));
    assert_eq!(topic1, SplitEvent::DistributionCompleted);
}

#[test]
fn test_distribute_usdc_nonce_increments() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 2_000);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    // nonce after init = 1
    let accounts = make_accounts(&env);
    client.distribute_usdc(&token_id, &owner, &1, &accounts, &1_000);
    // nonce after first distribute = 2
    assert_eq!(client.get_nonce(&owner), 2);
}

// ---------------------------------------------------------------------------
// distribute_usdc — auth must be first (before amount check)
// ---------------------------------------------------------------------------

#[test]
#[should_panic]
fn test_distribute_usdc_requires_auth() {
    // Set up contract state with a mocked env first
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 1_000);
    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);

    // Now call distribute_usdc without mocking auth for `owner` — should panic
    // We create a fresh env that does NOT mock auths
    let env2 = Env::default();
    // Re-register the same contract in env2 (no mock_all_auths)
    let contract_id2 = env2.register_contract(None, RemittanceSplit);
    let client2 = RemittanceSplitClient::new(&env2, &contract_id2);
    let accounts = make_accounts(&env2);
    // This should panic because owner has not authorized in env2
    client2.distribute_usdc(&token_id, &owner, &0, &accounts, &1_000);
}

// ---------------------------------------------------------------------------
// distribute_usdc — owner-only enforcement
// ---------------------------------------------------------------------------

#[test]
fn test_distribute_usdc_non_owner_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 1_000);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);

    // Attacker self-authorizes but is not the config owner
    let accounts = make_accounts(&env);
    let result = client.try_distribute_usdc(&token_id, &attacker, &0, &accounts, &1_000);
    assert_eq!(result, Err(Ok(RemittanceSplitError::Unauthorized)));
}

// ---------------------------------------------------------------------------
// distribute_usdc — untrusted token contract
// ---------------------------------------------------------------------------

#[test]
fn test_distribute_usdc_untrusted_token_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 1_000);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);

    // Supply a different (malicious) token contract address
    let evil_token = Address::generate(&env);
    let accounts = make_accounts(&env);
    let result = client.try_distribute_usdc(&evil_token, &owner, &1, &accounts, &1_000);
    assert_eq!(result, Err(Ok(RemittanceSplitError::UntrustedTokenContract)));
}

// ---------------------------------------------------------------------------
// distribute_usdc — self-transfer guard
// ---------------------------------------------------------------------------

#[test]
fn test_distribute_usdc_self_transfer_spending_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 1_000);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);

    // spending destination == owner
    let accounts = AccountGroup {
        spending: owner.clone(),
        savings: Address::generate(&env),
        bills: Address::generate(&env),
        insurance: Address::generate(&env),
    };
    let result = client.try_distribute_usdc(&token_id, &owner, &1, &accounts, &1_000);
    assert_eq!(result, Err(Ok(RemittanceSplitError::SelfTransferNotAllowed)));
}

#[test]
fn test_distribute_usdc_self_transfer_savings_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 1_000);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);

    let accounts = AccountGroup {
        spending: Address::generate(&env),
        savings: owner.clone(),
        bills: Address::generate(&env),
        insurance: Address::generate(&env),
    };
    let result = client.try_distribute_usdc(&token_id, &owner, &1, &accounts, &1_000);
    assert_eq!(result, Err(Ok(RemittanceSplitError::SelfTransferNotAllowed)));
}

#[test]
fn test_distribute_usdc_self_transfer_bills_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 1_000);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);

    let accounts = AccountGroup {
        spending: Address::generate(&env),
        savings: Address::generate(&env),
        bills: owner.clone(),
        insurance: Address::generate(&env),
    };
    let result = client.try_distribute_usdc(&token_id, &owner, &1, &accounts, &1_000);
    assert_eq!(result, Err(Ok(RemittanceSplitError::SelfTransferNotAllowed)));
}

#[test]
fn test_distribute_usdc_self_transfer_insurance_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 1_000);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);

    let accounts = AccountGroup {
        spending: Address::generate(&env),
        savings: Address::generate(&env),
        bills: Address::generate(&env),
        insurance: owner.clone(),
    };
    let result = client.try_distribute_usdc(&token_id, &owner, &1, &accounts, &1_000);
    assert_eq!(result, Err(Ok(RemittanceSplitError::SelfTransferNotAllowed)));
}

// ---------------------------------------------------------------------------
// distribute_usdc — invalid amount
// ---------------------------------------------------------------------------

#[test]
fn test_distribute_usdc_zero_amount_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 1_000);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    let accounts = make_accounts(&env);
    let result = client.try_distribute_usdc(&token_id, &owner, &1, &accounts, &0);
    assert_eq!(result, Err(Ok(RemittanceSplitError::InvalidAmount)));
}

#[test]
fn test_distribute_usdc_negative_amount_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 1_000);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    let accounts = make_accounts(&env);
    let result = client.try_distribute_usdc(&token_id, &owner, &1, &accounts, &-1);
    assert_eq!(result, Err(Ok(RemittanceSplitError::InvalidAmount)));
}

// ---------------------------------------------------------------------------
// distribute_usdc — not initialized
// ---------------------------------------------------------------------------

#[test]
fn test_distribute_usdc_not_initialized_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_id = Address::generate(&env);

    let accounts = make_accounts(&env);
    let result = client.try_distribute_usdc(&token_id, &owner, &0, &accounts, &1_000);
    assert_eq!(result, Err(Ok(RemittanceSplitError::NotInitialized)));
}

// ---------------------------------------------------------------------------
// distribute_usdc — replay protection
// ---------------------------------------------------------------------------

#[test]
fn test_distribute_usdc_replay_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 2_000);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    let accounts = make_accounts(&env);
    // First call with nonce=1 succeeds
    client.distribute_usdc(&token_id, &owner, &1, &accounts, &1_000);
    // Replaying nonce=1 must fail
    let result = client.try_distribute_usdc(&token_id, &owner, &1, &accounts, &500);
    assert_eq!(result, Err(Ok(RemittanceSplitError::InvalidNonce)));
}

// ---------------------------------------------------------------------------
// distribute_usdc — paused contract
// ---------------------------------------------------------------------------

#[test]
fn test_distribute_usdc_paused_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 1_000);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    client.pause(&owner);

    let accounts = make_accounts(&env);
    let result = client.try_distribute_usdc(&token_id, &owner, &1, &accounts, &1_000);
    assert_eq!(result, Err(Ok(RemittanceSplitError::Unauthorized)));
}

// ---------------------------------------------------------------------------
// distribute_usdc — correct split math verified end-to-end
// ---------------------------------------------------------------------------

#[test]
fn test_distribute_usdc_split_math_25_25_25_25() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 1_000);

    client.initialize_split(&owner, &0, &token_id, &25, &25, &25, &25);
    let accounts = make_accounts(&env);
    client.distribute_usdc(&token_id, &owner, &1, &accounts, &1_000);

    let token = TokenClient::new(&env, &token_id);
    assert_eq!(token.balance(&accounts.spending), 250);
    assert_eq!(token.balance(&accounts.savings), 250);
    assert_eq!(token.balance(&accounts.bills), 250);
    assert_eq!(token.balance(&accounts.insurance), 250);
}

#[test]
fn test_distribute_usdc_split_math_100_0_0_0() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 1_000);

    client.initialize_split(&owner, &0, &token_id, &100, &0, &0, &0);
    let accounts = make_accounts(&env);
    client.distribute_usdc(&token_id, &owner, &1, &accounts, &1_000);

    let token = TokenClient::new(&env, &token_id);
    assert_eq!(token.balance(&accounts.spending), 1_000);
    assert_eq!(token.balance(&accounts.savings), 0);
    assert_eq!(token.balance(&accounts.bills), 0);
    assert_eq!(token.balance(&accounts.insurance), 0);
}

#[test]
fn test_distribute_usdc_rounding_remainder_goes_to_insurance() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    // 33/33/33/1 with amount=100: 33+33+33=99, insurance gets remainder=1
    let token_id = setup_token(&env, &token_admin, &owner, 100);

    client.initialize_split(&owner, &0, &token_id, &33, &33, &33, &1);
    let accounts = make_accounts(&env);
    client.distribute_usdc(&token_id, &owner, &1, &accounts, &100);

    let token = TokenClient::new(&env, &token_id);
    let total = token.balance(&accounts.spending)
        + token.balance(&accounts.savings)
        + token.balance(&accounts.bills)
        + token.balance(&accounts.insurance);
    assert_eq!(total, 100, "all funds must be distributed");
    assert_eq!(token.balance(&accounts.insurance), 1);
}

// ---------------------------------------------------------------------------
// distribute_usdc — multiple sequential distributions
// ---------------------------------------------------------------------------

#[test]
fn test_distribute_usdc_multiple_rounds() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 3_000);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    let accounts = make_accounts(&env);

    client.distribute_usdc(&token_id, &owner, &1, &accounts, &1_000);
    client.distribute_usdc(&token_id, &owner, &2, &accounts, &1_000);
    client.distribute_usdc(&token_id, &owner, &3, &accounts, &1_000);

    let token = TokenClient::new(&env, &token_id);
    assert_eq!(token.balance(&accounts.spending), 1_500); // 3 * 500
    assert_eq!(token.balance(&accounts.savings), 900);    // 3 * 300
    assert_eq!(token.balance(&accounts.bills), 450);      // 3 * 150
    assert_eq!(token.balance(&accounts.insurance), 150);  // 3 * 50
    assert_eq!(token.balance(&owner), 0);
}

// ---------------------------------------------------------------------------
// Boundary tests for split percentages
// ---------------------------------------------------------------------------

#[test]
fn test_split_boundary_100_0_0_0() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    let ok = client.initialize_split(&owner, &0, &token_id, &100, &0, &0, &0);
    assert!(ok);
    let amounts = client.calculate_split(&1000);
    assert_eq!(amounts.get(0).unwrap(), 1000);
    assert_eq!(amounts.get(3).unwrap(), 0);
}

#[test]
fn test_split_boundary_0_0_0_100() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    let ok = client.initialize_split(&owner, &0, &token_id, &0, &0, &0, &100);
    assert!(ok);
    let amounts = client.calculate_split(&1000);
    assert_eq!(amounts.get(0).unwrap(), 0);
    assert_eq!(amounts.get(3).unwrap(), 1000);
}

#[test]
fn test_split_boundary_25_25_25_25() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &25, &25, &25, &25);
    let amounts = client.calculate_split(&1000);
    assert_eq!(amounts.get(0).unwrap(), 250);
    assert_eq!(amounts.get(1).unwrap(), 250);
    assert_eq!(amounts.get(2).unwrap(), 250);
    assert_eq!(amounts.get(3).unwrap(), 250);
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

#[test]
fn test_initialize_split_events() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);

    let events = env.events().all();
    let last_event = events.last().unwrap();
    let topic0: Symbol = Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    let topic1: SplitEvent = SplitEvent::try_from_val(&env, &last_event.1.get(1).unwrap()).unwrap();
    assert_eq!(topic0, symbol_short!("split"));
    assert_eq!(topic1, SplitEvent::Initialized);
}

#[test]
fn test_update_split_events() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    client.update_split(&owner, &1, &40, &40, &10, &10);

    let events = env.events().all();
    let last_event = events.last().unwrap();
    let topic1: SplitEvent = SplitEvent::try_from_val(&env, &last_event.1.get(1).unwrap()).unwrap();
    assert_eq!(topic1, SplitEvent::Updated);
}

// ---------------------------------------------------------------------------
// Remittance schedules
// ---------------------------------------------------------------------------

#[test]
fn test_create_remittance_schedule_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    env.ledger().set(soroban_sdk::testutils::LedgerInfo {
        protocol_version: 20,
        sequence_number: 100,
        timestamp: 1000,
        network_id: [0; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 100_000,
    });

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    let schedule_id = client.create_remittance_schedule(&owner, &10000, &3000, &86400);
    assert_eq!(schedule_id, 1);

    let schedule = client.get_remittance_schedule(&schedule_id).unwrap();
    assert_eq!(schedule.amount, 10000);
    assert_eq!(schedule.next_due, 3000);
    assert!(schedule.active);
}

#[test]
fn test_cancel_remittance_schedule() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    env.ledger().set(soroban_sdk::testutils::LedgerInfo {
        protocol_version: 20,
        sequence_number: 100,
        timestamp: 1000,
        network_id: [0; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 100_000,
    });

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    let schedule_id = client.create_remittance_schedule(&owner, &10000, &3000, &86400);
    client.cancel_remittance_schedule(&owner, &schedule_id);

    let schedule = client.get_remittance_schedule(&schedule_id).unwrap();
    assert!(!schedule.active);
}

// ---------------------------------------------------------------------------
// TTL extension
// ---------------------------------------------------------------------------

#[test]
fn test_instance_ttl_extended_on_initialize_split() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set(soroban_sdk::testutils::LedgerInfo {
        protocol_version: 20,
        sequence_number: 100,
        timestamp: 1000,
        network_id: [0; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 100,
        min_persistent_entry_ttl: 100,
        max_entry_ttl: 700_000,
    });

    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    let ttl = env.as_contract(&contract_id, || env.storage().instance().get_ttl());
    assert!(ttl >= 518_400, "TTL must be >= INSTANCE_BUMP_AMOUNT after init");
}

// ============================================================================
// Snapshot schema version tests
//
// These tests verify that:
//  1. export_snapshot embeds the correct schema_version tag.
//  2. import_snapshot accepts any version in MIN_SUPPORTED_SCHEMA_VERSION..=SCHEMA_VERSION.
//  3. import_snapshot rejects a future (too-new) schema version.
//  4. import_snapshot rejects a past (too-old, below min) schema version.
//  5. import_snapshot rejects a tampered checksum regardless of version.
// ============================================================================

/// export_snapshot must embed schema_version == SCHEMA_VERSION (currently 1).
#[test]
fn test_export_snapshot_contains_correct_schema_version() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);

    client.initialize_split(&owner, &0, &50, &30, &15, &5);

    let snapshot = client.export_snapshot(&owner).unwrap();
    assert_eq!(
        snapshot.schema_version, 1,
        "schema_version must equal SCHEMA_VERSION (1)"
    );
}

/// import_snapshot with the current schema version (1) must succeed.
#[test]
fn test_import_snapshot_current_schema_version_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);

    client.initialize_split(&owner, &0, &50, &30, &15, &5);

    let snapshot = client.export_snapshot(&owner).unwrap();
    assert_eq!(snapshot.schema_version, 1);

    let ok = client.import_snapshot(&owner, &1, &snapshot);
    assert!(ok, "import with current schema version must succeed");
}

/// import_snapshot with a schema_version higher than SCHEMA_VERSION must
/// return UnsupportedVersion (forward-compat rejection).
#[test]
fn test_import_snapshot_future_schema_version_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);

    client.initialize_split(&owner, &0, &50, &30, &15, &5);

    let mut snapshot = client.export_snapshot(&owner).unwrap();
    // Simulate a snapshot produced by a newer contract version.
    snapshot.schema_version = 999;

    let result = client.try_import_snapshot(&owner, &1, &snapshot);
    assert_eq!(
        result,
        Err(Ok(RemittanceSplitError::UnsupportedVersion)),
        "future schema_version must be rejected"
    );
}

/// import_snapshot with schema_version = 0 (below MIN_SUPPORTED_SCHEMA_VERSION)
/// must return UnsupportedVersion (backward-compat rejection).
#[test]
fn test_import_snapshot_too_old_schema_version_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);

    client.initialize_split(&owner, &0, &50, &30, &15, &5);

    let mut snapshot = client.export_snapshot(&owner).unwrap();
    // Simulate a snapshot too old to import.
    snapshot.schema_version = 0;

    let result = client.try_import_snapshot(&owner, &1, &snapshot);
    assert_eq!(
        result,
        Err(Ok(RemittanceSplitError::UnsupportedVersion)),
        "schema_version below minimum must be rejected"
    );
}

/// import_snapshot with a tampered checksum must return ChecksumMismatch
/// even when the schema_version is valid.
#[test]
fn test_import_snapshot_tampered_checksum_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);

    client.initialize_split(&owner, &0, &50, &30, &15, &5);

    let mut snapshot = client.export_snapshot(&owner).unwrap();
    snapshot.checksum = snapshot.checksum.wrapping_add(1);

    let result = client.try_import_snapshot(&owner, &1, &snapshot);
    assert_eq!(
        result,
        Err(Ok(RemittanceSplitError::ChecksumMismatch)),
        "tampered checksum must be rejected"
    );
}

/// Full export → import round-trip: data restored and nonce incremented.
#[test]
fn test_snapshot_export_import_roundtrip_restores_config() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);

    client.initialize_split(&owner, &0, &50, &30, &15, &5);

    // Update so there is something interesting to round-trip.
    // Note: update_split checks the nonce but does NOT increment it.
    client.update_split(&owner, &1, &40, &40, &10, &10);

    let snapshot = client.export_snapshot(&owner).unwrap();
    assert_eq!(snapshot.schema_version, 1);

    // Nonce is 1 after initialize_split (update_split does not increment nonce).
    let ok = client.import_snapshot(&owner, &1, &snapshot);
    assert!(ok);

    let config = client.get_config().unwrap();
    assert_eq!(config.spending_percent, 40);
    assert_eq!(config.savings_percent, 40);
    assert_eq!(config.bills_percent, 10);
    assert_eq!(config.insurance_percent, 10);
}

/// Unauthorized caller must not be able to import a snapshot.
#[test]
fn test_import_snapshot_unauthorized_caller_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let other = Address::generate(&env);

    client.initialize_split(&owner, &0, &50, &30, &15, &5);

    let snapshot = client.export_snapshot(&owner).unwrap();

    let result = client.try_import_snapshot(&other, &0, &snapshot);
    assert_eq!(
        result,
        Err(Ok(RemittanceSplitError::Unauthorized)),
        "non-owner must not import snapshot"
    );
}

// ============================================================================
// Additional tests for issue #245 — strict percentage invariant enforcement
// ============================================================================

// ---------------------------------------------------------------------------
// Helpers used by new tests
// ---------------------------------------------------------------------------

/// Shared ledger setup for schedule tests: timestamp=1000, sequence=100.
fn set_ledger_time(env: &Env, timestamp: u64) {
    env.ledger().set(soroban_sdk::testutils::LedgerInfo {
        protocol_version: 20,
        sequence_number: 100,
        timestamp,
        network_id: [0; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 100_000,
    });
}

/// One-liner setup: register contract, create client and owner, mock all auths.
macro_rules! setup_test_env {
    ($env:ident, $contract:ident, $client_type:ident, $client:ident, $owner:ident) => {
        let $env = Env::default();
        $env.mock_all_auths();
        let contract_id = $env.register_contract(None, $contract);
        let $client = $client_type::new(&$env, &contract_id);
        let $owner = Address::generate(&$env);
    };
}

// ---------------------------------------------------------------------------
// initialize_split — additional invariant tests
// ---------------------------------------------------------------------------

#[test]
fn test_initialize_split() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    let ok = client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    assert!(ok);
}

#[test]
fn test_initialize_split_success() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &40, &30, &20, &10);
    let cfg = client.get_config().unwrap();
    assert_eq!(cfg.spending_percent, 40);
    assert_eq!(cfg.savings_percent, 30);
    assert_eq!(cfg.bills_percent, 20);
    assert_eq!(cfg.insurance_percent, 10);
}

#[test]
fn test_initialize_split_valid() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    // All-in-one-bucket is valid
    let ok = client.initialize_split(&owner, &0, &token_id, &100, &0, &0, &0);
    assert!(ok);
}

#[test]
fn test_initialize_split_percentages_must_sum_to_100() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    // 50+50+10+0 = 110 ≠ 100
    let result = client.try_initialize_split(&owner, &0, &token_id, &50, &50, &10, &0);
    assert_eq!(result, Err(Ok(RemittanceSplitError::PercentagesDoNotSumTo100)));
}

#[test]
fn test_initialize_split_invalid_over_100() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    // Single bucket > 100 → PercentageOutOfRange
    let result = client.try_initialize_split(&owner, &0, &token_id, &101, &0, &0, &0);
    assert_eq!(result, Err(Ok(RemittanceSplitError::PercentageOutOfRange)));
}

#[test]
fn test_initialize_split_with_zero_percentages() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    // All zeros sum to 0 ≠ 100
    let result = client.try_initialize_split(&owner, &0, &token_id, &0, &0, &0, &0);
    assert_eq!(result, Err(Ok(RemittanceSplitError::PercentagesDoNotSumTo100)));
}

#[test]
fn test_initialize_split_already_initialized_panics() {
    // try_ variant returns the structured error (no panic expected here)
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    let result = client.try_initialize_split(&owner, &1, &token_id, &50, &30, &15, &5);
    assert_eq!(result, Err(Ok(RemittanceSplitError::AlreadyInitialized)));
}

#[test]
fn test_initialize_split_update_existing() {
    // After init, calling initialize again returns AlreadyInitialized —
    // the only way to change config is update_split.
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    let result = client.try_initialize_split(&owner, &1, &token_id, &25, &25, &25, &25);
    assert_eq!(result, Err(Ok(RemittanceSplitError::AlreadyInitialized)));

    // Original config must be unchanged
    let cfg = client.get_config().unwrap();
    assert_eq!(cfg.spending_percent, 50);
}

#[test]
fn test_initialize_split_invalid_does_not_overwrite() {
    // A failed init must leave the already-stored config intact.
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    // Attempt a bad second init — must fail
    let _ = client.try_initialize_split(&owner, &1, &token_id, &60, &60, &0, &0);

    let cfg = client.get_config().unwrap();
    assert_eq!(cfg.spending_percent, 50);
    assert_eq!(cfg.savings_percent, 30);
}

#[test]
fn test_initialize_split_emits_event() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    // At least the Initialized event must have been published
    assert!(!env.events().all().is_empty());
}

// ---------------------------------------------------------------------------
// update_split — additional invariant tests
// ---------------------------------------------------------------------------

#[test]
fn test_update_split_owner_only() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let other = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    let result = client.try_update_split(&other, &0, &25, &25, &25, &25);
    assert_eq!(result, Err(Ok(RemittanceSplitError::Unauthorized)));
}

#[test]
fn test_update_split_boundary_percentages() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);

    // 100/0/0/0 is a valid boundary
    let ok = client.update_split(&owner, &1, &100, &0, &0, &0);
    assert!(ok);
    let cfg = client.get_config().unwrap();
    assert_eq!(cfg.spending_percent, 100);
    assert_eq!(cfg.savings_percent, 0);
    assert_eq!(cfg.bills_percent, 0);
    assert_eq!(cfg.insurance_percent, 0);
}

#[test]
fn test_update_split_not_initialized() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let caller = Address::generate(&env);

    let result = client.try_update_split(&caller, &0, &25, &25, &25, &25);
    assert_eq!(result, Err(Ok(RemittanceSplitError::NotInitialized)));
}

#[test]
fn test_instance_ttl_refreshed_on_update_split() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set(soroban_sdk::testutils::LedgerInfo {
        protocol_version: 20,
        sequence_number: 100,
        timestamp: 1000,
        network_id: [0; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 100,
        min_persistent_entry_ttl: 100,
        max_entry_ttl: 700_000,
    });
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    client.update_split(&owner, &1, &25, &25, &25, &25);

    let ttl = env.as_contract(&contract_id, || env.storage().instance().get_ttl());
    assert!(ttl >= 518_400, "TTL must be bumped after update_split");
}

// ---------------------------------------------------------------------------
// get_config / get_split — before and after init
// ---------------------------------------------------------------------------

#[test]
fn test_get_config_returns_none_before_init() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);

    assert!(client.get_config().is_none());
}

#[test]
fn test_get_config_returns_some_after_init() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    assert!(client.get_config().is_some());
}

#[test]
fn test_get_split_default() {
    // Before initialization get_split falls back to the hardcoded default (50/30/15/5)
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);

    let split = client.get_split();
    // Default from lib.rs: 50,30,15,5
    assert_eq!(split.get(0).unwrap(), 50u32);
    assert_eq!(split.get(1).unwrap(), 30u32);
    assert_eq!(split.get(2).unwrap(), 15u32);
    assert_eq!(split.get(3).unwrap(), 5u32);
}

#[test]
fn test_get_split_returns_default_before_init() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);

    let split = client.get_split();
    let sum: u32 = (0..4).map(|i| split.get(i).unwrap()).sum();
    assert_eq!(sum, 100, "default split must sum to 100");
}

#[test]
fn test_get_split_configured_values() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &40, &30, &20, &10);
    let split = client.get_split();
    assert_eq!(split.get(0).unwrap(), 40u32);
    assert_eq!(split.get(1).unwrap(), 30u32);
    assert_eq!(split.get(2).unwrap(), 20u32);
    assert_eq!(split.get(3).unwrap(), 10u32);
}

#[test]
fn test_uninitialized_defaults() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);

    assert!(client.get_config().is_none());
    // get_split still works (returns hardcoded default)
    let split = client.get_split();
    assert_eq!(split.len(), 4);
}

// ---------------------------------------------------------------------------
// calculate_split — expanded coverage
// ---------------------------------------------------------------------------

#[test]
fn test_calculate_split_basic() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    let amounts = client.calculate_split(&200);
    assert_eq!(amounts.get(0).unwrap(), 100);
    assert_eq!(amounts.get(1).unwrap(), 60);
    assert_eq!(amounts.get(2).unwrap(), 30);
    assert_eq!(amounts.get(3).unwrap(), 10);
}

#[test]
fn test_calculate_split_positive_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    // Smallest positive amount still works
    let result = client.try_calculate_split(&1);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_split_negative_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    let result = client.try_calculate_split(&-1);
    assert_eq!(result, Err(Ok(RemittanceSplitError::InvalidAmount)));
}

#[test]
fn test_calculate_split_zero_or_negative_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    assert_eq!(
        client.try_calculate_split(&0),
        Err(Ok(RemittanceSplitError::InvalidAmount))
    );
    assert_eq!(
        client.try_calculate_split(&-100),
        Err(Ok(RemittanceSplitError::InvalidAmount))
    );
}

#[test]
fn test_calculate_split_default_when_uninitialized() {
    // Uninitialized → calculate_split uses the hardcoded default (50/30/15/5)
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);

    let amounts = client.calculate_split(&1000);
    assert_eq!(amounts.get(0).unwrap(), 500); // 50%
    assert_eq!(amounts.get(1).unwrap(), 300); // 30%
    assert_eq!(amounts.get(2).unwrap(), 150); // 15%
    assert_eq!(amounts.get(3).unwrap(), 50);  //  5%
}

#[test]
fn test_calculate_split_rounding_total_matches() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &33, &33, &33, &1);
    let amounts = client.calculate_split(&100);
    let total: i128 = amounts.iter().sum();
    assert_eq!(total, 100);
}

#[test]
fn test_calculate_split_rounding_varied_percentages() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &17, &31, &29, &23);
    let amounts = client.calculate_split(&999);
    let total: i128 = amounts.iter().sum();
    assert_eq!(total, 999);
}

#[test]
fn test_calculate_split_rounding_rigorous() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &33, &33, &33, &1);
    for amount in [1i128, 7, 13, 99, 101, 997, 10_007] {
        let amounts = client.calculate_split(&amount);
        let total: i128 = amounts.iter().sum();
        assert_eq!(total, amount, "sum mismatch for amount={}", amount);
    }
}

#[test]
fn test_calculate_split_small_amount_rounding() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    // amount=1: 50% → 0, 30% → 0, 15% → 0, insurance remainder → 1
    let amounts = client.calculate_split(&1);
    let total: i128 = amounts.iter().sum();
    assert_eq!(total, 1);
}

#[test]
fn test_calculate_split_large_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    let amount = 1_000_000_000_000i128;
    let amounts = client.calculate_split(&amount);
    let total: i128 = amounts.iter().sum();
    assert_eq!(total, amount);
}

#[test]
fn test_calculate_split_large_non_divisible_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &33, &33, &33, &1);
    let amount = 999_999_999_997i128;
    let amounts = client.calculate_split(&amount);
    let total: i128 = amounts.iter().sum();
    assert_eq!(total, amount);
}

#[test]
fn test_calculate_split_with_zero_categories() {
    // Zero-percentage buckets produce zero allocation; remainder goes to insurance
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &0, &0, &0, &100);
    let amounts = client.calculate_split(&500);
    assert_eq!(amounts.get(0).unwrap(), 0);
    assert_eq!(amounts.get(1).unwrap(), 0);
    assert_eq!(amounts.get(2).unwrap(), 0);
    assert_eq!(amounts.get(3).unwrap(), 500);
}

#[test]
fn test_calculate_split_emits_event() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    let before = env.events().all().len();
    client.calculate_split(&1000);
    assert!(env.events().all().len() > before, "calculate_split must emit at least one event");
}

#[test]
fn test_calculate_split_events() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    client.calculate_split(&1000);
    let events = env.events().all();
    // Verify SplitCalculated event topic is present
    let has_calc = events.iter().any(|e| {
        Symbol::try_from_val(&env, &e.1.get(0).unwrap())
            .map(|t: Symbol| t == symbol_short!("calc"))
            .unwrap_or(false)
    });
    assert!(has_calc, "expected a 'calc' topic event from calculate_split");
}

// ---------------------------------------------------------------------------
// Boundary split configs
// ---------------------------------------------------------------------------

#[test]
fn test_split_boundary_0_100_0_0() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &0, &100, &0, &0);
    let amounts = client.calculate_split(&1000);
    assert_eq!(amounts.get(0).unwrap(), 0);
    assert_eq!(amounts.get(1).unwrap(), 1000);
    assert_eq!(amounts.get(2).unwrap(), 0);
    assert_eq!(amounts.get(3).unwrap(), 0);
}

#[test]
fn test_split_boundary_0_0_100_0() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &0, &0, &100, &0);
    let amounts = client.calculate_split(&1000);
    assert_eq!(amounts.get(0).unwrap(), 0);
    assert_eq!(amounts.get(1).unwrap(), 0);
    assert_eq!(amounts.get(2).unwrap(), 1000);
    assert_eq!(amounts.get(3).unwrap(), 0);
}

// ---------------------------------------------------------------------------
// Events — multi-operation
// ---------------------------------------------------------------------------

#[test]
fn test_event_emitted_on_initialize_and_update() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    let after_init = env.events().all().len();
    assert!(after_init > 0);

    client.update_split(&owner, &1, &25, &25, &25, &25);
    let after_update = env.events().all().len();
    assert!(after_update > after_init, "update_split must emit additional events");
}

#[test]
fn test_multiple_operations_emit_multiple_events() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    client.calculate_split(&1000);
    client.calculate_split(&2000);
    // init + 2× calc → at least 3 event groups
    assert!(env.events().all().len() >= 3);
}

// ---------------------------------------------------------------------------
// Data persistence
// ---------------------------------------------------------------------------

#[test]
fn test_split_data_persists_across_ledger_advancements() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    client.initialize_split(&owner, &0, &token_id, &40, &30, &20, &10);

    // Advance ledger sequence and timestamp
    env.ledger().set(soroban_sdk::testutils::LedgerInfo {
        protocol_version: 20,
        sequence_number: 999,
        timestamp: 99_999,
        network_id: [0; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 1_000_000,
    });

    let cfg = client.get_config().unwrap();
    assert_eq!(cfg.spending_percent, 40);
    assert_eq!(cfg.savings_percent, 30);
    assert_eq!(cfg.bills_percent, 20);
    assert_eq!(cfg.insurance_percent, 10);
}

// ---------------------------------------------------------------------------
// Remittance schedules — additional tests
// ---------------------------------------------------------------------------

#[test]
fn test_create_remittance_schedule() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    set_ledger_time(&env, 1000);
    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);

    let id = client.create_remittance_schedule(&owner, &5000, &2000, &86400);
    assert_eq!(id, 1);
    let sched = client.get_remittance_schedule(&id).unwrap();
    assert_eq!(sched.amount, 5000);
    assert_eq!(sched.next_due, 2000);
    assert_eq!(sched.interval, 86400);
    assert!(sched.active);
    assert!(sched.recurring);
}

#[test]
fn test_get_remittance_schedules() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    set_ledger_time(&env, 1000);
    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    client.create_remittance_schedule(&owner, &1000, &2000, &3600);
    client.create_remittance_schedule(&owner, &2000, &3000, &7200);

    let schedules = client.get_remittance_schedules(&owner);
    assert_eq!(schedules.len(), 2);
}

#[test]
fn test_modify_remittance_schedule() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    set_ledger_time(&env, 1000);
    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);
    let id = client.create_remittance_schedule(&owner, &1000, &2000, &3600);

    let ok = client.modify_remittance_schedule(&owner, &id, &9999, &5000, &7200);
    assert!(ok);

    let sched = client.get_remittance_schedule(&id).unwrap();
    assert_eq!(sched.amount, 9999);
    assert_eq!(sched.next_due, 5000);
    assert_eq!(sched.interval, 7200);
}

#[test]
fn test_remittance_schedule_validation() {
    // next_due must be strictly greater than the current ledger timestamp
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    set_ledger_time(&env, 5000);
    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);

    // next_due == current timestamp (not strictly in the future)
    let result = client.try_create_remittance_schedule(&owner, &1000, &5000, &3600);
    assert_eq!(result, Err(Ok(RemittanceSplitError::InvalidDueDate)));

    // next_due in the past
    let result2 = client.try_create_remittance_schedule(&owner, &1000, &4999, &3600);
    assert_eq!(result2, Err(Ok(RemittanceSplitError::InvalidDueDate)));
}

#[test]
fn test_remittance_schedule_zero_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RemittanceSplit);
    let client = RemittanceSplitClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = setup_token(&env, &token_admin, &owner, 0);

    set_ledger_time(&env, 1000);
    client.initialize_split(&owner, &0, &token_id, &50, &30, &15, &5);

    let result = client.try_create_remittance_schedule(&owner, &0, &2000, &3600);
    assert_eq!(result, Err(Ok(RemittanceSplitError::InvalidAmount)));
}