# Protocol 23.49: a job Fleet hosts

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *A job Fleet hosts*.
**Additive only**: the operations below, one event kind, one optional field on `SessionRecord`, and the DTOs of `ipc::hosted_sessions`.
Nothing an older Bridge reads changes, and Helm's operations are untouched.

| Operation | Carries | Notes |
| --- | --- | --- |
| `start_session` (`POST /sessions/start`) | `StartSession`: `manifest_id`, and `title`, `model`, `effort`, `mode` | Answers the `SessionRecord`, `origin` `bridge`, `hosted` set, holding nothing |
| `send_session_message` (`POST /sessions/message`) | `SendSessionMessage`: `text`, `attachments` (`name`, `media_type`, base64 `data`), `mentions` (`SessionTag`) | **202**. Answers the `SessionRecord`; the reply is the thread's |
| `answer_session_ask` (`POST /sessions/ask/answer`) | `AnswerSessionAsk`: `session_id`, the ask's `call`, an `answer` it offered, a `note` | 409 where nothing is waiting under that call |
| `tune_session` (`POST /sessions/tune`) | `TuneSession`: `model`, `effort`, `mode` | A model or effort left out is the machine's own |
| `begin_session` (`POST /sessions/begin`) | `BeginSession`: `session_id`, `delivery?` | Start pressed on a held Job's start card. Answers the `SessionRecord`. `a-job-starts-on-its-start-card.md` |
| `close_session` (`POST /sessions/close`) | `CloseSession` | Parks the slot, ends the row |
| `stop_session` (`POST /sessions/stop`) | `StopSession` | Ends the turn that is running, keeps the row live and the slot held. Since 23.84 |
| `get_session` (`GET /sessions/one?session_id=`) | `SessionThread`: the record and its `rows` | |
| `get_session_subagent` (`GET /sessions/subagent?session_id=&subagent_id=`) | `SessionSubagent`: the subagent's own `rows`, `finished`, and its `report` once it has; `model`, `context_tokens` and `context_window` from its last answer, each absent until it has answered | Read again while it runs; 422 where the job has no such subagent transcript |
| `get_session_file` (`GET /sessions/file?session_id=&file=`) | The bytes, under the media type they were sent as | `file` is a `SentFile.id` |
| `gate_session_call` (`POST /sessions/gate`) | `SessionGate`, answered in the harness's own hook shape | **Reached by the harness, never by a client** |
| `ask_from_terminal` (`POST /sessions/ask/terminal`) | `TerminalAsk`: `asks` with the question's input, or a permission's with its `tool`, answered at once with `TerminalAsked` `asked` and the call's id; `wait` with that call, held up to 25 seconds and answered with `answered` (the input and its `answers`), `refused`, `gone`, or `waiting` where it ran out; `settled` where the terminal's own prompt ended first | **Reached by the job's mod, never by a client.** Writes the same `ask` row a hosted job's question does, and none for a permission; the question and a Bridge answer are kept in the store until the mod collects them |
| `take_held_messages` (`POST /sessions/held`) | `TakeHeld`, answered with `MessagesHeld`: what a person sent a terminal job, once | **Reached by the job's mod, never by a client.** Since 23.53 |
| `session.row` (event) | `SessionRowChanged`: `session_id`, `row` | Appends, or replaces the row with that `id` |

**A row is `message`, `tool`, `lease` or `ask`**, tagged by `kind`.
A message's `from` is `you`, `agent` or `session`, with the other job named.
An ask carries the whole `HelmCallInFlight` and a `state`, and is replaced by id as it is answered.
`session.changed` still carries the row whole, now with `hosted`: the `turn` (`idle`, or `working` with `woken_by` where another job started it), what the agent is `asked`, `model`, `effort`, `mode` and whether a process is `running`.

**`SessionRecord.hosted` is optional**, so a record a terminal reports reads as before.
**One migration**, `session_host.tables`: `hosted_sessions` and `session_rows`.
Bridge's half is `packages/protocol/src/hosted-sessions.ts`, written by hand like the rest.

**Two changes that are not wire.** The agent door offers a hosted job what any agent of the person's is offered, and `ask_the_person`, so its permission tool is answerable; its acts are not recorded as Helm's.
And `report_session` keeps `origin: bridge` when the mod in a hosted job reports `started`, and ignores its `ended`.
