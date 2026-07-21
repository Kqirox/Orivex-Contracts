# Reward Pool Contract

Central USDC reward distribution with an approved-spender allowlist.

## Wasm Size Budget

This contract must compile to ≤ 50 KB (enforced in CI).
Soroban's protocol limit is 64 KB; staying under 50 KB preserves
deploy-cost and gas headroom.

## Functions

- `initialize(admin, token)` — one-time deploy-time setup.
- `add_approved_spender(admin, spender)` — admin-only whitelisting.
- `set_pause(admin, status)` — admin-only circuit breaker.
- `distribute_reward(caller, learner, amount)` — caller must be whitelisted.
- `fund_pool(donor, amount)` — donor must authorize the token transfer.
- `emergency_sweep(admin, recovery_wallet)` — admin-only full-balance rescue.
- `upgrade_contract(admin, new_wasm_hash)` — admin-only WASM upgrade.
