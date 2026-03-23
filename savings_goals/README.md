# Savings Goals Contract

## Lock/Unlock Behavior

### Idempotent Transitions
`lock_goal` and `unlock_goal` are idempotent:
- Calling `lock_goal` on an already-locked goal returns `true` with no state change and no duplicate event.
- Calling `unlock_goal` on an already-unlocked goal returns `true` with no state change and no duplicate event.
- `GoalLocked` and `GoalUnlocked` events fire **only** on real state transitions.

### Security
- Only the goal owner can lock or unlock a goal.
- Idempotent calls are recorded in the audit log as successful.
- Time-locks are not bypassed by repeated unlock calls.
