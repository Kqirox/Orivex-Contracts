# Badge NFT Contract

Soulbound badge issuance, retrieval, and admin revocation.

## Functions

- `initialize(admin)` — one-time deploy-time setup.
- `mint_badge(caller, learner, course_id)` — only callable by `admin`. Panics on duplicate (learner, course_id).
- `revoke_badge(admin, learner, course_id)` — admin-only. No-op if badge not found.
- `get_badges(learner)` — returns the learner's vector of badges.
- `get_badge_count(learner)` — returns the count.
- `has_badge(learner, course_id)` — boolean lookup.
- `upgrade_contract(admin, new_wasm_hash)` — admin-only WASM upgrade.

## Two-Step Admin Transfer (Issue #20)

The registry role (`Admin`) is rotated through `propose → accept` so a
typo or compromised-key incident can be cancelled without permanently
locking mint authority to the wrong address.

- `propose_new_admin(current_admin, proposed)` — admin-only. Stores a
  `PendingTransfer` under `DataKey::PendingAdmin` and emits
  `TransferProposed`.
- `accept_admin_ownership(acceptor)` — only the proposed address may
  call. Overwrites `DataKey::Admin`, clears the pending record, emits
  `TransferAccepted`.
- `cancel_admin_transfer(caller)` — callable by the current admin OR
  the (typo'd) proposed address. Clears the pending record, emits
  `TransferCancelled`.

The timelock is **soft**; see `contracts/common::two_step` for details.

