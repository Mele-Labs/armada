# A Job's quick replies: `get_session_quick`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 31 Oct 2026. What a Job's quick bar is offered beyond the lines he saved: up to three answers a small model writes to the agent's last message, and up to six lines he sends often across every Job.
**Additive only**: one operation, one route and one type; no event kind, no field on an existing type, nothing kept on disk.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/session-quick.ts`, written by hand, field for field, and `readSessionQuick` on the BridgeApi.

| Where | Side | Carries |
| --- | --- | --- |
| `get_session_quick` (`GET /sessions/quick?session_id=`, `Bridge only`) | Bridge reads | `SessionQuick` `{ asked, learnt }`, two lists of lines, either possibly empty |

**`asked`** is written on `models.proposer`, which ships as haiku, through the proposer's own client and budget: one call with no tools and no directory, waited on for twenty seconds.
It is asked once for each agent message, on the first read after it, and kept until Fleet restarts, so a read after a failure or a timeout is the empty list and is not asked again.
Bridge's main waits `QUICK_MS`, thirty seconds, so Fleet's answer arrives first.

**`learnt`** reads his own sent lines, one line of sixty characters or fewer, out of every hosted Job's thread.
Lines are alike when equal after lowercasing, trimming, collapsing whitespace and dropping trailing punctuation; the wording shown is the latest.
A line is offered once sent twice, ranked by count weighted by recency with a fourteen-day half-life, and ranked higher while the agent's last message asks something when he has sent it before right after one that did.
Fleet reads the threads at most once a minute.

**Fleet reads four fields of `thread.look`**: `learn`, `quickOff`, `quickOffFor`, and the lines of `quick` and `forgotten`, which are left out of `learnt`.
Where the bar is hidden for the Job both lists are empty and no model is asked; with `learn` false `learnt` is empty and no thread is read.
A chip he presses is sent as a message, so it is a line he sent and counts the next time. Nothing else is stored.

| Refusal | Status | What a person reads |
| --- | --- | --- |
| `fleet.no_such_session` | 422 | No Job Fleet hosts has that id |

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** Untouched: it is request and answer, so the unmeasured sink risk is neither better nor worse. Bridge asks again when the Job's last message changes, not on an event.
