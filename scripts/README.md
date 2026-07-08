# Orivex — `scripts/` directory

The Python scripts in this directory are **historical rewrite helpers** from a
one-time project rebrand. They are not part of the production contracts and
should not be run as part of normal development.

| Script | Purpose | Status |
|---|---|---|
| `orivex_rewrite.py`               | Rewrote the git history of this repository to substitute every `Learnault` / `learnault-contracts` occurrence with the corresponding `Orivex` / `orivex-contracts` form, and re-authored every commit to a single identity. | **Destructive — already executed.** Do not re-run; it would further mutate history. |
| `orivex_polish_commits.py`        | Phase 1 of a polish phase — added ~52 substantively-distinct commits on top of the rewritten history (doc paragraphs above `pub fn`, `pub const` extractions, new top-level Markdown files, per-contract `README.md` files). | **Idempotent — re-running is a no-op** because each operation's sentinel marker is now present in the target source. |
| `orivex_polish_commits_phase2.py` | Phase 2 of the polish phase — added ~53 more commits (additional fn docs, more constants, additional top-level MD files, module-doc expansion continuations). | **Idempotent — re-running is a no-op.** |
| `orivex_apply_change.py`          | Helper invoked by the polish scripts to apply per-commit file edits; not used standalone. | **Idempotent — no-op if no pending operations remain.** |

## Pre-rewrite attribution

Original authorship of every commit BEFORE the destructive rewrite is
preserved in [`CONTRIBUTORS_OUTSIDE_REPO.md`](../CONTRIBUTORS_OUTSIDE_REPO.md)
at the repository root, and as a bundle backup of every ref in
`/tmp/orivex-backup/full-history.bundle` (only on the local build host).
Treat `CONTRIBUTORS_OUTSIDE_REPO.md` as the canonical record of the original
authors of work on this codebase; git commit metadata alone is no longer
sufficient to reconstruct them.

## How to verify the final state

If you have the Soroban / Rust toolchain installed locally, you can verify
the post-rewrite state with:

```bash
cd contracts
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
stellar contract build
```

These mirror the steps in `.github/workflows/ci.yml`. The polish-phase
commits intentionally omit intermediate-state `cargo fmt --check`
conformance — only the final HEAD must satisfy the workflow. To produce a
clean working tree, run `cargo fmt --all` once on HEAD and commit the
resulting formatting changes.
