# Security Policy

## Reporting Vulnerabilities

If you discover a potential security vulnerability within Orivex smart contracts or dependencies, please do NOT open a public issue.

Report security vulnerabilities via private disclosure to the Orivex maintainers.

## Dependency Security & CI Audits

Orivex enforces automated supply-chain dependency audits in CI:
- **`cargo-audit`**: Checks dependencies against the RustSec vulnerability database.
- **`cargo-deny`**: Enforces license compliance, source integrity, and advisory bans.

For detailed security policy and local audit execution steps, see [docs/security/audit.md](docs/security/audit.md).
We take the security of this project seriously. This document outlines our vulnerability disclosure process, scope, and SLAs.

## Reporting a Vulnerability

If you discover a security vulnerability, please do **NOT** open a public issue.

Instead, please report it via one of the following channels:
- **GitHub Security Advisory**: Navigate to the "Security" tab in this repository, select "Advisories", and click "Report a vulnerability".
- **Email**: Send a PGP-encrypted email to the maintainers (if an email is provided in the repository profile).

### SLAs and Disclosure Timeline

- **First Response SLA**: We aim to acknowledge receipt of your vulnerability report within **72 hours**.
- **Disclosure Timeline**: We default to a **90-day coordinated disclosure** timeline. The vulnerability will be disclosed publicly after 90 days or when a patch is released, whichever comes first.

## Scope

**In-Scope:**
- Smart contracts and core logic (e.g., contracts in the `contracts/` directory).
- Access control and authorization mechanisms.
- Cryptographic implementations.

**Out-of-Scope:**
- Frontend UI/UX bugs that do not lead to loss of funds or unauthorized access.
- Third-party dependencies (these should be reported directly to upstream maintainers).
- Issues requiring physical access or social engineering (e.g., phishing).
- Informational findings without a clear, exploitable impact.

## Patch Process

When a valid vulnerability is reported, we follow this process:
1. **Triage & Embargo**: The report is verified and kept confidential.
2. **Private Patch**: A fix is developed in a private fork via the GitHub Security Advisory.
3. **Review**: The fix is reviewed by maintainers and the reporter (if they wish to participate).
4. **Release & Disclosure**: The patch is merged and released. The security advisory is then published to inform users.

## Supported Versions

Currently, only the latest version of `main` is supported with security updates.
