# Drawing a sketch

**Part of:** [Job](../session.md).

**A session draws the person a diagram with `show_sketch`** (`title`, `scene`, optional `against`), a tool on the agent's door, in place of ASCII art.
The scene is the format of `docs/concepts/sketch-pane.md`: `nodes`, `edges` and optional `strokes`.
Fleet places a hosted session by its connection, as for `show_window`; a terminal session names its own id.

**A scene that `parseScene` would draw as an empty frame is refused first.** `ipc` (`sketch.rs`) checks it against the same rules and answers 422 with up to twelve problems, each naming the part (`nodes[1] `b``), what it holds and what is allowed. It is stricter in one way: a field the format has no name for is refused, where `parseScene` drops it. A scene with no nodes is refused. The kinds, wire parts and icons are listed in the tool's description, and `crates/ipc/src/tests/sketch.rs` fails when they differ from `scene.ts`.

**A `screen` node is a wireframe of an app's screen, for any product.** Its `screen` field is a layout tree of rows, columns and grids that size themselves, holding generic parts (window, nav, panel, tabs, list, table, button, field, text, chat, modal and the rest), so the agent says what is on the screen and never where.
`crates/ipc/src/sketch/screen.rs` checks the tree against `readScreen` in `packages/components/src/compositions/SketchScene/screen.ts` and refuses each fault by its path, such as `nodes[2] `team` screen.children[1].rows[0].icon`, with what it holds and what is allowed.
A tree nests at most 12 levels.
The test reads the element types, their fields, the enums and the glyph list out of `screen.ts`.
The tool's description teaches the types, and a refused glyph is answered with the whole list.

| Cap | Holds |
|---|---|
| Nodes, edges, strokes | 80, 120, 20 |
| Scene, written, with `against` | 32768 bytes |
| Title | 120 characters |

**It is kept as an artifact of form `sketch`**, `target` `sketch:<title>`, with `detail` of `by` `agent`, `title` and `scene`, the scene as JSON text, and `against` where the call named one.
A second call with the same title replaces the first where it stands; another title adds one.
It stays on the ledger after the session ends and survives a restart.
The call is a tool row on the thread whose text is `<tool> <arguments as JSON>`, the arguments unwrapped from the door's `body` and carried whole (`title`, `scene`, `against`; not `session_id`), which Bridge draws the sketch's card from.
No window opens.
Through the door the answer is a receipt of `session_id` and `sketch`.

**The person notes a part of it back to the job** by a right-click or a Shift+drag, and the note arrives as a message: [Noting what you read](noting-what-you-read.md).

**`step` orders what plays.** It is a finite number on a node or an edge, with one numbering for both: everything with the same step plays together, in ascending order, the dot travels an edge from `from` to `to`, and an edge with no step never pulses.
A sequence diagram is groups as lifelines, label nodes as points and stepped edges between them; a workflow steps its nodes and its arrows.

**`against` names an earlier sketch of the session by title**, which the owner reads this one as a change to.
Fleet resolves it at the call and keeps that scene whole in `detail.against`, a JSON object `{"title", "scene"}` written as text (`detail` holds strings only), `scene` being the earlier scene's text as it was kept.
A later replacement of the earlier sketch does not move the diff, and a sketch may be drawn against the title it replaces.
A title the session holds no sketch of is refused with the titles it does hold, and a blank one is refused.
The earlier scene counts in the byte cap with the new one.

**The scene rides on the record**, which is published whole on every change, so each sketch is carried by every event after it. That is why the byte cap is 32 KiB and not more.
