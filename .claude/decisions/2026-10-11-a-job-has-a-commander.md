# Who leads a Job when something goes wrong inside it?

**Decided 2026-10-11.**

Job 14 put ten Drones on one `implement` step, each starting cold, for $16.48
and 567 turns. One escalation sat eight hours until he raised the turn cap
from 300 to 2000 and the cost cap from $10 to $50 by hand. The plan was
undersized, about 60 files edited outside its scope, and the mid-step look
could rule the drift justified but could not change the plan. Nothing in a
Job both judges and acts for the length of the Job, so every judgement call
reached him. A Session works because he can tell its agent to fix whatever is
wrong, and a Job has no one to tell.

**Chosen: a Commander per Job.** One conversation per Job, on Helm's host
keyed by the Job, resumed by Fleet on the Job's events and by him whenever he
writes to it.

- **Fleet wakes it** on a Drone ending with nothing handed in, a group red with
  its retries spent, a cap nearly spent, drift past the plan, a Judge refusal,
  and a broken worktree or environment.
- **Without asking him it may** raise the turn and cost caps up to a ceiling,
  change the plan (split, add, drop or regroup tasks), restart a task or
  redirect a Drone, and dispatch a repair Drone.
- **It may not** pass a gate, attest, merge, push, or edit Checks or the
  Manifest. Checks and the Judge still decide whether work advances.
- **It asks him** with one card when the fix is out of its reach.
- **Drones still do the tasks.** The Commander steers them and does not code
  the step itself.

**What it reverses, with his yes:** `docs/scope.md`'s third attempt, which
reads any design that reaches for "ask the agent" as the abandoned
orchestrator, and `docs/concepts/helm.md`'s "Fleet never wakes the session".
What attempt 3 rejected was a transcript read as a claim, and no Commander
claim advances anything.

**Cost he took:** an agent now spends on a Job between Drones, and acts on it
with nobody watching, inside the reach above.

**Where it landed:** mock first, on `armada/session-b3f4fb84`. Wiring waits on
his walk.
