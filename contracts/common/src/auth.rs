use soroban_sdk::{Address, Env, Symbol};

use crate::errors::ContractError;

/// Requires that `caller` is the contract admin.
///
/// This function:
/// 1. Calls `caller.require_auth()` to verify the transaction signature
/// 2. Reads the stored admin address from instance storage using the given key
/// 3. Asserts that `caller` matches the stored admin
///
/// # Arguments
/// * `env` - The Soroban environment
/// * `caller` - The address to verify as admin
/// * `admin_key` - The storage key where the admin address is stored
///
/// # Panics
/// Panics with `ContractError::Unauthorized` if the caller is not the admin.
pub fn require_admin(env: &Env, caller: &Address, admin_key: &Symbol) {
    caller.require_auth();

    let stored_admin: Address = env
        .storage()
        .instance()
        .get(admin_key)
        .expect(ContractError::NotInitialized.msg());

    assert!(
        caller == &stored_admin,
        "{}",
        ContractError::Unauthorized.msg()
    );
}

/// Requires that the contract has been initialized.
///
/// # Arguments
/// * `env` - The Soroban environment
/// * `admin_key` - The storage key where the admin address is stored
///
/// # Panics
/// Panics with `ContractError::NotInitialized` if the admin key is not set.
pub fn require_initialized(env: &Env, admin_key: &Symbol) {
    assert!(
        env.storage().instance().has(admin_key),
        "{}",
        ContractError::NotInitialized.msg()
    );
}

/// Initializes the contract by setting the admin address.
///
/// # Arguments
/// * `env` - The Soroban environment
/// * `admin` - The admin address to store
/// * `admin_key` - The storage key where the admin address will be stored
///
/// # Panics
/// Panics with `ContractError::AlreadyInitialized` if the admin is already set.
pub fn initialize_admin(env: &Env, admin: &Address, admin_key: &Symbol) {
    if env.storage().instance().has(admin_key) {
        panic!("{}", ContractError::AlreadyInitialized.msg());
    }
    admin.require_auth();
    env.storage().instance().set(admin_key, admin);
}

/// Checks if the contract is paused.
///
/// # Arguments
/// * `env` - The Soroban environment
/// * `paused_key` - The storage key where the pause flag is stored
///
/// Returns `true` if the contract is paused, `false` otherwise.
pub fn is_paused(env: &Env, paused_key: &Symbol) -> bool {
    env.storage().instance().get(paused_key).unwrap_or(false)
}

/// Requires that the contract is not paused.
///
/// # Arguments
/// * `env` - The Soroban environment
/// * `paused_key` - The storage key where the pause flag is stored
///
/// # Panics
/// Panics with `ContractError::ContractPaused` if the contract is paused.
pub fn require_not_paused(env: &Env, paused_key: &Symbol) {
    assert!(
        !is_paused(env, paused_key),
        "{}",
        ContractError::ContractPaused.msg()
    );
}
