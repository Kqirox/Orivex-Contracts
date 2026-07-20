# Orivex Contract Upgrade Guide

This document describes the storage-versioning scheme used across all six
Orivex contracts and the procedure for safely upgrading deployed contracts.

---

## Why versioning is necessary

Soroban's `update_current_contract_wasm` hot-swaps the WASM bytecode but does
**not** automatically migrate existing storage. If the new WASM introduces a
new required field in a stored struct (e.g. `Course`), decoding an old record
with the new ABI will produce a runtime panic. The version + migrate pattern
prevents this.

---

## Schema version constants

Every contract declares:

```rust
pub const VERSION: u32 = 1;
```

`VERSION` is a compile-time constant. Bump it — and add a migration step to
`migrate()` — whenever you make a breaking change to a stored struct or
`DataKey` variant.

**Version history per contract (current: 1)**

| Version | Meaning |
|---------|---------|
| 0 | Pre-versioning baseline. No `Version` key in storage. |
| 1 | Initial versioned schema. All structs are wire-compatible with v0. |

---

## Storage key: `DataKey::Version`

All six contracts store the current schema version under
`DataKey::Version` in **instance storage**:

```rust
env.storage().instance().set(&DataKey::Version, &VERSION);
```

Reading it:
```rust
let current: u32 = env
    .storage()
    .instance()
    .get(&DataKey::Version)
    .unwrap_or(0);  // absent == v0
```

The `unwrap_or(0)` handles contracts deployed before versioning was introduced.

---

## The `V0` struct pattern

For every struct that could change in a future migration, a companion `V0`
struct is kept in `types.rs`:

```rust
/// Current schema.
#[contracttype]
pub struct Course {
    pub instructor: Address,
    pub total_modules: u32,
    pub metadata_hash: BytesN<32>,
    pub active: bool,
    // v2 example: pub category: Symbol,
}

/// Snapshot of the v0 layout — used by migrate() to decode old records.
#[contracttype]
pub struct CourseV0 {
    pub instructor: Address,
    pub total_modules: u32,
    pub metadata_hash: BytesN<32>,
    pub active: bool,
}
```

When v1 → v2 adds a field, the `migrate()` function reads each stored record
as `CourseV0`, converts it to the new `Course`, and writes it back. The `V0`
type must remain in the codebase for the lifetime of the v2 WASM so
in-progress migrations can finish cleanly.

---

## The upgrade procedure

Upgrading a live contract is a **two-transaction** sequence. Both transactions
must be sent by the Protocol Admin.

### Transaction 1 — swap the WASM

```rust
contract.upgrade_contract(admin, new_wasm_hash);
```

- Calls `env.deployer().update_current_contract_wasm(new_wasm_hash)`.
- Emits `ContractUpgraded { admin, new_wasm_hash }`.
- The new WASM is live immediately. **Do not call any other function before
  `migrate()` if the schema has changed.**

### Transaction 2 — apply migrations

```rust
contract.migrate(admin);
```

- Reads `DataKey::Version` (defaults to 0 if absent).
- Executes each incremental migration step in order:
  ```rust
  if current_version < 1 { /* v0 → v1 steps */ }
  if current_version < 2 { /* v1 → v2 steps */ }
  // …
  ```
- Writes `VERSION` to instance storage when done.
- Panics with `"Already at current version"` if the on-chain version already
  equals `VERSION` — preventing accidental double-migration.

### Checking the on-chain version at any time

```rust
let v = contract.contract_version(); // returns 0 before first migration
```

---

## Writing a new migration step

Suppose the next release adds a required `category: Symbol` field to `Course`.

1. **Increment `VERSION`** in `course-registry/src/lib.rs`:

   ```rust
   pub const VERSION: u32 = 2;
   ```

2. **Add `CourseV1`** to `types.rs` (snapshot of the current layout) and
   update `Course` with the new field:

   ```rust
   #[contracttype]
   pub struct CourseV1 { /* old fields */ }

   #[contracttype]
   pub struct Course {
       /* old fields */
       pub category: Symbol,
   }
   ```

3. **Add the migration step** to `migrate()` in `lib.rs`:

   ```rust
   if current_version < 2 {
       let count: u32 = env.storage().instance()
           .get(&DataKey::CourseCount).unwrap_or(0);
       for id in 1..=count {
           if let Some(old): Option<CourseV1> = env.storage()
               .persistent().get(&DataKey::Course(id))
           {
               let new_course = Course {
                   instructor:     old.instructor,
                   total_modules:  old.total_modules,
                   metadata_hash:  old.metadata_hash,
                   active:         old.active,
                   category:       Symbol::new(&env, "general"),
               };
               env.storage().persistent()
                   .set(&DataKey::Course(id), &new_course);
           }
       }
   }
   ```

4. **Write and run migration tests** covering:
   - Records written at v1 are readable at v2.
   - `contract_version()` returns 2 after `migrate()`.
   - Calling `migrate()` twice panics.
   - Non-admin cannot call `migrate()`.

---

## `upgrade_contract` vs `migrate` — separation of concerns

| Function | What it does | When to call |
|---|---|---|
| `upgrade_contract` | Swaps the WASM bytecode on-chain | Once, to deploy a new build |
| `migrate` | Transforms existing storage to the new schema | Once, immediately after `upgrade_contract` |

These are separate functions because:
- `update_current_contract_wasm` succeeds even if the storage migration would
  fail. Keeping them separate lets you inspect the new WASM, verify the hash,
  and then separately trigger the potentially expensive migration.
- A migration that iterates over thousands of records may approach gas limits
  and need to be split across multiple `migrate` calls in future designs
  (batch migration). Keeping it separate makes that refactor safe.

---

## Version mismatch abort

`migrate()` rejects calls where the on-chain version is already at `VERSION`:

```rust
assert!(current_version < VERSION, "Already at current version");
```

It does **not** panic if `current_version > VERSION` (e.g., an accidental
rollback to an older WASM). Operators should monitor `contract_version()` via
an indexer before and after every upgrade.

---

## Contracts covered

| Contract | `VERSION` const | `migrate()` | `contract_version()` | `V0` snapshot types |
|---|---|---|---|---|
| `course-registry` | ✓ | ✓ | ✓ | `CourseV0` |
| `badge-nft` | ✓ | ✓ | ✓ | `BadgeV0` |
| `reward-pool` | ✓ | ✓ | ✓ | _(no mutable structs)_ |
| `stake-vault` | ✓ | ✓ | ✓ | `StakeInfoV0` |
| `governance` | ✓ | ✓ | ✓ | `ProposalV0` |
| `quest-engine` | ✓ | ✓ | ✓ | `QuestV0` |

---

## References

- [Soroban contract upgrade guide](https://soroban.stellar.org/docs/fundamentals-and-concepts/contract-upgrades)
- Issue [#35](https://github.com/orivex/orivex-contracts/issues/35) — Add storage versioning and migrations to all contracts
