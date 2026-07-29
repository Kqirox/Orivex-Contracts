# Course Registry Contract

A Soroban smart contract managing the full course lifecycle for the Orivex platform — from creation and enrollment through module completion, badge minting, and reward distribution.

## Overview

The Course Registry provides functionality to:
- Create and manage courses with metadata
- Enroll learners (individually or in batches)
- Track per-learner module progress
- Mark modules as completed (admin-verified)
- Mint soulbound NFT badges on course completion
- Trigger reward payouts via the Reward Pool
- Support contract upgrades with versioned storage migrations

## Functions

### Admin Setup

#### `initialize(env, admin)`

One-time deploy-time setup. Stores the Protocol Admin address in instance storage. Panics with `AlreadyInitialized` if called twice.

**Auth:** `admin.require_auth()` — cryptographic signature required.

#### `migrate(env, admin)`

Applies pending storage-schema migrations after a WASM upgrade. Reads the on-chain version, runs any required migrations, and writes the new `VERSION` constant.

**Auth:** Admin-only. Panics with `"Already at current version"` if the schema is already up to date.

#### `contract_version(env) -> u32`

Returns the schema version stored in instance storage (0 = pre-versioning baseline).

### Course Management

#### `create_course(env, admin, instructor, total_modules, metadata_hash) -> u32`

Allocates a new course with a monotonically-increasing ID.

**Parameters:**
- `admin: Address` — Protocol Admin (must authenticate)
- `instructor: Address` — The course instructor/owner
- `total_modules: u32` — Module count; must be > 0 and ≤ `DEFAULT_TOTAL_MODULES_BOUND` (1000)
- `metadata_hash: BytesN<32>` — IPFS CID of course metadata (32-byte hash)

**Returns:** The newly allocated `u32` course ID.

**Panics:**
- `"total_modules must be greater than 0"`
- `"total_modules exceeds bound"` (above 1000)
- `"Course ID limit reached"` (u32::MAX courses)
- `"Unauthorized: Caller is not the protocol admin"`

**Events:** `CourseCreated { id, instructor, total_modules }`

#### `set_course_status(env, admin, id, active)`

Toggles a course's active flag. Inactive courses block new enrollments but preserve existing learner progress.

**Auth:** Protocol Admin only.

**Panics:** `"Course not found"` if the ID is invalid.

**Events:** `CourseStatusChanged { id, active }`

#### `get_course(env, id) -> Course`

Returns the full `Course` struct for the given ID.

**Returns:** `Course { instructor: Address, total_modules: u32, metadata_hash: BytesN<32>, active: bool }`

**Panics:** `"Course not found"` if the ID has no record.

#### `course_count(env) -> u32`

Returns the total number of registered courses. Returns 0 before the first `create_course` call.

#### `update_metadata(env, id, new_hash)`

Replaces a course's IPFS metadata hash. Only the current course instructor may call this.

**Auth:** `course.instructor.require_auth()` — current instructor must sign.

**Panics:** `"Course not found"`

**Events:** `MetadataUpdated { id, instructor, new_hash }`

#### `transfer_ownership(env, current_instructor, new_instructor, course_id)`

Transfers course ownership to a new instructor address.

**Auth:** `current_instructor.require_auth()`. Panics with `"Unauthorized: Caller is not the course instructor"` if the caller doesn't match.

**Events:** `OwnershipTransferred { course_id, previous_instructor, new_instructor }`

### Enrollment

#### `enroll(env, learner, id)`

Initializes a learner's progress record to 0 for the given course. The learner must sign the transaction.

**Auth:** `learner.require_auth()`

**Panics:**
- `"Course not found"`
- `"Course is not active"` (the course is deactivated)
- `"Learner already enrolled"` (progress record already exists)

**Storage:** Writes `DataKey::Progress(learner, id) = 0u32`.

#### `enroll_many(env, admin, learners, id) -> Vec<bool>`

Batch-enrolls multiple learners in a single transaction (cohort onboarding). Admin-authorized.

**Auth:** Protocol Admin only.

**Returns:** `Vec<bool>` — `true` for each newly-enrolled learner, `false` for each already-enrolled (skipped) learner. Does not abort on partial failure.

**Panics:**
- `"Course not found"`
- `"Course is not active"`
- `"Unauthorized: Caller is not the protocol admin"`

#### `is_enrolled(env, learner, id) -> bool`

Returns `true` if the learner has a progress record for the course (including completed courses).

#### `unenroll(env, learner, id)`

Removes a learner's progress record from storage, allowing re-enrollment.

**Auth:** `learner.require_auth()`

**Panics:**
- `"Course not found"`
- `"Already completed"` — completed courses cannot be unenrolled (completion is irreversible)
- (Soroban host error if the caller is not the learner)

**Events:** `Unenrolled { learner, course_id }`

**Storage:** Removes `DataKey::Progress(learner, id)` from persistent storage.

### Progress & Completion

#### `get_progress(env, learner, id) -> u32`

Returns a learner's completed module count. Returns 0 for unenrolled learners (no panic).

#### `is_course_finished(env, learner, id) -> bool`

Returns `true` when the learner's progress ≥ `course.total_modules`. Defensive: excess progress also counts as finished.

**Panics:** `"Course not found"` for invalid course IDs.

#### `complete_module(env, verifier, learner, id)`

Records a verifier-confirmed module completion. On the final module, mints a soulbound badge and triggers reward distribution.

**Auth:** Protocol Admin only (the verifier).

**Panics:**
- `"Course not found"`
- `"Course already completed"` (progress already at total_modules)
- `"NotInitialized"` / `"Unauthorized: Caller is not the protocol admin"`

