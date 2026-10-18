# Protocol 14.4: an abandoned step's restart names a new trigger

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1034`. `EscalationTrigger` gains `drone_gone`, the step-level trigger a
person's restart writes over a step whose Drone left before anybody acted —
`drone_killed`'s and `run_ended`'s third sibling. **Minor, on `queued_reason`'s
precedent**: `escalation_reason` carries no `wire_enum!` in `crates/ipc`, so
Bridge reads it as an opaque string through the generated vocabulary rather
than matching on it, and a new value is additive while nothing branches on it.
