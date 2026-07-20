# Soroban Project

## Project Structure

This repository uses the recommended structure for a Soroban project:

```text
.
├── contracts
│   └── course-registry
│       ├── src
│       │   ├── lib.rs
│       │   └── test.rs
│       └── Cargo.toml
├── Cargo.toml
└── README.md
```

## WASM Size Budget

Every Stellar contract must stay under its configured WASM size budget to keep
deploy costs low and gas-efficiency headroom high. The default budget is
**50,000 bytes** (~49 KB), well below the 64 KB protocol limit.

| Contract | Max WASM size |
|---|---|
| `course-registry` | 50 KB |
| `badge-nft`       | 50 KB |
| `reward-pool`     | 50 KB |
| `stake-vault`     | 50 KB |
| `governance`      | 50 KB |
| `quest-engine`    | 50 KB |

CI enforces these budgets on every push and pull request via
[`check-wasm-sizes.sh`](check-wasm-sizes.sh). To adjust a contract's budget,
edit the `BUDGETS` associative array at the top of that script.
