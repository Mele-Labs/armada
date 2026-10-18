# Protocol 23.0: a node sits inside a Zone or a Cluster

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Decided with the owner on 2 Oct 2026: everything a read-in brings back lands inside one Zone, and a
Cluster is a frame round its Notes. `.claude/decisions/2026-10-02-a-read-in-lands-in-a-zone.md`,
#1620.

**Major, because a position an older Bridge reads changed meaning.** `StudioNode` gains `within`,
the id of the frame it sits in, absent on the board; a node with one has its `position` measured
from that frame's corner. A Bridge built before this draws every such node at its offset from the
board's origin, which is the table's *field that parses the same and means something else*.
`StudioNodeContent` gains `{ "kind": "zone" }`, nothing beside the tag, which on its own would be
14.18's additive kind.

**`move_studio_node` takes `within`**, the frame a node was put down in, and absent is the board.
A frame that does not hold the kind is `fleet.studio_frame_cannot_hold`, and a Note moved out of
its Cluster `fleet.studio_note_stays_in_its_cluster`. **`add_studio_node` takes a Zone from
Bridge**, empty. **`group_studio_nodes` draws a Cluster round its Notes** and sets each Note's
`within`, wherever `position` said; a Note already in a Cluster is `fleet.studio_note_in_a_cluster`.

**A read-in's nodes arrive with `within` set**: its Zone, then its Finding inside it, and each
Cluster's Notes inside the Cluster. The source still produces every one of them, the Zone among
them, so `edges` is what it was plus one. Which of those edges Bridge draws is Bridge's
(`docs/concepts/studio.md`, *Edges*). Store V91 adds the column, and every node before it sits on
the board. Minor resets to 0.
