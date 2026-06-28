# Stake Vault Contract

Token staking, lock, and multiplier accessor.

## Functions

- `initialize(admin, token)` — one-time deploy-time setup.
- `stake(user, amount)` — locks tokens and resets the lock timestamp.
- `unstake(user)` — releases funds after the lock period elapsed.
- `get_multiplier(user)` — basis-points multiplier based on stake tier.
- `upgrade_contract(admin, new_wasm_hash)` — admin-only WASM upgrade.
