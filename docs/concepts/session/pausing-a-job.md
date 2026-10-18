# Pausing a Job

**Part of:** [Job](../session.md).

**A pause gives the slot back and keeps the Job.** `pause_session` (`POST /sessions/pause`, 23.92) ends the running turn as `stop_session` does, commits what is in the Job's slot to the branch the checkout is on as a `WIP:` commit (nothing is pushed; a checkout on no branch or on the base gets a `kept/` branch), and frees the slot for the pool. The row stays live in the list and carries `hosted.paused` (`at`, `resuming`). It is not a state: `state` is `live` and `hosted.turn` is `idle`. The ledger's `slot` row is given back and its `branch` row stays, since the Job still owns the branch. It is the Job pause (`job.md`, *Pausing a Job*) for a Job, and it reuses the same park.

**A resume takes a slot at the branch tip.** `resume_session` (`POST /sessions/resume`) leases the branch into a free slot and clears the marker. It starts no turn. **It never forces another holder out**: where every slot is held, or Jobs already wait, the Job joins the repository's line and the row reads `hosted.paused.resuming` until the first slot to free is leased to it. A pause while it waits calls the wait off.

**A message to a paused Job resumes it**, as a stopped Job's next message resumes its conversation: the lease is taken before the process starts, and a full pool leaves the Job paused and waiting.

| Refused | Code |
|---|---|
| Ended, a Helm, or holding no slot | 409 `fleet.session_not_pausable` |
| Paused already | 409 `fleet.session_already_paused` |
| Not paused (resume) | 409 `fleet.session_not_paused` |
| The pool or git will not park the work or seat the branch | 409 `fleet.session_pause_refused` |
| A terminal Job | 422 `fleet.session_command_refused` |

**What it does not do.** Only the slot the ledger records for the Job is parked; one a subagent of the Job leased stays held. The marker is kept in `hosted_sessions` (`paused_at`, `paused_resuming`), so it survives a Fleet restart; a restart forgets the line, and the Job's next message joins it again.
