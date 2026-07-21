# Security Policy

## Reporting Vulnerabilities

If you discover a potential security vulnerability within Orivex smart contracts or dependencies, please do NOT open a public issue.

Report security vulnerabilities via private disclosure to the Orivex maintainers.

## Dependency Security & CI Audits

Orivex enforces automated supply-chain dependency audits in CI:
- **`cargo-audit`**: Checks dependencies against the RustSec vulnerability database.
- **`cargo-deny`**: Enforces license compliance, source integrity, and advisory bans.

For detailed security policy and local audit execution steps, see [docs/security/audit.md](docs/security/audit.md).
