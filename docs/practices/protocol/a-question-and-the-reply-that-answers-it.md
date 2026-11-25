# A question and the reply that answers it: `list_job_questions`, `mark_answer_read`, `SessionRecord.answer_unread`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 17 Nov 2026. A message of his can be sorted as a question, and Fleet links each question to the reply of the Job's that answers it, unread until he reads it.
**Additive only**: one enum value, two operations and their routes, one optional field on `SessionRecord`; no new event kind.
The protocol ID moves with `crates/ipc/src`, `crates/ipc/operations`, the route table and `classic-final:packages/protocol/src`, and there is no number to take.
The Rust half is `crates/ipc/src/story.rs`; Fleet's is `crates/fleet/src/story/questions.rs`.
Bridge's half is `answer_unread` in `classic-final:packages/protocol/src/sessions.ts`; the story calls have no TypeScript mirror, since only native Bridge reads them.
The rules are `docs/concepts/job-story.md`, *A question and its answer*.

| Where | Side | Carries |
| --- | --- | --- |
| `MessageKind::Question` | Fleet writes, Bridge writes on `set_message_kind` | `question` on the wire |
| `list_job_questions` | Bridge reads | `GET /sessions/questions?session_id=`, `JobQuestions { session_id, questions }`, the unanswered first, each group newest first |
| `JobQuestion` | Fleet writes | `row_id`, his message's; `words`; `asked_at`; `answer`, absent while open |
| `JobAnswer` | Fleet writes | `reply_row`, the agent's `message` row id; `excerpt`, about 300 characters of prose; `answered_at`; `read` |
| `mark_answer_read` | Bridge writes | `POST /sessions/question/read`, `MarkAnswerRead { session_id, row_id }`, answered with the `JobQuestion` |
| `JobAct::AnswerRead` | Fleet sends to the Mac holding the Job | `answer_read`, body `MarkAnswerRead` |
| `SessionRecord.answer_unread` | Fleet answers | `true` where an answer of the Job's waits unread. Absent where none does |

**A Job held from another Mac is marked read on that Mac.** `mark_answer_read` is passed on as the act `answer_read` (`JobAct::AnswerRead`), queued while that Mac sleeps, and its store keeps what he read.

**`reply_row` is a `SessionRow` id**, the one `get_job_thread` takes as `before` and `upto`, so a client scrolls its thread to the answer without a new lookup.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream is neither better nor worse.** `session.story` already names the Job, and is published when a question is answered or its answer read; `session.changed` goes out then too, since `answer_unread` may have moved.
Each is one event a read of the model or a press of his, far below a Drone's rate.
