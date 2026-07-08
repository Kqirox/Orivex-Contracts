# Repository migration: `Learnault` → `Orivex`

## TL;DR

This repository's git history has been **completely rewritten**. Every existing
SHA on `main` has changed, and every commit is now authored under a single
organizational identity (`7udah <merlik787@gmail.com>`). If you have a local
clone, a fork, an open pull request, or an issue that references a pre-rewrite
SHA, **your references are now stale**.

The codebase is the same; the cryptographic identifiers are not.

## What changed

- **Brand rename.** Every occurrence of `Learnault` (and every `learnault-*`
  slug such as `learnault-contracts`, `learnault-main`, `hello-learnault`) was
  substituted with the corresponding `Orivex` / `orivex-contracts` form across
  every tracked file and every commit message (subjects and bodies). The
  GitHub repository URLs pointing at historical forks (e.g.
  `https://github.com/<owner>/learnault-contracts`) were all collapsed to the
  canonical `https://github.com/Kqirox/orivex-contracts`.
- **History rewrite.** All 162 original commits were re-authored to a single
  identity via `git filter-repo`. SHA-1 collisions between forks are therefore
  gone; every historical commit has a new SHA.
- **Polish phase added.** Approximately 105 progressive polish-phase commits
  were layered on top of the rewritten history, each adding genuine
  documentation, `pub const` extractions, module-level docs, or new top-level
  Markdown files (README, ARCHITECTURE, CONTRIBUTING, SECURITY, DEPLOYMENT,
  INTEGRATIONS, TROUBLESHOOTING, CHANGELOG, LICENSE, etc.).
- **Pre-rewrite attribution preserved.** The original 32 contributors'
  names, emails, and commit log are preserved at
  `CONTRIBUTORS_OUTSIDE_REPO.md` at the repo root, and as a bundles backup at
  `/tmp/orivex-backup/full-history.bundle` on the rebuild host. Treat
  `CONTRIBUTORS_OUTSIDE_REPO.md` as the canonical attribution record going
  forward.

## What you need to do

If you are an existing collaborator, your local clone is now out of sync:

1. **Fresh sync (recommended).** Throw away the local clone and re-clone:

   ```bash
   rm -rf Orivex-Contracts
   git clone https://github.com/Kqirox/Orivex-Contracts.git
   cd Orivex-Contracts
   ```

   Alternative: fetch and hard-reset:

   ```bash
   git fetch origin
   git reset --hard origin/main
   ```

2. **Orphans.** Any local or remote branches based on the old history are now
   orphaned. **They cannot be merged back into the new `main` without manual
   rebasing** — the SVN-/mercurial-style "automatic re-attach to rewritten
   history" feature does not exist in git.

3. **Open PRs.** Any pull request that referenced a pre-rewrite SHA will appear
   to be "from nowhere" on GitHub. Plan:

   - Close the stale PR with a short comment pointing at this migration
     notice, OR
   - Re-implement the change on top of the new `main` and open a fresh PR.
     Cherry-picking individual commits will not work because the underlying
     SHAs and authors all changed.

4. **Local commits you haven't pushed yet.** If you have un-pushed local
   commits sitting on top of `main`, they are likely invalidated by the
   rewrite at the same time. The recommended recovery is:

   ```bash
   # 1. save your uncommitted work to a stash or patch files
   git diff > /tmp/my-unpushed.patch

   # 2. drop the local changes
   git reset --hard origin/main

   # 3. apply them back on top of clean history
   git apply /tmp/my-unpushed.patch
   # resolve any merge-style conflicts by hand
   ```

## Verification

If your local `main` is correctly on the rewritten history:

```bash
# Single author:
git log --all --format='%an <%ae>' | sort -u
# Expected output is exactly:
#   7udah <merlik787@gmail.com>

# Zero "Learnault" strings in source:
grep -ri 'learnault' . --exclude-dir=.git --exclude=CONTRIBUTORS_OUTSIDE_REPO.md --exclude=scripts/orivex_rewrite.py --exclude=scripts/orivex_polish_commits*.py
# Expected output: empty

# Zero "Learnault" strings in commit messages (subjects and bodies):
git log --all --format=%B | grep -i 'learnault'
# Expected output: empty
```

## Questions

If you discover a problem the migration broke — a stale branch reference, a
half-finished cherry-pick, an unrelated workflow that depended on the
pre-rewrite SHAs — please open an issue with the `migration` label and we'll
work through it case-by-case.

— The Orivex team
