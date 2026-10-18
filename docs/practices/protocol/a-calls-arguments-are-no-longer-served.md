# A call's arguments are no longer served

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1774`. **A removal**: `get_call` (`GET /jobs/:job_id/calls/:call_id`) and `CallArguments` are gone, from `crates/ipc` and from `packages/protocol`. Bridge stopped reading it when the activity log's sheet went (`#1761`, `#1773`), so no Bridge on this side of the ID asks for it.

| Where | Carries | Notes |
| --- | --- | --- |
| `explain_command` (`GET /jobs/:job_id/calls/:call_id/explain`) | unchanged | Still under the call. Fleet reads a cut command whole from the transcript for it, and that read no longer crosses the wire |
| `Saw.called`, `Refusal`, `CommandInFlight` | unchanged | `truncated` and `length` still say how much was cut. Nothing serves the rest |
