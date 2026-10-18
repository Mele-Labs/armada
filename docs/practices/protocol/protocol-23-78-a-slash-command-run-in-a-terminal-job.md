# Protocol 23.78: a slash command run in a terminal Job

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *Running a block*. **Additive only**: one operation, `agent_access: No`, and its type. No type that exists changes.

| Where | Carries | Notes |
| --- | --- | --- |
| `run_session_command` (`POST /sessions/command`) | `RunSessionCommand`: `session_id`, `command`. Answers 200 `SessionRecord` | Holds the command for the Job's own mod as a `HeldCommand` (name without the slash, the rest as `args`), the path `tune_session` takes for `/model`; the mod runs it as typed on its next ask and nothing comes back. 422 `fleet.session_command_refused` where the Job is unknown or not a terminal one, or the text is not one slash command on one line (`/name` or `/name args`, a name of lower-case letters, digits, `:` and `-`); 409 `fleet.session_closed` where it ended, and `fleet.terminal_session_unreachable` where its mod is not listening |

**Skew.** A Fleet before 23.78 answers 404 and Bridge shows the refusal as for any act on a Job. A Bridge before it never calls the route. Bridge's half is `packages/protocol/src/hosted-sessions.ts`, written by hand like the rest.
