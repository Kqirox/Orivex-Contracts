# Troubleshooting

## "Caller is not an authorized spender"

The calling contract has not been whitelisted via `add_approved_spender` on
the RewardPool. Call `reward_pool.add_approved_spender(admin, caller)`.

## "Course already completed"

The learner's progress already equals `course.total_modules`. Either the
learner has already finished all modules, or there's a duplicate module.

## "Submission is not pending review"

The submission has already been approved or rejected. Batch reviews can't
re-process already-decided submissions.
