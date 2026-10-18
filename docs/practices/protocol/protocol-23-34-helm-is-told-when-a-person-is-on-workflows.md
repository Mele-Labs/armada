# Protocol 23.34: Helm is told when a person is on Workflows

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

The Workflow creator hands its draft to Helm, and Helm has to know which screen the person is on to read it as a workflow under edit.

**Additive.** `HelmScreen` gains `workflows`, sent in `AskHelm.context.screen` while the Workflow creator is the screen. Fleet's screen phrase for it is *Workflows*. A 23.33 Fleet refuses the value as an unknown variant, and a 23.34 Bridge behind it is refused by the skew rule's own direction, so no Bridge sends it to a Fleet that cannot read it.

**No migration, no store change, and no event.**
