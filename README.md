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

## Build

```
cd contracts
cargo build --target wasm32-unknown-unknown --release
stellar contract build
```

### Wasm Size Budget

CI enforces a size budget on every compiled Wasm artifact (default: **50 KB**).
The Soroban protocol limit is 64 KB; staying under 50 KB preserves deploy-cost and
gas headroom.

| Contract          | Threshold |
|-------------------|-----------|
| `course-registry` | 50 KB     |
| `badge-nft`       | 50 KB     |
| `reward-pool`     | 50 KB     |
| `stake-vault`     | 50 KB     |
| `governance`      | 50 KB     |
| `quest-engine`    | 50 KB     |

To adjust a per-contract threshold, edit the `THRESHOLDS` map in
`.github/workflows/ci.yml` under the `Check Wasm sizes` step.

To verify locally:
```bash
for wasm in contracts/target/wasm32v1-none/release/*.wasm; do
  size=$(stat -c%s "$wasm")
  echo "$(basename "$wasm"): $size bytes"
done
```

## Test

```
cd contracts
cargo test
```
