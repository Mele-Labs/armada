# A job that hit the plan's limit

**Part of:** [Job](../session.md).

**A job the plan's limit stopped is woken when the limit resets, and nobody has to poke it.** A turn that ends on `error: rate_limit` is reported by the mod as `limited`, with the instant the full window opens again: the latest reset among the windows at or near full, so a weekly window spent beside a five-hour one is waited out.
A limit no window shows full leaves nothing to wait for, and is not reported.
Fleet keeps the instant in `session_limit`, one row a job, and serves it as `SessionRecord.held_until` for Bridge to draw as `battery-low` reading "Limit reached · resumes 6:40 PM" (`hourglass` is banned; `packages/icons/icons/battery-low.toml`).

A minute after `held_until`, a tick of the notice loop sends the job `Continue` through `told_by_fleet`, the word Fleet already has for its own messages, and lets the hold go.
The row is persisted, so a Fleet that restarts meanwhile still wakes it.

| Job | The wake |
|---|---|
| Hosted | A turn on its process, started again if it was ended for being quiet. The thread draws `Continue` as Fleet's |
| Terminal | Held for its mod as `Fleet: Continue` and submitted when the mod next asks. A mod that is not asking is tried again each tick, and given up on 30 minutes past the wake |
| Ended | Nothing is sent, and the hold goes |

**A job that took a turn in the meantime is not woken.** The hold goes with a finished turn, with a message from the person, with a hosted turn starting and with the job ending.
The turn the limit ended is not a finished one: the mod does not report a turn that died on an API error as `turn_completed`.

**The mod keeps no timer of its own.** A terminal job is woken by Fleet through the path it already asks on; a mod timer would wake it again after a Fleet restart that had already done so.
