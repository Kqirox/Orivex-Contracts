# Governance Contract

Badge-weighted proposal lifecycle.

## Functions

- `initialize(admin, badge_contract)` — one-time deploy-time setup.
- `get_proposal(proposal_id)` — returns the proposal struct.
- `cast_vote(voter, proposal_id, support)` — single vote per (voter, proposal).
- `execute_proposal(proposal_id)` — marks a passed proposal as executed.
- `cancel_proposal(caller, proposal_id)` — proposer or admin only.
- `upgrade_contract(admin, new_wasm_hash)` — admin-only WASM upgrade.
