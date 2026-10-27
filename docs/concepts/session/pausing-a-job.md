# Pausing a Job

**Part of:** [Job](../session.md).

**A pause gives the slot back and keeps the Job.** `pause_session` (`POST /sessions/pause`, 23.92) ends the running turn as `stop_session` does, commits what is in the Job's slot to the branch the checkout is on as a `WIP:` commit (nothing is pushed; a checkout on no branch or on the base gets a `kept/` branch), and frees the slot for the pool.
The row stays live in the list and carries `hosted.paused` (`at`, `resuming`).
It is not a state: `state` is `live` and `hosted.turn` is `idle`.
The ledger's `slot` row is given back and its `branch` row stays, since the Job still owns the branch.
It is the Job pause (`job.md`, *Pausing a Job*) for a Job, and it reuses the same park.

**A resume takes a slot at the branch tip.** `resume_session` (`POST /sessions/resume`) leases the branch into a free slot and clears the marker.
It is the person's press alone: `agent_access` is `No`, so Helm cannot call it.
Once the Job holds its slot again it carries on (below).
**It never forces another holder out**: where every slot is held, or Jobs already wait, the Job joins the repository's line and the row reads `hosted.paused.resuming` until the first slot to free is leased to it.
A pause while it waits calls the wait off.

**Only a person's Resume lifts a pause, and nothing else wakes the Job.** Its process is not started by any message, its lease is not taken by any call, and Fleet's notices answer "not told" so the watch, limit, pull request and batched notices ask again after Resume.
A person's message, Helm's note and another Job's word are written to the thread at once and kept, in the order they came, in the table the start card keeps its own turns in (`held_later_turns`).
A message may land while the Resume waits for a slot; it is kept the same way.
**When the Job holds its slot again, at the Resume or when the line grants one,** the kept turns go as its next turns.
With none kept, Fleet tells it "You were paused and have been resumed. Carry on with what you were doing."
A paused Job raises no item or call on Needs you and stands idle; what it still owes comes back after Resume.

**A pause halts the Job's work, not only its agent.** Before the turn is ended and the slot parked, each Check the Job has running is stopped as a person's force stop stops it (`SessionCheckStop::Force`, heard by the run's `armada check` at its next chunk), and the pause waits up to three seconds for each to end; one that does not answer ends failed with "the Job was paused".
Its watches stay live but wait: none starts a run while the Job is paused, a command one has running is ended, and a reading that lands after the pause is dropped.
They run again on the first sweep after Resume, so none reads the main checkout for a parked branch.
A watch's notice waits like any Fleet notice and is asked again after Resume.
A watch's give-up time still runs while it waits.

| Refused | Code |
|---|---|
| Ended, a Helm, or holding no slot | 409 `fleet.session_not_pausable` |
| Paused already | 409 `fleet.session_already_paused` |
| Not paused (resume) | 409 `fleet.session_not_paused` |
| The pool or git will not park the work or seat the branch | 409 `fleet.session_pause_refused` |
| A terminal Job | 422 `fleet.session_command_refused` |

**What it does not do.** Only the slot the ledger records for the Job is parked; one a subagent of the Job leased stays held.
The marker is kept in `hosted_sessions` (`paused_at`, `paused_resuming`), so it survives a Fleet restart; a restart forgets the line, and the Job's next message joins it again.
