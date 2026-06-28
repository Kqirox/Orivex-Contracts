# Deployment checklist

1. Deploy `reward-pool` first; record its address.
2. Deploy `badge-nft`; record its address.
3. Deploy `course-registry`; call `initialize(admin)`, then
   `set_reward_pool_address` and `set_badge_nft_address`.
4. Deploy `stake-vault`; call `initialize(admin, usdc_token)`.
5. Deploy `quest-engine`; call `initialize(admin, usdc_token, reward_pool, stake_vault)`.
6. Whitelist `course-registry` on `reward-pool` via `add_approved_spender`.
7. Whitelist `quest-engine` on `reward-pool` via `add_approved_spender`.
8. Deploy `governance`; call `initialize(admin, badge_nft)`.
9. Verify end-to-end by completing a single-module course end-to-end.
