# Protocol 21.1: a workflow says what requests it is for

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Decided with the owner on 1 Oct 2026: the approval screen shows the promise the proposer picked a
workflow on. One optional field added to `WorkflowSummary`, `for_requests` — the definition's own
`for_requests` line, the sentence saying what kind of request the workflow is for. Additive by
18.4's reading: a Bridge built before this ignores it, and a Fleet built before this sends none.

**Absent is a definition that declares no line**, and a Fleet older than 21.1; the two read the
same, because either way there is no promise to show. It is `Option<String>` left out when empty,
and `?: string` on Bridge's side. `crates/fleet/src/wire.rs`'s `workflow_summary` is the one
builder.

**It is served on the workflow list and nowhere else.** `JobDetail` carries the Job's frozen
workflow, which `core-model` holds without the line, so Bridge reads it off `GET /workflows` by the
Job's `workflow_id` and `manifest_id`. The list is the catalogue as it stands now, not the
definition the Job froze; a line edited since dispatch reads as the new one.
