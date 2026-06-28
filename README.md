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

## Test

```
cd contracts
cargo test
```
