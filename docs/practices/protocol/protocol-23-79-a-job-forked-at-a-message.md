# Protocol 23.79: a Job forked at a message

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *A fork at a message*. **Additive only**: one optional field on `ForkFrom`, one 422 and one key in an existing detail. No new operation.

| Where | Carries | Notes |
| --- | --- | --- |
| `ForkFrom.at?` (`start_session` `fork`) | The id of a thread row | The copy ends at that message, included. Left out, the fork is the whole conversation and is made of an ended or dead job only, as since 23.69 |
| `start_session` with `fork.at` | | **The old job may be live** and is left as it was: no 409 `fleet.session_fork_live`. 422 `fleet.fork_point_unwritten` where no message of the thread has that id or the agent's conversation does not hold it; nothing is written |
| `Attachment.detail` of `forked_from` | `at`, `cut` | `at` is the row id as sent, which Bridge cuts the folded old rows at. `cut` is the transcript line the agent is told to end at and Bridge does not read it. Both absent for a whole-conversation fork |

**Skew.** A Fleet before 23.79 reads a `fork` with an `at` as it reads one without and refuses a live job 409, so a Bridge that sends `at` to it shows the refusal.
Bridge's half is `packages/protocol/src/hosted-sessions.ts`, written by hand like the rest; the part that offers the act is `forkSessionAt` on the preload.
