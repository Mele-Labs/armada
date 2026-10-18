# Protocol 14.11: Studio capture, and what a Note keeps of it

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1290`. One command and one optional field, both additive. `capture_studio_note` (`agent_access`
`No`) puts a Note on a Studio where a person pointed in Bridge. A `note` node's content keeps
`said` and gains `capture`, left out on a Note that was typed rather than pointed — so a Note from
before this is on the wire exactly as it was at 14.6.

`capture` carries the development annotation layer's own fields — `component`, `owners`,
`selector`, `element`, `screen`, `layer`, `location`, `bounds` and `window` — and four the layer
does not record: `styles`, `markup`, `source` and `frame`. **`source` is absent, never guessed**:
React 19 fibers carry no `_debugSource`, so Bridge sends a path only where the build gives it one.

**The frame crosses as a staged file and reads back as a kept one.** The request's `frame` is
`staged_path`, `width` and `height` — the PNG Bridge's main process wrote where `stage_attachment`
writes one. Fleet copies it under `<machine>/studios/<studio_id>/` and the Note's `capture.frame`
names `filename`, `byte_size`, `width` and `height`. Nothing about where Bridge staged it reaches
a client, and no image crosses in a `studio.changed`. A staged frame over 4 MiB is
`fleet.studio_frame_too_large`, and one Fleet cannot read is `fleet.studio_frame_unreadable`.
