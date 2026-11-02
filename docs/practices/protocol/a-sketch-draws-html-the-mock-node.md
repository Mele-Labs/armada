# A sketch draws HTML in the repository's styles: the `mock` node, `get_style_sources` and `get_kept_styles`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 1 Nov 2026. `.armada/documents/hi-fi-sketches.html`, *How a Job draws one* and *How Fleet finds the styles*.
**Additive only**: one node kind and four node fields in the scene `show_sketch` carries, two operations, two routes and the types they answer; no event kind and no field on an existing type.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half of the node is `mock`, `html`, `css`, `styles` and `kept` on `SceneNode` in `packages/components/src/compositions/SketchScene/scene.ts`; the mirror of the two answers is `packages/protocol/src/sessions.ts`, written by hand.

| Where | Side | Carries |
| --- | --- | --- |
| Scene node `kind` `mock` (`show_sketch`) | Job writes, Fleet checks | HTML drawn in a sealed frame `w` wide. Needs `w` (a number) and non-blank `html`; takes `css` (a string) and `styles` (a list of repository paths, strings). At most 12 `mock` nodes per scene (`MOST_MOCKS`), else 422 telling the agent to split it |
| Scene node `html`, `css`, `styles` on any other kind | Fleet refuses | 422 naming the node, as `screen` on a non-`screen` node is |
| Scene node `kept` | Fleet writes | The hash of the style set Fleet drew the mock with. An agent that sends it is refused, so it is not in `NODE_FIELDS` and `tests/sketch.rs` names it as the one field `scene.ts` types and `ipc` does not take |
| `get_style_sources` (`GET /sessions/style_sources?path=&session_id=`, `agent_access = "Yes"`) | Job reads | `StyleSources` `{ sources[], start?, packages[]? }`; each `StyleSource` is `{ path, bytes, kind, usable, tokens[] }` with `kind` one of `css`, `css_module`, `tailwind_output`, `tailwind_reset`, `theme_css`, `tailwind_config`, `scss`, `less`, `theme_module`, and `tokens` `{ name, value }` pairs read from the file, never its text |
| `get_kept_styles` (`GET /sessions/kept_styles?hash=`, `agent_access = "No"`) | Bridge reads | `KeptStyles` `{ sheets[], fonts[] }`: `sheets` are `{ path, css, edited }` in the order they apply, `fonts` are `{ family, weight, style?, bytes }` with `bytes` base64. A hash Fleet does not hold is a 404 |

**The scene's byte cap stays 32 KiB** and counts `html` and `css`, because the scene is written whole.
**The kept sheets and fonts never ride on a record.** The scene carries a hash, the answer to a hash never changes, and Bridge reads it once when it draws the node.
**Not built yet.** The wire, the routes and the door tool are in; Fleet answers `get_style_sources` with 422 `fleet.style_sources_not_built` and every hash of `get_kept_styles` with 404 `fleet.no_such_kept_styles` until it finds and keeps styles.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream is neither better nor worse.** A mock rides in the scene on the Job's record, whose weight the byte cap bounds, and the sheets are read on demand and not published.
The unbounded sink and the missing resync message are untouched.
