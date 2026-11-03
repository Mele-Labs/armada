# A question kept whole: the `kept` ask state

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 17 Oct 2026. `docs/concepts/session.md`, *Showing a window*.
**Additive only**: one variant on `SessionAskState`, `kept`.
Bridge's half is `packages/protocol/src/hosted-sessions.ts`, written by hand.

| Where | Side | Change |
| --- | --- | --- |
| `SessionAskState` `kept` (`session.row`) | Bridge reads; Fleet no longer writes it | An `AskUserQuestion` that was refused because its job's walk was open. The row carries the whole question as any ask row does, and Bridge draws it as the question that lapsed, answerable in the Needs you list. Unlike `unanswered` and `refused`, a message from the person does not clear it unless the message answers it |

A Bridge before this reads an ask state it has no word for and draws the row as a settled tool line.

**Fleet no longer writes `kept` (3 Nov 2026).** A walk is a tab beside the thread and stays open for hours, so the Job's `AskUserQuestion` is put to the person while one is open, as at any other time, and `show_window` no longer tells the Job to end its turn.
The variant stays on the wire: a thread written before reads, and answers from Needs you, as it did.
Nothing about the shape changed.
