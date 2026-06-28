# Architecture

## Cross-contract call graph

```
CourseRegistry.complete_module
  ├─ BadgeNFT.mint_badge            (if BadgeNFT address is wired)
  └─ RewardPool.distribute_reward   (if RewardPool address is wired)

QuestEngine.review_submission
  ├─ StakeVault.get_multiplier      (basis points -> payout boost)
  └─ token.Contract.transfer        (splits 85/15 learner/reward-pool)

QuestEngine.verify_explore_quest
  └─ RewardPool.distribute_reward
```
