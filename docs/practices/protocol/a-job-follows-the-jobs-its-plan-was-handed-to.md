# A Job follows the Jobs its plan was handed to

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *Linked Jobs*. **Additive throughout**; the protocol ID moves with the wire files, and nothing is bumped.

| Where | Carries | Notes |
| --- | --- | --- |
| `StartAnotherSession.group` | The id of a group of the caller's recorded plan. Absent is the start of 23.80 | Fleet titles the Job with the group's title, in the caller's repository whatever `manifest_id` says, and holds the group's tasks word for word, the caller's handoff and `message` as its first message. 422 `fleet.group_unknown`, 409 `fleet.group_handed` |
| `hand_to_session` (`POST /sessions/hand_to_session`) | `HandToSession`: `session_id`, `group`. Answers `SessionHeld` | `agent_access` No: Bridge's. The same start with the first message **sent**, since the press is the person's |
| `Attachment.detail.group` on `started_by` and `started_to` | The group's id | No field is added: the existing `detail` map carries it on both ledgers, so Bridge reads a Job's group off its own record |
| `SessionRow.linked` (`session.row`, `get_session`) | `id`, `at`, `session` (`SessionLink`), `event` (`done` or `merged`; `needs` only in threads written before 19 Oct 2026) | Written in the **parent's** thread where it happened, and the parent's agent is sent a message in the same moment. Bridge draws it as the press that opens `session` |

**Every Job with a `started_by` row is followed, grouped or not**, which is also the rule for nesting it under its parent in Work.
**Each notice is told once**: `done` and `merged` are kept in the store (`session_linked_told`, an additive migration), so a restart tells nothing again.
A Job that needs the owner is not told to its parent at all, since 19 Oct 2026; the call stays on the child's own row.

**The event stream is neither better nor worse.** One `session.row` and one message to a Job's process for each notice, and a notice is rare beside a turn's rows.
The unbounded sink and the missing resync message are untouched.
