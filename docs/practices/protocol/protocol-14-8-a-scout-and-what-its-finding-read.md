# Protocol 14.8: a scout, and what its Finding read

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1292`. Three commands and a Finding's fields, all additive. `ask_scout` (`Bridge only`) adds a Finding Gathering and starts its scout; `start_scout` (`Helm only`) starts a Finding already Proposed; `stop_scout` (`Bridge only`) is the stop on its node. A Finding's content keeps `asked` and gains `checkout` (`commit`, `uncommitted`), `read`, `searched`, `learned` and `ended` (`outcome` of `answered`, `stopped` or `failed` with `why`, and `cost_micros`), each left out until the scout records it — so a Proposed Finding is on the wire exactly as it was at 14.6.

**`cost_micros` is absent, never nought, where no cost was reported**: a scout whose group had to be ended rather than interrupted reports none, spike 017. `outcome` is a serde tag rather than a `wire_enum!`, for the node's own `kind`'s reason: it is the field the rest hang off. A Finding added through `add_studio_node` carrying any of the new fields is refused as `fleet.studio_finding_is_the_scouts`.
