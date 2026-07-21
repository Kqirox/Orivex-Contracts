# Quest Engine Contract

Build quests (employer-funded, peer-reviewed) and Explore quests (admin-verified, pool-funded).

## Functions

- `initialize(admin, token, reward_pool, stake_vault)` — one-time deploy-time setup.
- `set_pause(admin, status)` — admin-only circuit breaker.
- `create_build_quest(employer, reward_amount, metadata_hash)` — escrow (direct transfer).
- `create_build_quest_with_allowance(employer, reward_amount, metadata_hash)` — escrow via SAC allowance (pull pattern).
- `create_explore_quest(admin, reward_amount, metadata_hash)` — pool-backed.
- `submit_proof(learner, quest_id, proof_hash)` — single per learner per quest.
- `review_submission(employer, learner, quest_id, approve)` — single review with multiplier.
- `batch_review_submissions(employer, quest_id, learners)` — bulk review.
- `refund_quest(employer, quest_id)` — employer-only cancel + refund.
- `verify_explore_quest(admin, learner, quest_id)` — admin-only pool payout.
- `get_quest(quest_id)` / `get_submission(learner, quest_id)` — view accessors.
- `upgrade_contract(admin, new_wasm_hash)` — admin-only WASM upgrade.

## Employer Guide: SAC Token Allowance Pattern

The standard `create_build_quest` requires the employer to send both the token
approval and the quest creation in a single transaction. For employers who want
to batch-create quests or decouple funding from quest creation, use the SAC
token allowance pattern with `create_build_quest_with_allowance`.

### Flow

1. **Pre-approve** — Call the SAC token's `approve` to authorize the QuestEngine
   to pull funds on the employer's behalf. On testnet/mainnet this call must be
   signed by the employer wallet:
   ```text
   token.approve(
     employer,
     quest_engine_contract_address,
     total_budget,
     expiration_ledger
   );
   ```

2. **Create quests** — Call `create_build_quest_with_allowance` one or more
   times. Each invocation pulls `reward_amount` from the pre-authorized
   allowance into the QuestEngine contract.

3. **Revoke** (optional) — To cancel the remaining allowance, call `approve`
   with amount `0`.

### Comparison

| `create_build_quest` | `create_build_quest_with_allowance` |
|---|---|
| Uses `token::transfer` (push) | Uses `token::transfer_from` (pull) |
| One quest per transaction | Multi-quest with single approval |
| Token approval + escrow in one call | Token approval decoupled from escrow |
