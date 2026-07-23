# Soroban Project

## Project Structure

This repository uses the recommended structure for a Soroban project:

```text
.
├── contracts
│   └── course_registry
│       ├── src
│       │   ├── lib.rs
│       │   └── test.rs
│       └── Cargo.toml
├── Cargo.toml
└── README.md
```

## Two-Step Access Control (Issue #20)

Every contract exposes a **two-step** flow for rotating each admin and
wiring address:

1. `propose_new_<role>(current_admin, proposed)` — admin-only.
2. `accept_<role>(acceptor)` — only the proposed address.
3. `cancel_<role>(caller)` — current admin OR proposed address.

The shared types (`PendingTransfer`) and events
(`TransferProposed` / `TransferAccepted` / `TransferCancelled`) live in
`contracts/common::two_step`. The timelock is **soft**: the proposed
address may accept immediately. Off-chain monitors are expected to alert
on `TransferProposed` events so communities can react before acceptance.
