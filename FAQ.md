# Orivex Contracts FAQ

This document answers frequently asked questions about the Orivex Contracts project, organized into three sections for easy navigation.

## General Questions

### What is Orivex?

Orivex is a learning-and-rewards protocol built on Soroban (Stellar) that enables soulbound badges, USDC rewards, quest management, and decentralized governance. The protocol allows learners to earn soulbound badges upon course completion, receive USDC rewards for completing courses and quests, and participate in decentralized governance.

### What blockchain does Orivex use?

Orivex uses Soroban, a smart contract platform built on the Stellar blockchain. Stellar is a decentralized, open-source blockchain network that enables fast, low-cost cross-border transactions and asset issuance.

### What is a soulbound badge?

A soulbound badge is a type of non-fungible token (NFT) that is permanently bound to a specific account and cannot be transferred, sold, or traded to another account. In Orivex, soulbound badges are issued to learners upon completion of courses to represent their achievements.

### How do I earn USDC rewards?

You can earn USDC rewards by completing courses and quests on the Orivex platform. When you complete a course or quest, the approved spender (like a course instructor or quest employer) will initiate a reward distribution from the reward pool.

### How do I participate in governance?

You can participate in governance by holding soulbound badges. Orivex uses a badge-weighted voting system, meaning that your voting power is proportional to the number of soulbound badges you hold. You can create proposals, vote on proposals, and execute proposals using the governance contract.

---

## Developer & Integration FAQ

### What programming language are Orivex Contracts written in?

Orivex Contracts are written in Rust, a systems programming language known for its safety, performance, and memory safety.

### How do I build and test Orivex Contracts?

To build and test Orivex Contracts, you can use the following commands:

```bash
cd contracts
cargo build --target wasm32-unknown-unknown --release
stellar contract build
cargo test
```

### How do I integrate Orivex Contracts into my application?

You can integrate Orivex Contracts into your application using the Soroban SDK. We also provide example integrations and tutorials in our documentation.

### Where can I find the API documentation for Orivex Contracts?

The API documentation for Orivex Contracts is available in our documentation. You can also find inline code documentation in the source code.

### How do I contribute to Orivex Contracts?

You can contribute to Orivex Contracts by:

1. Forking the repository on GitHub
2. Creating a new branch for your changes
3. Making your changes and committing them
4. Submitting a pull request

We welcome contributions of all kinds, including bug fixes, feature requests, documentation improvements, and more. Please see our CONTRIBUTING.md file for more information.

### What is the test coverage goal for Orivex Contracts?

Our goal is to achieve 90%+ test coverage across all contracts. We use both unit tests and integration tests to ensure the contracts are reliable and secure.

---

## Security & Governance

### How secure are Orivex Contracts?

Orivex Contracts are designed with security in mind. We conduct internal security audits, address all critical and high-severity issues, implement formal verification for critical functions, and plan to conduct external security audits by reputable firms. We also have a bug bounty program to encourage community members to report security vulnerabilities.

### What should I do if I find a security vulnerability?

If you find a security vulnerability, please report it to us immediately. You can report security vulnerabilities by following the instructions in our SECURITY.md file. We take security vulnerabilities seriously and will respond promptly to all reports.

### How does the governance system work?

The governance system uses a badge-weighted voting system, meaning that your voting power is proportional to the number of soulbound badges you hold. You can create proposals, vote on proposals, and execute proposals using the governance contract.

### How are proposals created and voted on?

To create a proposal, you can use the governance contract. Once a proposal is created, community members can vote on it during the voting period. After the voting period ends, the proposal can be executed if it meets the quorum and approval requirements.

### What is the quorum and approval requirement for proposals?

The quorum and approval requirement for proposals is defined in the governance contract. You can find the exact requirements in the contract's documentation.

---

## Additional Resources

- [README.md](README.md) - Project overview and getting started guide
- [CONTRIBUTING.md](CONTRIBUTING.md) - Guidelines for contributing to the project
- [SECURITY.md](SECURITY.md) - Security policy and vulnerability reporting process
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) - Community code of conduct
- [ROADMAP.md](ROADMAP.md) - Project roadmap and milestones
- [GLOSSARY.md](GLOSSARY.md) - Glossary of key terms and concepts

If you have any other questions, please feel free to open an issue on GitHub or join our community Discord server.
