# Protocol 19.3: a pasted path is a File, and a pasted image a Picture

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Two node kinds a person adds by hand: a File, decided with the owner on 1 Oct 2026, and a Picture,
decided with him on 28 Sep 2026.

**A File is the path and nothing else.** `StudioNodeContent` gains `file`,
`{ "kind": "file", "path": "…" }`, and `StudioNodeByHand` gains the same shape. Kept as pasted —
absolute, under `~` or relative to the repository — and trimmed on the way in; Fleet neither
resolves it nor checks that it exists. A blank one is refused as `fleet.studio_node_blank`, and a
body with no `path`, or one that is not text, does not decode.

**A Picture is the frame and nothing else, and it is read and written in two shapes.** Read, in
`StudioNodeContent`, it is the frame Fleet kept, in a Note's `capture.frame` field names:
`{ "kind": "picture", "frame": { "filename", "byte_size", "width", "height" } }`, fetched from
`get_studio_frame` by its node exactly as a Note's is. Written, in `add_node` and `StudioNodeByHand`,
it is the PNG Bridge's main staged, `{ "kind": "picture", "staged": { "staged_path", "width",
"height" } }` — `capture_note`'s `StagedFrame` — and Fleet copies it into the Studio's own keeping
through the same path a capture's frame takes, under the same 4 MiB cap. Over the cap is
`fleet.studio_frame_too_large`, a file Fleet cannot read is `fleet.studio_frame_unreadable`, and
from Helm a Picture is `fleet.studio_node_not_helms`, each before anything is written. Deleting the
node deletes the file.

**A write never names a kept frame.** A kept frame is a file name Fleet chose, and
`get_studio_frame` opens what a node names, so an `add_node` body with `"kind": "picture"` decodes
as the staged shape alone: one naming `frame`, beside `staged` or instead of it, or naming nothing
staged, does not decode. On Fleet's side the request is `ipc::StudioNodeAdded`, whose other variant
holds an `AddedContent` that cannot be a Picture; TypeScript's `AddStudioNode` says the same with
`Exclude`. Only Bridge's main builds the staged shape, from bytes it staged itself — the renderer
never names a path.

**Nor does a Note's `capture.frame`, which closed a hole older than the Picture.** A Note through
`add_node` could carry any file name there, and `get_studio_frame` joined it onto the Studio's
directory. Such a body no longer decodes; a captured Note's frame arrives staged, through
`capture_note`, as it always has, and no Bridge sent one the other way. Behind the record, Fleet
also refuses to open or delete a kept name that is not one plain path component — a row holding
`../x` reads as `fleet.studio_frame_unreadable` and is skipped by a delete.

**Additive by 14.7's and 14.18's reading**, which added node kinds the same way. Nothing an older
Bridge already parses changes; one meeting a File or a Picture draws no card for it, which is
`packages/screens`' `cardOf` default for a kind it does not know.
