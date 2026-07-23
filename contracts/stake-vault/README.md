# Stake Vault Contract

Token staking, lock, and multiplier accessor.

## Functions

- `initialize(admin, token)` — one-time deploy-time setup.
- `stake(user, amount)` — locks tokens and resets the lock timestamp.
- `unstake(user)` — releases funds after the lock period elapsed.
- `get_multiplier(user)` — basis-points multiplier based on stake tier.
- `upgrade_contract(admin, new_wasm_hash)` — admin-only WASM upgrade.

## Two-Step Admin Transfer (Issue #20)

The admin role is rotated through `propose → accept` so a typo'd or
compromised key can be cancelled before locking the contract.

- `propose_new_admin(current_admin, proposed)` — admin-only.
- `accept_admin_ownership(acceptor)` — only the proposed address may
  call.
- `cancel_admin_transfer(caller)` — callable by the current admin OR
  the (typo'd) proposed address.

Follows the soft-timelock pattern shared across all Orivex contracts;
see `contracts/common::two_step` for the global event types
(`TransferProposed` / `TransferAccepted` / `TransferCancelled`).

