# Orivex Contracts

Soroban (Stellar) smart contracts for the Orivex learning-and-rewards protocol.

## Crates

| Crate | Role |
|---|---|
| `course-registry` | Course CRUD, learner progress, soulbound badge mint, USDC payout triggering |
| `badge-nft`     | Soulbound badge issuance, retrieval, and admin revocation |
| `reward-pool`   | USDC funding, distribution, approved-spender gate, emergency sweep |
| `stake-vault`   | Token staking + lock + multiplier accessor |
| `governance`    | Badge-weighted proposal lifecycle |
| `quest-engine`  | Build & Explore quests, submissions, batch review, refunds |
| `common`        | Shared TTL constants and `bump_persistent` helper |

## Build

```
cd contracts
cargo build --target wasm32-unknown-unknown --release
stellar contract build
```

## Test

```
cd contracts
cargo test
```

## Storage TTL Semantics

Soroban persistent storage entries have a live-until ledger (TTL). Without
intervention they expire silently, causing learner badges, progress records,
and other state to disappear. The Orivex protocol follows a **bump-on-touch**
policy to prevent this.

### Constants (`contracts/common/src/lib.rs`)

| Constant | Value | Approximate real time |
|---|---|---|
| `LEDGER_BUMP_PERSISTENT` | 535,000 ledgers | ≈ 30 days at 5 s/ledger |
| `LEDGER_THRESHOLD_PERSISTENT` | 517,000 ledgers | Bump only triggers when remaining TTL drops below this |

The threshold is set 18,000 ledgers (≈ 25 hours) below the bump target. The
Soroban host only writes a new TTL when the current one is below the threshold,
so repeated calls on a hot key incur no extra fee.

### Policy: bump on every touch

Every contract function that performs a persistent storage `get` or `set`
calls `bump_persistent(&env, &key)` immediately afterwards. This resets the
key's live-until ledger to `current_ledger + LEDGER_BUMP_PERSISTENT`.

```rust
use orivex_common::bump_persistent;

// after a persistent write:
env.storage().persistent().set(&DataKey::Course(id), &course);
bump_persistent(&env, &DataKey::Course(id));

// after a persistent read:
let course: Course = env.storage().persistent()
    .get(&DataKey::Course(id))
    .expect("Course not found");
bump_persistent(&env, &DataKey::Course(id));
```

Keys that may not exist (e.g. first-time reads of optional state) are guarded
with `has()` before bumping to avoid charging fees for absent entries:

```rust
if env.storage().persistent().has(&key) {
    bump_persistent(&env, &key);
}
```

### Covered persistent keys

| Contract | Persistent key(s) | Bump sites |
|---|---|---|
| `course-registry` | `Course(u32)`, `Progress(Address, u32)` | `create_course`, `update_metadata`, `enroll`, `set_course_status`, `is_course_finished`, `get_course`, `get_progress`, `transfer_ownership`, `complete_module` |
| `badge-nft` | `UserBadges(Address)` | `mint_badge`, `revoke_badge`, `get_badges` (transitively covers `get_badge_count`, `has_badge`) |
| `reward-pool` | `Spender(Address)` | `add_approved_spender`, `distribute_reward` |
| `stake-vault` | `UserStake(Address)` | `stake`, `unstake`, `get_multiplier` |
| `governance` | `Proposal(u32)`, `UserVote(Address, u32)` | `get_proposal`, `cast_vote`, `cancel_proposal`, `execute_proposal` |
| `quest-engine` | `Quest(u32)`, `Submission(Address, u32)` | `create_build_quest`, `create_explore_quest`, `get_quest`, `submit_proof`, `get_submission`, `review_submission`, `refund_quest`, `batch_review_submissions`, `verify_explore_quest` |

Instance storage (`Admin`, `Token`, etc.) is not listed above because
Soroban automatically ties instance storage TTL to the contract instance
itself, which is managed separately via `extend_ttl` on the instance.

### Audit test

`contracts/common/src/ttl_audit_test.rs` contains six tests — one per
contract — that:

1. Write a persistent key through the contract's public API.
2. Fast-forward the ledger by **100,000 sequences** using
   `env.ledger().with_mut(|li| li.sequence_number += 100_000)`.
3. Read the key back and assert its value is unchanged.

Run the audit tests with:

```
cd contracts
cargo test ttl_audit
```

### Fee considerations

Each `extend_ttl` call is metered by the Soroban host. The threshold guard
(`LEDGER_THRESHOLD_PERSISTENT = 517,000`) prevents a bump on entries whose
TTL is already healthy, which keeps the overhead to a single additional host
function call only when genuinely needed (roughly once every ≈ 25 hours of
continuous use on a hot key).
