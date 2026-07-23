# Governance Contract

Badge-weighted proposal lifecycle.

## Functions

- `initialize(admin, badge_contract)` — one-time deploy-time setup.
- `get_proposal(proposal_id)` — returns the proposal struct.
- `cast_vote(voter, proposal_id, support)` — single vote per (voter, proposal).
- `execute_proposal(proposal_id)` — marks a passed proposal as executed.
- `cancel_proposal(caller, proposal_id)` — proposer or admin only.
- `upgrade_contract(admin, new_wasm_hash)` — admin-only WASM upgrade.

## Two-Step Transfer (Issue #20)

Both `Admin` and the `BadgeContractAddress` reference are rotated
through `propose → accept`. The `BadgeContractAddress` two-step is the
only path to rotate the badge contract after init.

- `propose_new_admin(current_admin, proposed)` / `accept_admin_ownership(acceptor)`
  / `cancel_admin_transfer(caller)`.
- `propose_new_badge_contract_address(current_admin, proposed)` /
  `accept_badge_contract_address(acceptor)` /
  `cancel_badge_contract_transfer(caller)`.

All three steps for each role emit the shared events
`TransferProposed` / `TransferAccepted` / `TransferCancelled` from
`contracts/common::two_step`, with a soft timelock (immediate accept).

