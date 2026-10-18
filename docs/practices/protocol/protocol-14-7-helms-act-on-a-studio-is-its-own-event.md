# Protocol 14.7: Helm's act on a Studio is its own event

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1288`. `studio.helm_acted`, a new event kind, additive: published after the
`studio.changed` a write publishes, only where the door placed the call in a
Helm job, carrying `StudioHelmActed { studio_id, manifest_id, act, at }`
with `act` one of `added_node { node_id }`, `proposed_edge { edge_id }` or
`named { name }`. A person's act on a Studio publishes `studio.changed` alone,
so Helm's act is told apart by kind — `docs/concepts/helm.md`, *Audit trail*.

`StudioNode.added_by`, `StudioEdge.added_by` and `Studio.named_by`, additive
and left out where absent: `person` or `helm`, kept by store migration V78. A
row from before V78 has none, since Helm could already act under V77 and a
default would name an author nobody recorded.

`get_checkout_run_sheet`, `list_checkout_runs` and `get_checkout_run_output`
move from `No` to `Helm only`. **That half moves no number**: `agent_access`
decides what the agent door offers, which is not the Fleet/Bridge seam, and no
message either side parses changed.
