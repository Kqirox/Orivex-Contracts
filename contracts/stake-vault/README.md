# Stake Vault Contract

Token staking, lock, and multiplier accessor.

## Functions

- `initialize(admin, token)` — one-time deploy-time setup.
- `stake(user, amount)` — locks tokens and resets the lock timestamp.
- `unstake(user)` — releases funds after the lock period elapsed.
- `get_multiplier(user) -> MultiplierBps` — returns the stake-tier multiplier for `user` as a typed `MultiplierBps` enum value.
- `upgrade_contract(admin, new_wasm_hash)` — admin-only WASM upgrade.

## MultiplierBps enum

`get_multiplier` returns a `MultiplierBps` enum (defined in `stake_vault::types`) instead of a raw `u32`. This makes the basis-points convention explicit and prevents silent payout bugs in cross-contract callers.

| Variant              | Staked tokens | BPS | Effective multiplier |
|----------------------|---------------|-----|----------------------|
| `MultiplierBps::None` | < 100        | 100 | 1.0×                 |
| `MultiplierBps::Low`  | 100 – 499    | 120 | 1.2×                 |
| `MultiplierBps::High` | ≥ 500        | 200 | 2.0×                 |

### Basis-points convention

All variants are expressed in **basis points** (hundredths of 1×). Cross-contract callers must use `MultiplierBps::as_bps()` to obtain the numeric value for arithmetic, then divide by 100:

```rust
let multiplier: MultiplierBps = stake_vault_client.get_multiplier(&learner);
let boosted_amount = (base_amount * multiplier.as_bps() as i128) / 100;
```

Using the raw enum discriminant (0 / 1 / 2) instead of `as_bps()` will produce incorrect results.

## Staking tiers

Staking thresholds are defined as constants in `stake_vault::lib`:

| Constant                  | Value | Meaning                         |
|---------------------------|-------|---------------------------------|
| `TIER_LOW_STAKE_BOUND`    | 100   | Minimum stake for `Low` tier    |
| `TIER_HIGH_STAKE_BOUND`   | 500   | Minimum stake for `High` tier   |
| `DEFAULT_LOCK_PERIOD_SECONDS` | 604800 | Lock window (7 days)        |
