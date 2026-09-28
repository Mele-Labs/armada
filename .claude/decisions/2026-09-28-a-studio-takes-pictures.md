# Can a picture go on a Studio?

**Decided 2026-09-28.**

He said: *"it feels like our current studio rules are too strict. Some agent
wrote that rule and I didn't agree to it. What does it mean never pixels? I am
constantly providing sketches and screenshots to agents and I must be able to do
so in studio or else its worthless to me."*

He asked for the rule audited before anything was built, rather than choosing a
design. What the audit found:

- **The premise was false when it was written.** `docs/concepts/studio.md:31`
  justifies itself with *"an agent can read a record and cannot read a
  drawing."* Armada's agent door had been returning MCP image content parts
  since 11 September (`crates/ipc/src/door.rs:555`, commit `7eca0abc`). The rule
  landed on 17 September.
- **The reader it protects does not exist.** `crates/fleet/src/briefing.rs`
  never mentions a Studio. Nothing turns a board into prompt text. Two single
  fields of two node kinds reach a model at all.
- **One of seven sites traces to him with a quote**, `design-system.md:118`, and
  it is permissive. The pixel rule's reason clause is attributed to nobody. Four
  of the seven are that one sentence copied outward to satisfy a gate.
- **A false sentence was on screen.** `StudioAddNode.tsx:180` told him, when he
  added a Sketch, that an agent reads a record rather than a drawing.

**Chosen:** Cut the rule and its four copies. Cut the line on screen. Build a
picture a person adds by hand, stored the way a Note's frame already is. Open
`get_studio_frame` past Bridge, which its own note said was expected.

**Cost he took:** A store migration and the four-spelling gate, while #1545 is
trying to settle the schema. And an agent reads the picture by asking for it,
because nothing yet assembles a Studio into a prompt.

**Kept deliberately:** `studio.md:169`, the frame as a file beside the records
with a 4 MiB cap. It is the mechanism that makes pictures work, and its reason
still holds.

**Where it landed:** annotations and the workbook handoff, not yet filed.
