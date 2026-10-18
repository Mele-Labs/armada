# Protocol 23.9: a node is added inside a Zone

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

The owner, 2 Oct 2026: pressing inside a Zone places the armed kind there and puts it in that Zone.
Bridge did it in two writes, an `add_node` on the board and a `move_studio_node` into the Zone, and
a refused move left the node on the board.

**`add_studio_node` takes `within`**, `move_studio_node`'s field: the frame it is added in, with
`position` measured from that frame's corner, and absent is the board. It is one store write, and
it refuses what the move refuses, with the move's codes: a frame that does not hold the kind is
`fleet.studio_frame_cannot_hold`, so a Zone in a Zone is, and so is anything added in a Cluster,
since a node just made is none of the Notes the Cluster was made of.

**Additive, so the minor moves.** A new optional field on a request. A 23.7 Fleet would drop
`within` and add the node at the board's origin plus the offset, so a 23.8 Bridge behind it is
refused, which is the skew rule's own direction.
