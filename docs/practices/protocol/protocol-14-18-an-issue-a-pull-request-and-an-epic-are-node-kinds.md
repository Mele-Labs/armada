# Protocol 14.18: an Issue, a Pull request and an Epic are node kinds

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`StudioNodeContent` gains `issue`, `pull_request` and `epic`, and `Link` loses `forge`. `#1394`.

Each of the three carries `address`, `number` and `said`, and a `title` absent until the node is
read in. An Issue and a Pull request carry `state` — `open`, `closed` or `merged` — and an Epic
carries `read_in`, `{ issues, total }`, how many of its issues are on the Studio of how many it
holds. Every field but `address` and `number` is optional and left out rather than sent as null.

**The kind is the concept and the adapter decides it.** `adapters::forge_node` reads an address
once, when the node is made, and answers with the node's content. Nothing reads an address again:
`Studio::of` no longer takes a classifier, and `dispatch_studio_draft` offers the three by kind.
A Link is what no adapter recognised — a board, a page, a document, a job — and dispatches
nothing.

**`add_studio_node` still takes a Link, and Fleet writes what it is.** Bridge cannot read an
address, so the seam carries the paste and not the kind; `StudioNodeByHand` is unchanged.

**Additive by 14.7's reading, which added `finding` the same way.** `forge` goes with nothing that
carries one: a Link whose address names something on the forge is converted on the boot that
applies store V79, so no message an older Bridge parses stops parsing the same way. An older
Bridge meeting one of the three leaves it off the whiteboard rather than failing, which is
`whiteboardEdges`' rule for an unknown edge and is what `cardOf` gained here.
