# Off-chain integrations

The protocol expects the following off-chain actors:

- **Admin backend** — calls `course-registry.complete_module` on verified quiz
  submissions, and `quest-engine.verify_explore_quest` on verified off-chain
  actions.
- **Employer portal** — calls `quest-engine.create_build_quest` and then
  `review_submission` (single) or `batch_review_submissions` (bulk).
- **Badge viewer** — listens for `BadgeMinted`/`BadgeRevoked` events from the
  BadgeNFT contract.
- **Reward accounting** — listens for `RewardDistributed` events to update
  off-chain ledgers.
