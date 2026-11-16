# An aside to a Job

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 15 Nov 2026.
`docs/concepts/session/an-aside.md` has the model.
The protocol ID is derived from the wire files and needs no bump.

| Where | Carries | Notes |
| --- | --- | --- |
| `Aside` | `aside_id`, `session_id`, `question`, `quote?`, `answer`, `state`, `failed?`, `asked_at`, `answered_at?`, `told_at?`, `reads` | `state` is `asking`, `answered` or `failed`. `answer` grows while `asking`. `failed` is Fleet's words. `told_at` is absent until the owner tells it to the Job. `reads` is always sent, `<Tool> <target>` a line in the order read (`Read <file>`, `Grep <pattern>[ in <path>]`, `Glob <pattern>`), paths relative to the Job's directory where inside it, empty until the copy reads something and for an aside kept before 16 Nov 2026 |
| `ask_aside` (`POST /sessions/aside`) | `AskAside`: `session_id`, `question`, `quote?`. Answers 202 `Aside` | 404 `fleet.no_such_session`. 422 `fleet.aside_blank`, `fleet.fork_point_unwritten`. `agent_access: No` |
| `list_asides` (`GET /sessions/asides?session_id=`) | Answers 200 `AsideList`: `session_id`, `asides` | Oldest first, and the whole side chat of the Job. The answer and the reads of an aside still `asking` are empty here, and one no turn of this Fleet holds reads `failed`. 404 `fleet.no_such_session`. `agent_access: No` |
| `tell_aside` (`POST /sessions/aside/tell`) | `TellAside`: `session_id`, `aside_id`. Answers 200 `Aside` | 404 `fleet.no_such_session`, `fleet.no_such_aside`. 409 `fleet.aside_not_answered`, `fleet.aside_told`. `agent_access: No` |
| `session.aside` | The `Aside`, whole | A client replaces the one it holds by `aside_id`. At most about five a second while the answer or a read arrives, and always the last. Published again when it is told. Not carried by a resync: read `list_asides` |
