# Should Fleet free a slot from a parked Job by itself?

**Decided 2026-10-05.**

Jobs that were stopped or parked held worktree slots and the work behind them could not start. He said they "are holding onto worktree slots ... the work can't start because the stopped jobs are holding the leases."

**Chosen: Pause and Resume, plus auto-release.** When other work is waiting for a slot and the pool is full, Fleet pauses a parked Job by itself so the waiting work can start.

- **A grace window and an off switch**, asked about the risk of pausing a Job he had just opened. Fleet never pauses a Job whose last event is under fifteen minutes old, because Fleet cannot see what Bridge shows. `setup.auto_release` turns it off and `setup.auto_release_grace_minutes` sets the window.
- **The oldest parked Job first.**
- **Fleet never resumes a Job.** A Fleet-paused Job comes back when a person resumes it or acts on it.
- **A person's Resume waits for a free slot** and never forces another Job out.
- **A finished Job still holding its slot** (`completed_success`) is never taken.
- **A paused Job stays out of the merge line.**

**Where it landed:** `fleet/auto-release-parked-jobs`, slice 4. The keys sit under `setup:` beside `worktrees`, and the pass is `fleet::releasing`.

**Not changed:** nothing on the wire or in Bridge. `Paused.by` already says who paused a Job.
