# What is a file path pasted onto a Studio?

**Decided 2026-10-01.**

Pasting a path was one of the three things his annotation `20261001-123631-62n3` named. Offered a Note holding the path, a Link holding it as an address, or a kind of its own, he chose the kind of its own.

**Chosen:** a fourth kind a person adds by hand, **File**, beside Note, Link and Sketch.

- It holds `{ path }` and nothing else: absolute, under `~`, or relative to the repository like `crates/fleet/src/briefing.rs`. It is kept as pasted and trimmed, and Fleet neither resolves it nor checks that it exists.
- It is drawn in mono, as a Link's address is, under the kind label "File".
- No rail button and no key. It arrives by paste, and the registry asked for neither.
- No rung starts from it, so nothing reads it in, writes it up or dispatches it.

**Cost he took:** a kind is a migration and the four-spelling gate. That meant store V84, which rebuilds both Studio tables to widen the `CHECK`, and protocol 19.2, a minor bump on 14.7's and 14.18's reading.

**Where it landed:** `studios/paste`, in `crates/core-model/domain/studio-kinds.toml` and its readers.
