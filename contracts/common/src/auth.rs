use soroban_sdk::{Address, Env};

use crate::errors::ContractError;

/// Requires that the caller is the authorized admin.
///
/// # Arguments
/// * `env` - The Soroban environment
/// * `caller` - The address of the caller
/// * `stored_admin` - The expected admin address stored in contract storage
///
/// # Panics
/// * If the caller is not the authorized admin
///
/// # Example
/// ```ignore
/// pub fn admin_only_function(env: Env, admin: Address) {
///     let stored_admin: Address = env.storage().instance().get(&DataKey::Admin)
///         .expect("Contract not initialized");
///     require_admin(&env, &admin, &stored_admin);
///     // ... business logic
/// }
/// ```
pub fn require_admin(env: &Env, caller: &Address, stored_admin: &Address) {
    caller.require_auth();

    if caller != stored_admin {
        soroban_sdk::panic_with_error!(env, ContractError::Unauthorized);
    }
}
