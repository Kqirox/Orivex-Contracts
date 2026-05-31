# Circuit Breaker Implementation - Testing Guide

## Overview

This document provides step-by-step instructions to verify the circuit breaker (emergency pause) feature implementation for the RewardPool and QuestEngine contracts.

## Assignment Summary

**#52 [Global] Contract circuit breaker (emergency pause)**

### What Was Implemented

1. **DataKey::IsPaused** in both contracts' storage
2. **set_pause(admin: Address, status: bool)** function for toggling pause state
3. Pause checks in payout functions:
   - `distribute_reward()` in RewardPool
   - `review_submission()` in QuestEngine
4. Admin-only access control for pause functionality
5. Admin initialization in QuestEngine (extended from original)

---

## Step-by-Step Testing Instructions

### Prerequisites

Ensure the following are installed:

- Rust (with wasm target)
- Cargo
- Soroban CLI (optional, for deployment testing)

### Phase 1: Code Compilation & Build Verification

#### Step 1.1: Build All Contracts

```bash
cd c:\Users\HomePC\Documents\D\orivex-contracts
cargo build --release
```

**Expected Result**: Build completes without errors.

**Verification Checklist**:

- [ ] No compilation errors
- [ ] No warnings related to circuit breaker implementation
- [ ] WASM binaries generated successfully

#### Step 1.2: Check Compilation Output

```bash
cargo build --release 2>&1 | grep -i "error\|warning"
```

**Expected Result**: No errors related to:

- DataKey::IsPaused
- set_pause function
- Pause checks in payout functions

---

### Phase 2: Unit Tests

#### Step 2.1: Run Reward Pool Tests

```bash
cd contracts/reward-pool
cargo test -- --nocapture
```

**Expected Test Results**:

- ✓ Initialization tests pass
- ✓ Spender management tests pass
- ✓ Reward distribution tests pass
- ✓ Pool funding tests pass

#### Step 2.2: Run Quest Engine Tests

```bash
cd contracts/quest-engine
cargo test -- --nocapture
```

**Expected Test Results**:

- ✓ Initialization tests pass (with admin parameter)
- ✓ Quest creation tests pass
- ✓ Proof submission tests pass
- ✓ Submission review tests pass
- ✓ Refund tests pass

---

### Phase 3: Circuit Breaker Functionality Tests

#### Step 3.1: Reward Pool - Set Pause Function

**Test Objective**: Verify only admin can pause/unpause the contract

```bash
cd contracts/reward-pool
cargo test test_set_pause
```

**Manual Test Logic** (if automated tests exist):

1. Initialize contract with admin = Alice, token = USDC
2. Call `set_pause(alice, true)` → Should succeed
3. Verify pause status is true in storage
4. Call `set_pause(alice, false)` → Should succeed
5. Verify pause status is false in storage
6. Call `set_pause(bob, true)` where bob ≠ admin → Should panic with "Unauthorized"

**Expected Results**: ✓ Pass

#### Step 3.2: Reward Pool - Pause Check in distribute_reward

**Test Objective**: Verify payouts fail when paused

```bash
cd contracts/reward-pool
cargo test test_distribute_reward_when_paused
```

**Manual Test Logic**:

1. Initialize RewardPool with admin and token
2. Add a spender (e.g., QuestEngine)
3. Fund the pool with tokens
4. Pause the contract: `set_pause(admin, true)`
5. Attempt to distribute reward: `distribute_reward(spender, learner, 100)`
6. Expected: Panic with "Contract is paused"

**Verification**:

- [ ] Contract panics with correct error message
- [ ] No tokens are transferred
- [ ] Event is not emitted

#### Step 3.3: Reward Pool - Payouts Work When Unpaused

**Test Objective**: Verify payouts succeed after unpausing

```bash
cd contracts/reward-pool
cargo test test_distribute_reward_after_unpause
```

**Manual Test Logic**:

1. Initialize and fund pool
2. Add spender
3. Pause contract: `set_pause(admin, true)`
4. Unpause contract: `set_pause(admin, false)`
5. Attempt to distribute reward: `distribute_reward(spender, learner, 100)`
6. Expected: Success, tokens transferred, event emitted

**Verification**:

- [ ] Distribution succeeds
- [ ] Correct amount transferred to learner
- [ ] RewardDistributed event emitted

#### Step 3.4: Quest Engine - Set Pause Function

**Test Objective**: Verify admin control of pause in QuestEngine

```bash
cd contracts/quest-engine
cargo test test_set_pause
```

**Manual Test Logic**:

1. Initialize with admin, token, reward_pool
2. Call `set_pause(admin, true)` → Should succeed
3. Call `set_pause(admin, false)` → Should succeed
4. Call `set_pause(non_admin, true)` → Should panic

**Expected Results**: ✓ Pass

#### Step 3.5: Quest Engine - Pause Check in review_submission

**Test Objective**: Verify submission reviews fail when paused

```bash
cd contracts/quest-engine
cargo test test_review_submission_when_paused
```

**Manual Test Logic**:

1. Initialize QuestEngine
2. Create a build quest (employer deposits tokens)
3. Submit proof (learner submits proof_hash)
4. Pause contract: `set_pause(admin, true)`
5. Attempt to review: `review_submission(employer, learner, quest_id, true)`
6. Expected: Panic with "Contract is paused"

**Verification**:

- [ ] Review fails with correct error message
- [ ] No tokens transferred
- [ ] Submission status remains Pending
- [ ] Event is not emitted

#### Step 3.6: Quest Engine - Reviews Work When Unpaused

