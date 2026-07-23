# Contributing

> **Security Note:** If you have found a security vulnerability, please do **not** open a public issue. See our [SECURITY.md](SECURITY.md) for instructions on how to securely report it.

## Pull requests

Open against the `main` branch. CI runs `cargo fmt --check`, `cargo clippy -D
warnings`, and `cargo test` from the `contracts/` subdirectory; PRs must keep
all three green.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/) form. The
scope is the crate name for in-tree changes (`feat(course-registry): …`) and
omitted for cross-cutting changes (`docs: …`, `chore: …`).
