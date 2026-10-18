# A question refused for a walk is kept whole

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 17 Oct 2026. `docs/concepts/session.md`, *Showing a window*. **Additive only**: one variant on `SessionAskState`, `kept`. Bridge's half is `packages/protocol/src/hosted-sessions.ts`, written by hand.

| Where | Side | Change |
| --- | --- | --- |
| `SessionAskState` `kept` (`session.row`) | Fleet writes, Bridge reads | An `AskUserQuestion` refused because its job's walk is open. The row carries the whole question as any ask row does, and Bridge draws it as the question that lapsed, answerable in the Needs you list. Unlike `unanswered` and `refused`, a message from the person does not clear it unless the message answers it |

A Bridge before this reads an ask state it has no word for and draws the row as a settled tool line.
