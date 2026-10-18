# Protocol 20.0: a Sketch is the pad's drawing

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Decided with the owner on 1 Oct 2026: a Studio's Sketch and the dispatch composer's Sketch pad are
one drawing. `.claude/decisions/2026-10-01-a-sketch-is-the-pad.md`.

**Major, because a field an older Bridge reads is gone.** `StudioNodeContent`'s `sketch` was
`{ "kind": "sketch", "body": "…" }`, the diagram written as text, and is now
`{ "kind": "sketch", "drawing": { "boxes", "joins", "strokes", "pictures" } }`. A Bridge built
before this reads `body` off every Sketch on a Studio and finds nothing, which is the table's
*field removed* row. Keeping a derived `body` beside the drawing would have bought a minor at the
cost of a field nothing new reads, and Armada has no Bridge in the field to keep it for.

**The drawing is the pad's own four parts**, in the pad's coordinates as whole numbers: a box is
`{ id, x, y, body }`, a join `{ id, from, to }` naming a box or a picture, a stroke
`{ id, points: [{ x, y }] }`, and a picture `{ id, x, y, width, height, frame }`, where `width` and
`height` are the size it is drawn at and `frame` is the file Fleet kept, in a Note's
`capture.frame` shape. Every array is sent, empty or not. **It is checked on decode**, by
`core_model::Drawing::drawn`: an id on two parts, a blank id, a join to nothing or to itself, a
stroke of fewer than two points and a picture at no size do not decode, and nor does an unknown
field anywhere in it. Fleet's store holds a row to the same rule on read. Every Sketch written as
text is migrated by store V85 to one box, `b1` at the origin, holding the old body. **A Sketch
carries no `state`**, where it carried `frozen`: it is drawn on whenever its pad is opened, and V85
clears the state on every Sketch row. Finding's and Outline's `frozen` went in 21.0.

**Written, a picture is its staged file or nothing**, the way a Picture is written (19.3). `add_node`
takes `{ "kind": "sketch", "drawing": … }` with each picture carrying `staged`, decoded apart from
every other kind as a Picture is; Fleet copies each into the Studio's keeping under a name it mints,
`<node>-<ulid>.png`, under the 4 MiB cap. **`edit_studio_sketch`**, new and `Bridge only`,
`POST /studios/:studio_id/edit_sketch` with `{ node_id, drawing }`, replaces the whole drawing; a
picture carrying nothing keeps the frame the node already holds under that id, and one it holds
none under is `fleet.studio_sketch_picture_not_kept`. A picture naming `frame` in a write does not
decode, so no client names a file Fleet opens. Frames the new drawing no longer names are deleted
once it is written. A drawing with nothing on it is `fleet.studio_node_blank`, and a redraw of any
other kind `fleet.studio_not_a_sketch`.

**`get_studio_frame` takes `?picture=`**, the picture's id on a Sketch, read off that picture's own
`frame` — the record is still the allowlist. Deleting a Sketch deletes every frame it kept.
