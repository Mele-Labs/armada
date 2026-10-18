# Protocol 23.85: the model a hosted Job resolved

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *Which model, and a fresh start*. **Additive only**: one optional field on `HostedFacts`. No operation, event or error is new.

| Where | Carries | Notes |
| --- | --- | --- |
| `HostedFacts.resolved_model?` (`session.changed`, `list_sessions`, `get_session`) | The model the CLI resolved for the running process, as its stream's `init` line names it: `claude-opus-5-5` | Absent until the first `init` of a process, and again once the process is let go (a stop, a tune, a quiet spell). **Not `model`**, which is the alias a person picked and is absent when none was: a job with no pick still runs on something. Left out when empty, never sent as `null` |

**Skew.** A Fleet before 23.85 sends none, so a Bridge names the picked alias and draws no chip where none was picked.
Bridge's half is `packages/protocol/src/hosted-sessions.ts`, written by hand like the rest, and `resolvedModel` on the Job the window draws (`packages/screens/src/sessions-wire.ts`).
