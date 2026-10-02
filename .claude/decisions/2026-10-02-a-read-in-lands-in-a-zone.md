# Where does what a read-in brings back land, and what is a Cluster on the board?

**Decided 2026-10-02.**

Reading issue #1657 into a Studio dropped eighteen loose cards in a grid — a
Finding, twelve Notes, four Clusters and a Contradiction — each with its own
`Produced` line from the Issue. The owner, on the Finding: *"I feel like when I
read something in from an issue or write something up they should get all put
into a nice framed region to indicate they all go together. Then I could move
them all around on their own."* On a Cluster: *"Shouldn't clusters just be
frames around a bunch of nodes? Im confused why these clusters are showing as
nodes with nothing in them?"*

**Chosen:** **Inside one Zone.** *"Everything the read-in brings back lands
inside one Zone, with one line from the issue to the Zone instead of 18. You
drag the Zone and they all move together, and you can still rearrange inside
it."* And **a frame around its notes**: *"A Cluster draws as a titled box around
the notes in it, like a Zone, and it can sit inside a read-in's Zone. Moving it
moves its notes."*

**Cost he took:** a Zone is a new node kind across the store, the wire and
Bridge, about a day's build; and a Note is in one Cluster at a time, or it would
have to be drawn twice. Freehand ink in a Zone stays out, and so does
dispatching one.

**What it says about him:** a board he cannot arrange is a board he cannot read,
and he will take a new kind over a convention a person has to remember. What
goes together should look together and move together.

**What was decided in the build, and not by him:**

| Gap | Built as |
|---|---|
| How containment is kept | A `within` column on `studio_nodes`, store V88; a contained node's `x` and `y` are from its frame's corner, so moving a Zone is one write |
| The Finding | Inside its Zone, as its first item |
| A Zone's size | Never kept. Bridge sizes each frame round what it holds, every render |
| The one line | **What is drawn, not what is kept.** The Issue still produces every node the read-in made, and the Zone too; Bridge draws a `Produced` edge into a node inside a frame its source also produced as that frame, and a Note's edge into the Cluster round it as the frame |
| A Note named by two of a scout's Clusters | Drawn in the first, and joined to that one only |
| Grouping Notes by hand | The Cluster is drawn round them, inside the Zone they share, wherever the request said to put it; a Note already in a Cluster is refused |
| An Epic | Its issues land in a Zone too; one whose block was laid out before this keeps it on the board |
| Moving in and out | A node dropped on a Zone goes in, and dragged off one comes out; a Note never leaves its Cluster |
| Deleting a frame | What it held stays where it was on the board |

**Where it landed:** `studio/read-in-lands-in-a-zone`, #1620.
