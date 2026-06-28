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
