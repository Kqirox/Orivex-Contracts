# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial project structure with core contract modules
- `badge-nft` contract with soulbound badge issuance, retrieval, and revocation
- `course-registry` contract with course CRUD, learner progress, soulbound badge mint, and USDC payout triggering
- `reward-pool` contract with USDC funding, distribution, approved-spender gate, and emergency sweep
- `stake-vault` contract with token staking, lockup, and multiplier accessor
- `governance` contract with badge-weighted proposal lifecycle
- `quest-engine` contract with build and explore quests, submissions, batch review, and refunds
- Comprehensive unit tests for all contracts
- Integration tests for cross-contract interactions
- CI/CD pipeline for automated testing and deployment
- Initial documentation including README.md, CONTRIBUTING.md, and SECURITY.md

### Changed

- N/A

### Deprecated

- N/A

### Removed

- N/A

### Fixed

- N/A

### Security

- N/A

---

## [0.1.0] - 2026-07-22

### Added

- Initial repository setup with core contract modules
- Initial project structure and build system
- Initial test suite for all contracts
- Initial documentation

---

## How to Contribute to the Changelog

When contributing to this repository, please first discuss the change you wish to make via issue, email, or any other method with the owners of this repository before making a change. Please update the CHANGELOG.md with details of changes to the included.

The CHANGELOG.md should contain an entry for each release, including:

- The date of release
- The version number
- A list of changes in the following categories:
  - Added for new features
  - Changed for changes in existing functionality
  - Deprecated for soon-to-be removed features
  - Removed for now removed features
  - Fixed for any bug fixes
  - Security in case of vulnerabilities

[Unreleased]: https://github.com/[INSERT GITHUB REPOSITORY OWNER/NAME HERE]/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/[INSERT GITHUB REPOSITORY OWNER/NAME HERE]/releases/tag/v0.1.0
