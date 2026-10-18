# Protocol 14.9: Helm is told which Studio a person has open

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1287`. `HelmScreen` gains `studio`, the Studios surface, and `HelmContext` gains `studio` and
`node`, both optional and left out where empty: the Studio open on that surface, and the node
selected on its whiteboard, only beside its Studio. Fleet's line to the job names both by id,
so Helm can read the Studio with `get_studio` rather than guess one. **Minor, though Fleet matches
on the screen**: Bridge sends `studio` only to a Fleet at 14.9 or later, because a Fleet behind
Bridge is refused before an ask is ever sent.
