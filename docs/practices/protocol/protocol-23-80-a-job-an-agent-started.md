# Protocol 23.80: a Job an agent started

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *A Job an agent started*. **Additive**: two operations on the agent's door, one thread row, two enum values and two attachment kinds. No existing field changes.

| Where | Carries | Notes |
| --- | --- | --- |
| `start_another_session` (`POST /sessions/start_another`) | `StartAnotherSession`: `manifest_id`, `title`, `message`, `group?` (a plan group; see *A Job follows the Jobs its plan was handed to*). Answers `SessionHeld`: `session_id`, `title`, `fork` | `agent_access` Yes. The Job is created and listed at once with **no process and no turn**. 422 `fleet.agent_start_unplaced` where the connection places no hosted Job, 422 `fleet.agent_start_empty` for a blank `title` or `message` |
| `fork_this_session` (`POST /sessions/fork_this`) | `ForkThisSession`: `title`, `message`, `at?`. Answers `SessionHeld` with `fork` true | `agent_access` Yes. **Of the caller's own conversation only**: the caller is placed by its connection and no id is read. `at` is a thread row id as `fork.at` is; left out, Fleet takes the caller's latest message of its own that the transcript holds. 422 `fleet.fork_point_unwritten` as for `fork.at`; nothing is written |
| `SessionRow.started` (`session.row`, `get_session`) | `id`, `at`, `session` (`SessionLink`: `id`, `title`), `fork` | Written in the **caller's** thread. Bridge draws it as the press that opens `session` |
| `Attachment.kind` `unsent` | `target` is the first message's words, `standing` until the person's ordinary send, then `spent` | On the new Job. Bridge reads it as the text to put in the box once. Ending the Job gives it back. Kinds are open text, so no type changes |
| `Attachment.kind` `started_by`, `started_to` | `target` is the other Job's id, `spent` | A start's links. A fork keeps the `forked_from` and `forked_to` it already had |
| `WaitingSource` | `first_message` | Fleet's. Id `unsent:<session id>`, text `First message`, no options |
| `WaitingActKind` | `message` | `target` is the Job's id; a press opens it. An agent may also state it in `waiting_for` |
| `answer_waiting` | | Refused 422 `fleet.session_waiting_empty` for a `first_message` item: sending the first message is the answer |

**`send_session_message` is the one call that sends it.** What the box holds is sent, so an edit before the send changes nothing on the wire.
It starts the first turn and settles the attachment and the item together.

**Skew.** A Fleet before 23.80 has neither route, so the tools are not offered to an agent, and sends no `started` row, `first_message` item or `unsent` attachment.
A Bridge before it meets `first_message` and `message` as values it does not know.
Bridge's half is `packages/protocol/src/sessions.ts` and `hosted-sessions.ts`, written by hand like the rest.
**The event stream is unchanged**: one more `session.row` and `session.changed` per call, on the same sink.