**Test Objective**: Verify submission reviews succeed after unpausing

```bash
cd contracts/quest-engine
cargo test test_review_submission_after_unpause
```

**Manual Test Logic**:

1. Setup quest and submission
2. Pause: `set_pause(admin, true)`
3. Unpause: `set_pause(admin, false)`
4. Review and approve: `review_submission(employer, learner, quest_id, true)`
5. Expected: Success, tokens transferred, event emitted

**Verification**:

- [ ] Review succeeds
- [ ] Learner receives tokens (85% after 15% fee)
- [ ] Reward pool receives fee (15%)
- [ ] SubmissionReviewed event emitted

---

### Phase 4: Edge Cases & Security Tests

#### Step 4.1: Emergency Pause Scenario

**Test Objective**: Simulate emergency vulnerability response

```bash
Scenario: Vulnerability in reward calculation detected
1. Pause active during investigation: set_pause(admin, true)
2. Verify all payouts blocked
3. Investigate and fix
4. Unpause for resumption: set_pause(admin, false)
```

**Verification**:

- [ ] No payouts leak funds during investigation
- [ ] Admin can pause/unpause multiple times
- [ ] System recovers completely after unpause

#### Step 4.2: Unauthorized Pause Attempt

**Test Objective**: Verify non-admin cannot pause

```bash
Test with multiple non-admin addresses:
- Random address → Should panic "Unauthorized"
- Contract owner (if different) → Should panic "Unauthorized"
- Previous admin (after admin change) → Should panic "Unauthorized"
```

**Verification**:

- [ ] All unauthorized attempts fail
- [ ] Pause status unchanged
- [ ] Error messages consistent

#### Step 4.3: Pause State Persistence

**Test Objective**: Verify pause state persists across function calls

```bash
1. Set pause to true
2. Call other functions (create_quest, distribute_reward)
3. Verify pause is still true
4. Set pause to false
5. Verify pause is now false
```

**Verification**:

- [ ] Pause state persists correctly
- [ ] No accidental state resets

---

### Phase 5: Integration Tests

#### Step 5.1: Full Workflow with Pause

**Test Objective**: Complete flow with circuit breaker toggling

```bash
Scenario: Complete quest workflow with emergency pause
1. Initialize QuestEngine & RewardPool
2. Create build quest (employer funds)
3. Submit proof (learner submits)
4. [PAUSE TRIGGERED] set_pause(admin, true)
5. Attempt review → FAILS (paused)
6. [EMERGENCY RESOLVED] set_pause(admin, false)
7. Review and approve → SUCCEEDS
8. Verify tokens distributed correctly
```

**Verification**:

- [ ] Pause blocks submission at critical point
- [ ] Resume allows normal operation
- [ ] No data corruption
- [ ] All events emitted correctly

---

### Phase 6: Test Execution Summary

#### Run All Tests

```bash
# Reward Pool tests
cd contracts/reward-pool
cargo test

# Quest Engine tests
cd contracts/quest-engine
cargo test

# Workspace tests
cd ../..
cargo test --workspace
```

**Expected Result**: All tests pass, including circuit breaker tests

---

## Acceptance Criteria Verification

### ✓ Requirement 1: All payouts fail instantly when is_paused is true

**Test**: Phase 3, Steps 3.2 and 3.5

- [ ] distribute_reward panics with "Contract is paused"
- [ ] review_submission panics with "Contract is paused"
- [ ] No state changes occur during failed calls

### ✓ Requirement 2: Only official Admin can toggle the pause

**Test**: Phase 3, Steps 3.1 and 3.4, and Phase 4, Step 4.2

- [ ] Admin can call set_pause successfully
- [ ] Non-admin calls panic with "Unauthorized"
- [ ] Admin authentication required (require_auth)

---

## Files Modified

### RewardPool

- **contracts/reward-pool/src/types.rs**: Added `IsPaused` to DataKey enum
- **contracts/reward-pool/src/lib.rs**: Added `set_pause()` function and pause check in `distribute_reward()`

### QuestEngine

- **contracts/quest-engine/src/types.rs**: Added `Admin` and `IsPaused` to DataKey enum
- **contracts/quest-engine/src/lib.rs**: Modified `initialize()` to accept admin, added `set_pause()` function, and pause check in `review_submission()`

---

## Branch & PR Information

- **Branch**: `feat/circuit-breaker`
- **PR Title**: `feat(global): add admin-controlled circuit breaker to payout contracts`
- **Depends on**: Admin initialization in RewardPool and QuestEngine

---

## Troubleshooting

### Build Fails

1. Ensure Rust toolchain is up to date: `rustup update`
2. Clean build: `cargo clean && cargo build --release`
3. Check WASM target: `rustup target add wasm32-unknown-unknown`

### Tests Fail

1. Check error messages for specific DataKey issues
2. Verify admin addresses match between initialization and pause calls
3. Ensure mock_all_auths() is set in test setup for authentication

### Runtime Errors

1. "Not initialized" error: Initialize contract before calling set_pause
2. "Unauthorized" error: Ensure using correct admin address
3. "Contract is paused" error: This is expected when paused (in negative tests)

---

## Completion Checklist

- [ ] Code compiles without errors
- [ ] All unit tests pass
- [ ] Circuit breaker functionality tests pass
- [ ] Edge case tests pass
- [ ] Integration tests pass
- [ ] Emergency pause scenario verified
- [ ] Unauthorized pause attempts blocked
- [ ] Pause state persists correctly
- [ ] Documentation complete
- [ ] Ready for PR submission on feat/circuit-breaker branch
