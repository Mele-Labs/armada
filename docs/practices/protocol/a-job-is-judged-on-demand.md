# A Job is judged on demand

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 31 Oct 2026.
**One Judge replaces two things**: the old Session judge (`judge_session`, `send_concern`, `dismiss_concern`, concerns in `waiting_for`) and the plan's adversarial review (`start_adversarial_review`, `send_finding`, `dismiss_finding`, `SessionPlan.adversarial`).
Pre-alpha, so both are removed outright and nothing reads the old shapes.
The protocol ID is derived from the wire files and needs no bump.

| Where | Carries | Notes |
| --- | --- | --- |
| `SessionRecord` | `judge_readings?` | The Job's `JudgeReading`s, one per artifact (`kind` and `id`), the latest replacing the one before, oldest asked first. It rides every `session.changed`, so no event of its own |
| `JudgeReading` | `id`, `artifact`, `prompt`, `state`, `asked_at`, `finished_at?`, `failed?`, `read`, `notes`, `cost_micros?` | `id` is Fleet's and a reading asked again for the same artifact is a new one. `state` is `reading`, `done` or `failed`. `failed` is Fleet's words and a failed reading is never a result. `notes` empty on `done` is an answer: it read the artifact and found nothing. `prompt` empty asks what was missed. `read` is the person's: the notes were opened |
| `JudgeArtifact` | `kind`, `id`, `title` | `kind` is `job`, `plan`, `pull_request`, `commit`, `file`, `page`, `doc`, `sketch`, `command`, `subagent`, `issue` or `note`. `id` is the kind's own: a pull request's number, a commit's row, a path, a page's address, the plan's version. `title` names it in a line |
| `JudgeNote` | `id`, `about?`, `claim`, `where?`, `expected?`, `found?`, `impact?`, `cites?`, `settled?` | `claim` heads the note and is what the adversarial finding called `consequence`. `about` is the part, in the artifact's own terms: a plan group or task id, `path:line` of a diff, a document's paragraph. `where` is the same place as a person reads it. `cites` is `path` or `path:line`. `settled` is `{ how: "sent" or "dismissed", at }`, absent while the note waits |
| `ask_judge` (`POST /sessions/judge`) | `AskJudge`: `session_id`, `artifact`, `prompt?`. Answers 200 `JudgeReading` | Answers at once with the reading `reading`. A reading already going for the same artifact is answered, not started twice. 404 `fleet.no_such_session`, 409 `fleet.session_closed`. `agent_access: No` |
| `send_judge_note` (`POST /sessions/judge/send`) | `SendJudgeNote`: `session_id`, `reading`, `note`. Answers 200 `JudgeReading` | The note becomes the person's message to the agent, sent before it is settled. A note already settled is answered as it stands. 404 `fleet.no_such_judge_reading`, `fleet.no_such_judge_note`. `agent_access: No` |
| `dismiss_judge_note` (`POST /sessions/judge/dismiss`) | `DismissJudgeNote`: the same three. Answers 200 `JudgeReading` | Settled; the agent is told nothing. The same refusals |
| `mark_judge_read` (`POST /sessions/judge/read`) | `MarkJudgeRead`: `session_id`, `reading`. Answers 200 `JudgeReading` | `read` true. The agent is told nothing |
| `WaitingSource` | `judge` removed | A Judge's note is no longer a waiting item, so it never calls for the person and `waiting_for` carries none |
| `SessionRecord`, `WaitingItem`, `SessionPlan` | `judging`, `judge`, `settled`, `concern`, `cites` and `adversarial` removed | `workflow_step` stays |

**The event stream.**
This adds no event kind and makes the stream's load slightly worse.
A reading starting, finishing, opened and each note settled is one more `session.changed`, each carrying the record whole with every reading on it, so the frame grows by the Job's kept readings.
That is a small load on the sink [that has back-pressure built and no measurement](the-unmeasured-risk-the-websocket-sink-has-no-back-pressure.md), and no change to what a reconnecting Bridge must resync: the readings are on the record it reads.
A Job's readings are one per artifact, so the growth is bounded by what a Job has artifacts for.

**Skew.** A Fleet before this answers 404 for `ask_judge` and its three, and a Bridge before it ignores `judge_readings` and draws a `judge` source as an item it has no card for.
The IDs differ, so the two are refused regardless and the lifeboat serves.
Bridge's half is `packages/protocol/src/judge.ts` and `sessions.ts`, written by hand like the rest.