**Events:**
- `ModuleCompleted { learner, course_id, new_progress }` — every module
- `CourseCompleted { learner, course_id, reward_amount }` — final module only (if reward distributed)

**Cross-contract calls:**
- Mints `BadgeNFT` via the configured badge address (if set)
- Triggers `RewardPool::try_distribute_reward` (if configured)

Follows the checks-effects-interactions pattern. If reward distribution fails, a `PendingReward` record is stored for later retry.

#### `claim_completion_reward(env, learner, course_id)`

Retries a previously-failed reward payout.

**Auth:** `learner.require_auth()`

**Panics:** `"No pending reward for this learner and course"`

### Cross-Contract Wiring

#### `set_reward_pool_address(env, admin, reward_pool_address)`

Registers the RewardPool contract address so course completions can trigger payouts.

**Auth:** Protocol Admin only.

#### `set_badge_nft_address(env, admin, badge_nft_address)`

Registers the BadgeNFT contract address so completed courses can mint soulbound badges.

**Auth:** Protocol Admin only.

### Upgrade

#### `upgrade_contract(env, admin, new_wasm_hash)`

Replaces the contract WASM with the supplied hash on the Soroban host. After swapping, the caller must invoke `migrate()` in a subsequent transaction.

**Auth:** Protocol Admin only.

**Events:** `ContractUpgraded { admin, new_wasm_hash }`

## Events

All events use Soroban's typed `#[contractevent]` with `#[topic]` attributes for efficient indexing.

| Event | Topics | Data |
|-------|--------|------|
| `CourseCreated` | `id`, `instructor` | `total_modules` |
| `CourseStatusChanged` | `id` | `active` |
| `MetadataUpdated` | `id`, `instructor` | `new_hash` |
| `OwnershipTransferred` | `course_id`, `previous_instructor` | `new_instructor` |
| `ModuleCompleted` | `learner`, `course_id` | `new_progress` |
| `CourseCompleted` | `learner`, `course_id` | `reward_amount` |
| `Unenrolled` | `learner`, `course_id` | — |
| `ContractUpgraded` | `admin` | `new_wasm_hash` |

## Storage Layout

### Instance Storage (per-contract singleton)

| Key | Type | Description |
|-----|------|-------------|
| `DataKey::Admin` | `Address` | Protocol Admin address |
| `DataKey::CourseCount` | `u32` | Monotonically-increasing course counter |
| `DataKey::BadgeNftAddress` | `Address` | BadgeNFT contract (optional) |
| `DataKey::RewardPoolAddress` | `Address` | RewardPool contract (optional) |
| `DataKey::Version` | `u32` | Schema version (0 = pre-versioning) |

### Persistent Storage (per-entity)

| Key | Type | Description |
|-----|------|-------------|
| `DataKey::Course(id)` | `Course` | Full course record |
| `DataKey::Progress(learner, course_id)` | `u32` | Learner's completed module count |
| `DataKey::PendingReward(learner, course_id)` | `bool` | Pending reward flag for retry |

## Auth Model

| Function | Caller | Auth Check |
|----------|--------|------------|
| `initialize` | Deployer | `admin.require_auth()` |
| `create_course` | Protocol Admin | `admin.require_auth()` + stored admin check |
| `set_course_status` | Protocol Admin | `admin.require_auth()` + stored admin check |
| `update_metadata` | Course Instructor | `course.instructor.require_auth()` |
| `transfer_ownership` | Current Instructor | `current_instructor.require_auth()` + equality check |
| `enroll` | Learner | `learner.require_auth()` |
| `enroll_many` | Protocol Admin | `admin.require_auth()` + stored admin check |
| `unenroll` | Learner | `learner.require_auth()` |
| `complete_module` | Protocol Admin | `verifier.require_auth()` + stored admin check |
| `claim_completion_reward` | Learner | `learner.require_auth()` |
| `set_reward_pool_address` | Protocol Admin | `admin.require_auth()` + stored admin check |
| `set_badge_nft_address` | Protocol Admin | `admin.require_auth()` + stored admin check |
| `upgrade_contract` | Protocol Admin | `admin.require_auth()` + stored admin check |
| `migrate` | Protocol Admin | `admin.require_auth()` + stored admin check |
| `get_course` | Anyone | None (read-only) |
| `get_progress` | Anyone | None (read-only) |
| `is_enrolled` | Anyone | None (read-only) |
| `is_course_finished` | Anyone | None (read-only) |
| `course_count` | Anyone | None (read-only) |
| `contract_version` | Anyone | None (read-only) |

## Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `VERSION` | `1` | Current storage schema version |
| `INITIAL_COURSE_ID` | `1` | First course ID (1-based) |
| `MAX_COURSE_ID` | `u32::MAX` | Maximum course count |
| `DEFAULT_TOTAL_MODULES_BOUND` | `1000` | Max modules per course |
| `BASE_REWARD_AMOUNT` | `10_0000000` | 10 USDC (7 decimals) |

## Cross-Contract Dependencies

The Course Registry integrates with two sibling contracts:

1. **BadgeNFT** — Called via `BadgeNFTClient::mint_badge` on course completion to mint a soulbound badge for the learner.
2. **RewardPool** — Called via `RewardPoolClient::try_distribute_reward` on course completion to pay out the learner's reward. The Course Registry must be whitelisted via `RewardPool::add_approved_spender` before payouts succeed.

Both addresses are configured via their respective setter functions by the Protocol Admin.

## Building

```bash
cargo build -p course-registry --release
```

## Testing

```bash
cargo test -p course-registry --lib
```
