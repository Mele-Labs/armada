# Protocol 14.2: a sub-dispatched Job names its parent

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`JobSummary.dispatched_by`, additive: the parent Job's id alone, where `origin` is `sub_dispatched` (#1165).
`sub_dispatched`'s registry sentence, `"Sub-dispatched by {dispatched_by.job_id}"`, had nothing to fill its slot with, so Job detail's facts line drew nothing for it.
`dependencies` and `gate_manifests` stay off the wire — the M1 decision they were withheld alongside `dispatched_by` for — since a caller reading every row still cannot draw the DAG either would.
