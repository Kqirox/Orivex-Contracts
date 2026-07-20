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

## Deployment

Deploy all six contracts to a Stellar network with a single command.

### Prerequisites

- [Stellar CLI](https://developers.stellar.org/docs/smart-contracts/deployment) v23+
- `STELLAR_SECRET_KEY` — deployer secret key
- `STELLAR_TOKEN_ADDRESS` — USDC token contract address on the target network
- (Optional) `STELLAR_RPC_URL` and `STELLAR_NETWORK_PASSPHRASE` if not using defaults

### Deploy

```bash
# Standalone (local) network
./tools/deploy.sh standalone

# Testnet
STELLAR_SECRET_KEY=S... STELLAR_TOKEN_ADDRESS=C... ./tools/deploy.sh testnet

# Futurenet
STELLAR_SECRET_KEY=S... STELLAR_TOKEN_ADDRESS=C... ./tools/deploy.sh futurenet
```

The script is **idempotent** — re-running on a fully-deployed state skips
already-deployed contracts and only performs missing wiring steps.

Deployment artifacts are saved to `deployments/<network>.json`.

### Deployment order

Contracts are deployed in dependency order:

| Step | Contract | Dependencies |
|------|----------|--------------|
| 1 | `badge-nft` | — |
| 2 | `reward-pool` | — |
| 3 | `stake-vault` | — |
| 4 | `course-registry` | `badge-nft`, `reward-pool` (post-init wiring) |
| 5 | `quest-engine` | `reward-pool`, `stake-vault` (init args) |
| 6 | `governance` | `badge-nft` (init arg) |

### Status check

Query each contract and print a health report:

```bash
./tools/status.sh standalone
./tools/status.sh testnet
```

### Directory structure

```
tools/
├── Cargo.toml       # Workspace for tooling crates
├── deploy.sh        # Deploy all contracts
└── status.sh        # Health report

deployments/
├── standalone.json  # Deployment config (local)
├── testnet.json     # Deployment config (testnet)
└── futurenet.json   # Deployment config (futurenet)
```
