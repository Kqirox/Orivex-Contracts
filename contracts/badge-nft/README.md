# Badge NFT Contract

Soulbound badge issuance, retrieval, and admin revocation.

## Wasm Size Budget

This contract must compile to ≤ 50 KB (enforced in CI).
Soroban's protocol limit is 64 KB; staying under 50 KB preserves
deploy-cost and gas headroom.

## Functions

- `initialize(admin)` — one-time deploy-time setup.
- `mint_badge(caller, learner, course_id)` — only callable by `admin`. Panics on duplicate (learner, course_id).
- `revoke_badge(admin, learner, course_id)` — admin-only. No-op if badge not found.
- `get_badges(learner)` — returns the learner's vector of badges.
- `get_badge_count(learner)` — returns the count.
- `has_badge(learner, course_id)` — boolean lookup.
- `upgrade_contract(admin, new_wasm_hash)` — admin-only WASM upgrade.
