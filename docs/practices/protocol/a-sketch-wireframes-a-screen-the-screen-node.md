# A sketch wireframes a screen: the `screen` node

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 16 Oct 2026. `docs/concepts/sketch-pane.md`, *Screens*, and `docs/concepts/session.md`, *Drawing a sketch*.
**Additive only**: one node kind and one node field in the scene `show_sketch` carries, no operation, no event kind and no route.
The protocol ID moves with the files below, and there is no number to take.

| Where | Side | Change |
| --- | --- | --- |
| Scene node `kind` `screen` and field `screen` (`show_sketch`, `POST /sessions/sketch`) | Job writes, Fleet checks | A layout tree of rows, columns, grids and wireframe parts. `crates/ipc/src/sketch/screen.rs`, the same rules as `screen.ts`; a tree on any other kind, or a screen with none, is refused |
| Fleet's check | Fleet refuses | 422 naming the part by its path in the tree, what it holds and what is allowed. Nesting is bounded |
| `Attachment.detail.scene` on the `sketch` artifact | Fleet writes, Bridge reads | Unchanged in shape: the scene is JSON text, so Bridge's `parseScene` reads the new kind with no DTO to mirror |

**Skew.** A Bridge before this draws a scene holding a screen as an empty frame, because `parseScene` refuses a kind it does not know.
The IDs differ, so the two are refused regardless.
The scene stays under the sketch byte cap: a desktop screen of the 16 Oct layout measured 2560 bytes.
