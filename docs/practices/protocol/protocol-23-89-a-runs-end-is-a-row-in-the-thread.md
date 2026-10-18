# Protocol 23.89: a run's end is a row in the thread

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *Running a block*. **Additive only**: one thread row, and no operation changes.

| Where | Carries | Notes |
| --- | --- | --- |
| `session.row`, `get_session` | `SessionRow` kind `ran`: `id`, `at`, `command`, `code?`, `tail` | A hosted Job's thread keeps one when a run ends, in place of the message from Fleet it kept before. `code` is absent where the run was stopped or a signal ended it. `tail` is the last 40 lines as a person would have read them, escapes and carriage-return overwrites resolved: the text the agent is sent, and empty where the run printed nothing |

The agent is sent the same words as before (the command, `Exit <code>.` or `Stopped.`, the last lines, and the note that Fleet took a waiting item off), **once**, and the thread shows no message from Fleet beside the row. A terminal Job's thread is the transcript its CLI keeps and Fleet cannot write to it, so it has no `ran` row and its mod is still handed the words.

**Skew.** A Bridge before 23.89 reads a `ran` row as a kind it does not know and draws nothing for it, or draws it badly. A Fleet before sends the message. The IDs differ, so the two are refused regardless, and the lifeboat serves. Bridge's half is `packages/protocol/src/hosted-sessions.ts`, written by hand like the rest.
