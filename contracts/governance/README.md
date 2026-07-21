# Governance Contract

Badge-weighted proposal lifecycle.

## Wasm Size Budget

This contract must compile to ≤ 50 KB (enforced in CI).
Soroban's protocol limit is 64 KB; staying under 50 KB preserves
deploy-cost and gas headroom.

## Functions

- `initialize(admin, badge_contract)` — one-time deploy-time setup.
- `get_proposal(proposal_id)` — returns the proposal struct.
- `cast_vote(voter, proposal_id, support)` — single vote per (voter, proposal).
- `execute_proposal(proposal_id)` — marks a passed proposal as executed.
- `cancel_proposal(caller, proposal_id)` — proposer or admin only.
- `upgrade_contract(admin, new_wasm_hash)` — admin-only WASM upgrade.
