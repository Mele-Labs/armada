# Protocol 17.4: Fleet decides what Helm is asked about

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1525`. One variant added to `HelmCallSettled` (`ran_unasked`). Additive: nothing renamed, nothing
retyped, no field an older peer reads changed.

**The behaviour behind it is not additive and the wire is.** Fleet now classifies every call a
person's settings did not cover and only puts three classes of them to a person — destructive,
pushes code to a shared space, writes off this machine. A Bridge that does not know `ran_unasked`
sees fewer cards and one settled value it has no wording for, in a record; nothing it draws depends
on the value.

**It is published for a call no card was drawn for**, which is the whole reason it exists. Every
other member of that union answers a card. This one is the only account of what Helm did unasked,
so leaving it off the wire would have made the audit trail quieter exactly where it matters most —
`docs/concepts/helm.md`, *Audit trail*.

**`allow_every_read` is now unreachable and stays on the wire.** 17.3 added it for a card over a
read of Armada's own door; a door read no longer draws one. Removing a variant is a major bump and
a reversal of the owner's own decision of an hour earlier, so it is his call rather than this
change's.
