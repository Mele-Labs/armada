# Protocol 23.78: a Job's retro in its thread

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/retro.md`, *A Job's retro*. **Additive only**: one thread row, no operation, state or error.

| Where | Carries | Notes |
| --- | --- | --- |
| `SessionRow` kind `retro` | `id`, `at`, `stage`, `stage_since`, `characters`, `items`, `retro?`, `why?` | Added when a press is accepted, replaced by id as the retro is written, kept when done. `stage` is `asking_agent`, `reading_record`, `writing`, `written` or `failed`. `characters` is how much of the model's answer has arrived. `items` is a list of `PartialRetroItem` (`title?`, `what?`, `fix?`, `who?`, `lands_in?`), each field present once its value has ended, and is empty outside `writing`. `retro` is the kept retro's number on `written`. `why` is Fleet's words on `failed` |
| `session.row`, `get_session` | The row above | A hosted Job only. `write_session_retro` still answers when the retro is written |

**A press that is refused, and one answered with the last retro, adds no row.** A row a restart left going is returned by `get_session` as `failed` with `why` "Fleet restarted while it was written".
Replacements are at most four a second; a stage change and the last state are always sent.

**Skew.** A Bridge before 23.78 reads a `retro` row as a row kind it does not know and draws nothing for it, or draws it badly; it still reads the retro from `write_session_retro`.
A Fleet before it sends no such row.
Bridge's half is `packages/protocol/src/hosted-sessions.ts`, written by hand.
