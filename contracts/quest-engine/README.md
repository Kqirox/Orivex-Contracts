# Quest Engine Contract

Build quests (employer-funded, peer-reviewed) and Explore quests (admin-verified, pool-funded).

## Functions

- `initialize(admin, token, reward_pool, stake_vault)` — one-time deploy-time setup.
- `set_pause(admin, status)` — admin-only circuit breaker.
- `create_build_quest(employer, reward_amount, metadata_hash)` — escrow.
- `create_build_quest_with_allowance(employer, reward_amount, metadata_hash)` — allowance-based escrow.
- `create_explore_quest(admin, reward_amount, metadata_hash)` — pool-backed.
- `submit_proof(learner, quest_id, proof_hash)` — single per learner per quest.
- `review_submission(employer, learner, quest_id, approve)` — single review with multiplier.
- `batch_review_submissions(employer, quest_id, learners)` — bulk review.
- `refund_quest(employer, quest_id)` — employer-only cancel + refund.
- `verify_explore_quest(admin, learner, quest_id)` — admin-only pool payout.
- `get_quest(quest_id)` / `get_submission(learner, quest_id)` — view accessors.
- `upgrade_contract(admin, new_wasm_hash)` — admin-only WASM upgrade.

## Employer Funding Patterns

### Direct Transfer (`create_build_quest`)

The employer sends funds atomically with the quest creation call. This requires the employer to have the exact `reward_amount` available as a balance in the same transaction.

```rust
// One-shot: fund + create in a single invocation
let quest_id = quest_engine.create_build_quest(
    &employer,
    &1_000_i128,
    &metadata_hash,
);
```

### Allowance Pattern (`create_build_quest_with_allowance`)

Employers can pre-authorize the QuestEngine contract to pull funds on demand using the SAC token allowance mechanism. This decouples funding authorization from quest creation timing.

**Step 1: Employer approves the QuestEngine as a spender**

```rust
use soroban_sdk::token;

let token_client = token::Client::new(&env, &token_address);
let current_ledger = env.ledger().sequence();
let expiration_ledger = current_ledger + 525_600; // ~1 month of ledgers

// Approve QuestEngine to spend up to 10_000 on employer's behalf
token_client.approve(
    &employer,
    &quest_engine_address,
    &10_000_i128,
    &expiration_ledger,
);
```

**Step 2: Create quest(s) using the allowance**

```rust
// Create first quest (pulls 1_000 from allowance)
let quest_1 = quest_engine.create_build_quest_with_allowance(
    &employer,
    &1_000_i128,
    &metadata_hash_1,
);

// Create second quest (pulls another 1_000 from allowance)
let quest_2 = quest_engine.create_build_quest_with_allowance(
    &employer,
    &1_000_i128,
    &metadata_hash_2,
);

// Remaining allowance: 8_000
```

**Benefits:**
- Set a single large allowance and create multiple quests without re-approving.
- Decouple funding authorization from quest creation timing.
- Reduce transaction failures from balance/amount mismatches.

**Important:** The `approve` call overwrites (does not add to) any existing allowance. To safely update an allowance, first set it to `0`, verify no pending spends exist, then set the new value. Allowances have a ledger-based expiration — choose an `expiration_ledger` far enough in the future for your workflow.
