# Reward Pool Contract

Central USDC reward distribution with an approved-spender allowlist.

## Functions

- `initialize(admin, token)` — one-time deploy-time setup.
- `add_approved_spender(admin, spender)` — admin-only whitelisting.
- `set_pause(admin, status)` — admin-only circuit breaker.
- `distribute_reward(caller, learner, amount)` — caller must be whitelisted.
- `fund_pool(donor, amount)` — donor must authorize the token transfer.
- `emergency_sweep(admin, recovery_wallet)` — admin-only full-balance rescue.
- `upgrade_contract(admin, new_wasm_hash)` — admin-only WASM upgrade.

## Two-Step Admin Transfer (Issue #20)

The admin role is rotated through `propose → accept` so a typo or
compromised-key incident can be cancelled before locking out the
contract forever.

- `propose_new_admin(current_admin, proposed)` — admin-only. Stores a
  `PendingTransfer` under `DataKey::PendingAdmin` and emits
  `TransferProposed { current, proposed, proposed_at }`.
- `accept_admin_ownership(acceptor)` — only the proposed address may
  call. Overwrites `DataKey::Admin`, clears the pending record, emits
  `TransferAccepted`.
- `cancel_admin_transfer(caller)` — callable by the current admin OR
  the (typo'd) proposed address. Clears the pending record, emits
  `TransferCancelled`.

The timelock is **soft**: the proposed address can accept immediately.
Off-chain monitors are expected to alert on `TransferProposed` so
communities can react before acceptance. See
`contracts/common::two_step` for the shared types and events.

