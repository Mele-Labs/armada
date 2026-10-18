# Protocol 23.56: a command and a compaction in a terminal job's thread

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *A terminal job's thread*. **Additive only**: two row kinds and nothing else. 23.53 is the terminal job.

| Where | Carries | Notes |
| --- | --- | --- |
| `session.row`, `get_session` | `SessionRow` kind `command`: `text` | A slash command or `!` shell line as the person typed it, `/reload-plugins` or `/model opus`. **No markup and no output**: the command's own output is not drawn |
| `session.row`, `get_session` | `SessionRow` kind `compaction`: `text` | The summary the CLI wrote in place of a conversation it compacted. It is neither the person's nor the agent's words |

Bridge's half is in `packages/protocol/src/hosted-sessions.ts`, written by hand like the rest.
