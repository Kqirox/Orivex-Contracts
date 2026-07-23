# Orivex Contracts Glossary

This glossary defines key terms and concepts used in the Orivex Contracts project and the broader blockchain and learning-and-rewards domain.

## Core Project Terms

### Scaling Tier

A predefined level of resource allocation and functionality access within the Orivex protocol, designed to accommodate different usage patterns and growth stages of users and organizations. Scaling tiers typically include features like increased reward pool limits, quest creation limits, and governance voting power multipliers.

### Soulbound

A type of non-fungible token (NFT) that is permanently bound to a specific account and cannot be transferred, sold, or traded to another account. In Orivex, soulbound badges are issued to learners upon completion of courses to represent their achievements and cannot be transferred.

### Approved Spender

An account authorized by the reward pool contract to initiate reward distributions to learners. Approved spenders are typically verified entities (like course instructors or quest employers) who have been granted permission to access and distribute funds from the reward pool.

### Module Completion

The state achieved when a learner successfully finishes all required tasks and requirements for a specific module within a course. Module completion is tracked by the course registry contract and is a prerequisite for progressing to subsequent modules and ultimately completing the entire course.

## Domain & Blockchain Terms

### Soroban

A smart contract platform built on the Stellar blockchain, designed for high performance, low cost, and ease of use. Orivex Contracts are written in Rust and deployed to Soroban.

### Stellar

A decentralized, open-source blockchain network that enables fast, low-cost cross-border transactions and asset issuance. Orivex uses Stellar as its underlying blockchain infrastructure.

### USDC

A stablecoin issued by Circle that is pegged to the US dollar. Orivex uses USDC as its primary reward currency for learners who complete courses and quests.

### Non-Fungible Token (NFT)

A unique digital asset that represents ownership of a unique item or piece of content. In Orivex, NFTs are used to issue soulbound badges to learners.

### Smart Contract

A self-executing contract with the terms of the agreement between buyer and seller being directly written into lines of code. Orivex Contracts are a set of smart contracts that power the learning-and-rewards protocol.

### Governance

The process by which a decentralized community makes decisions about the protocol's future. In Orivex, governance is badge-weighted, meaning that learners with more soulbound badges have voting power proportional to the number of badges they hold.

### Quest

A task or set of tasks designed to engage learners and reward them for their participation. Orivex supports two types of quests: build quests (where learners build something) and explore quests (where learners explore and learn).

### Reward Pool

A smart contract that holds USDC funds and distributes rewards to learners who complete courses and quests. The reward pool is managed by approved spenders and can be funded by employers, course creators, and other contributors.

---

## Architecture Terms

### Course Registry

A smart contract that manages the creation, updating, and deactivation of courses, as well as tracking learner progress and module completion.

### Badge NFT

A smart contract that issues, retrieves, and revokes soulbound badges for learners who complete courses.

### Stake Vault

A smart contract that allows users to stake tokens and earn rewards, with lockup periods and multiplier accessors.

### Quest Engine

A smart contract that manages the creation, submission, review, and refund of quests, as well as batch review of submissions.

---

## How to Contribute to the Glossary

If you have suggestions for additional terms or improvements to existing definitions, please open an issue or submit a pull request on GitHub. We welcome community contributions to make this glossary as comprehensive and useful as possible.
