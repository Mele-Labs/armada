# Protocol 11.6: a recording is watched a span at a time

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#615`. A captured video over 20 MiB never played on the Job screen and a
smaller one downloaded whole before a frame of it drew, because every hop read
the file whole: `get_frame` answered the bytes in one response, Bridge's main
process took them into an array, and the renderer made a `blob:` of it. A
two-minute walk through an app runs past that in a minute, so the evidence most
worth watching was the evidence the screen skipped.

`get_frame` now honours `Range` — **on a frame whose media type is a video, and
on nothing else**. A satisfiable span answers 206 with `Content-Range`; one
beginning past the end answers 416 naming the length, which is the one fact a
player can ask again with. A range on any other kind is ignored and the file is
answered whole, which is always legal and keeps the executable kinds — SVG,
HTML — on the single path that has always answered them as bytes.

**Additive, and that is why the minor moved.** An older Bridge sends no `Range`
and gets exactly the response it got before, headers included. The route, the
id and the refusals are unchanged: `kept` still names a row, the record is
still the allowlist that resolves it before a file is opened, and a name no row
of the Job holds still reaches nothing whatever it spells.

**A long span is windowed rather than honoured**, because a player opens a
recording by asking for everything from byte zero. `showing::frame_part` seeks
and reads a bounded window, so the allocation is the window and never the
capture's length — answering fewer bytes than were asked for is legal, and the
player asks again.

Bridge's half is a privileged scheme in the main process, which forwards the
range to Fleet and streams the answer back. **The renderer still never reaches
Fleet's port** — `docs/practices/bridge.md` is where that half is written down.
