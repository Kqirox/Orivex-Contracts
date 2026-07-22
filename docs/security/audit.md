# Dependency Security & Audit Policy

## Overview

Orivex smart contracts custody user funds and execute financial logic on the Soroban/Stellar network. Supply-chain security is critical to prevent vulnerabilities, malicious code injection, or unauthorized license usage across workspace dependencies.

This document outlines the automated dependency security checks and policy enforced in CI.

---

## Tooling & Verification

### 1. `cargo-audit`
- **Purpose**: Scans `Cargo.lock` against the RustSec Advisory Database.
- **Enforcement**: Any crate with an unmitigated vulnerability marked as `deny` will cause CI build failure.
- **Local Execution**:
  ```bash
  cd contracts
  cargo audit
  ```

### 2. `cargo-deny`
- **Purpose**: Enforces license compliance, dependency bans, advisory checking, and crate source restrictions configured in `deny.toml`.
- **Policy Configuration**: Defined in `deny.toml` at workspace root.
  - **Advisories**: Vulnerabilities result in immediate build denial.
  - **Licenses**: Allowlist restricted to permissive open-source licenses (`MIT`, `Apache-2.0`, `Unicode-3.0`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `CC0-1.0`, `Zlib`, `WTFPL`).
  - **Bans**: Warns on duplicate crate versions; disallows unverified crate origins.
- **Local Execution**:
  ```bash
  cd contracts
  cargo deny check
  ```

---

## Continuous Integration Trigger Schedule

The security audit workflow (`.github/workflows/audit.yml`) executes under the following conditions:
1. **Push**: Every push targeting `main` or `develop`.
2. **Pull Requests**: Every open, synchronized, or reopened PR targeting `main` or `develop`.
3. **Scheduled Scan**: Weekly automated cron job (`0 0 * * 0`) to catch newly disclosed vulnerabilities in existing dependencies.

---

## Advisory Waivers & Exception Process

If an advisory warning is a false positive or does not impact contract safety (e.g. build-only tool dependency), temporary waivers must be explicitly recorded in `deny.toml` under `[advisories.ignore]` with a clear rationale, issue reference, and expiry plan.
