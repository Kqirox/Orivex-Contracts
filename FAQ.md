# FAQ

## Can badges be transferred?

No. Badges are soulbound and stay with the original learner's address.

## How are rewards denominated?

USDC with seven decimals of precision, configured in `reward-pool/Cargo.toml`.

## What happens to a learner's progress if a course is deactivated?

Progress records persist in storage. New module completions are blocked by the
`course.active` invariant.
