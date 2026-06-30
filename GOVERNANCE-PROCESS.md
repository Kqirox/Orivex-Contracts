# Governance process

Proposals are created off-chain, encoded as `BytesN<32>` metadata hashes, then
submitted to the on-chain `Governance` contract. Voting runs for the configured
period (default one week, see `DEFAULT_VOTING_PERIOD_SECONDS`). After the
period ends, proposer or admin can call `execute_proposal` for proposals with
strictly more `votes_for` than `votes_against`.
