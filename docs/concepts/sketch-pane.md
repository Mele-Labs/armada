# Sketch pane

**What it is:** The scene an agent draws with `show_sketch` and the owner's pad shares, and how a sketch is read, played and marked.

---

**Kind:** Concept.

The view is `SketchScene` in `packages/components/src/compositions/`.

## Status

**Rule.** What the owner marks on a sketch lives for the window and is lost on reload.

Gaps found in the sketch are tracked in #2052.

## The scene format

A scene is the sketch pad's drawing plus the kinds the pad cannot say. It is defined in `packages/components/src/compositions/SketchScene/scene.ts`, and an agent's sketch and the owner's pad share it.

| Part | Fields |
|---|---|
| Scene | `nodes`, `edges`, optional `strokes` |
| Node | `id`, `kind`, `x`, `y`, optional `w`, `h`, `title`, `body`, `icon`, `lang`, `wire`, `screen`, `step` |
| Edge | `id`, `from`, `to`, optional `label`, `flow`, `step` |
| Stroke | `id`, `points` of `x` and `y` |

| Node kind | Draws |
|---|---|
| `box` | A frame with a title band and a mono body |
| `group` | A ground region; needs `w` and `h` |
| `lane` | A swimlane ground; needs `w` and `h` |
| `label` | Bare words |
| `code` | A snippet, with `lang` in the header |
| `wire` | A wireframe block named by `wire`, never a control |
| `screen` | A wireframe screen drawn from a layout tree in `screen`; needs `screen` |

**Rule.** A scene arrives as `unknown`, and `parseScene` is the only way in.

**Rule.** A scene that does not validate draws an empty frame: no partial drawing and no repair.
Why: ids are unique per list, every edge joins two nodes the scene holds, and an unknown `kind`, `wire` or `icon` fails the whole scene.

**Rule.** A node `icon` is a registry glyph from `SCENE_ICONS`, and any other name fails the scene.

**Rule.** `sceneOfDrawing` takes a pad's drawing as it is, so nothing the pad can draw is outside the format.

## Screens

**Rule.** A `screen` node is a wireframe of any product's screen, in a neutral kit that takes none of Armada's hues.
An agent says what is on the screen and never where: `w` is its width and its height comes from its contents.

**Rule.** The tree is `ScreenElement` in `packages/components/src/compositions/SketchScene/screen.ts`, and it arrives as `unknown`. Any element that is not one fails the whole scene.

| Element | Takes |
|---|---|
| `row`, `column` | `children`, `gap`, `pad`, `border` |
| `grid` | `cols` of 1 to 12, `children`, `gap`, `pad`, `border` |
| `window` | `chrome` of `desktop`, `browser` or `phone`; `title`, `url`, `children` |
| `nav` | `brand`, `items`, `active`, `actions`, `vertical` |
| `panel` | `title`, `icon`, `actions`, `gap`, `pad`, `children` |
| `tabs` | `items`, `active` |
| `list` | `rows` of `text`, `meta`, `icon`, `selected` |
| `table` | `columns`, `rows` |
| `button` | `text`, `icon`, `variant` of `primary`, `secondary` or `ghost` |
| `field` | `label`, `value`, `placeholder`, `kind` of `input`, `textarea`, `select`, `checkbox`, `toggle` or `search` |
| `text` | `text`, or `lines` bars; `style` of `heading`, `title`, `body` or `caption` |
| `image` | `label` |
| `avatar`, `badge` | `text` |
| `divider`, `spacer` | Nothing |
| `chat` | `messages` of `from` as `me` or `them`, and `text` |
| `modal` | `title`, `children` |

### Sizing

**Rule.** Every element takes `id`, `step`, `size` and `align`. A `size` of `auto` takes the content, `fill` shares what is left along the parent's axis, and a number is the weight of its share.

**Rule.** `size` is a share only in a `row` or a `column`, and a grid cell ignores it.

**Rule.** A part stretches across its parent's cross axis.
A `button`, `badge` and `avatar` keep their own size in a column, and a `button`, `badge`, `avatar`, `text`, `field` and `tabs` sit centred in a row.

**Rule.** An `align` of `start`, `center`, `end` or `stretch` wins over both.

**Rule.** A `gap` or a `pad` is `none`, `sm`, `md` or `lg`. Layouts default to a `sm` gap and no `pad`, and a panel's body to `sm` and `md`. Nothing inside a screen is written in pixels.

### Icons

**Rule.** An element's `icon` is a registry glyph from `SCREEN_ICONS`, kept apart from `SCENE_ICONS`, and any other name fails the whole scene.
A bare string in `items` or `actions` is the words of a button, and `{ icon }` is a button of a glyph alone.

**Rule.** A screen has no `house`, which the registry reserves to the shared home. A Home item takes `layout-dashboard`.

### Steps and modals

**Rule.** An element's `step` joins the scene's step order, beside the steps of nodes and arrows. While it plays the element is ringed and the rest of the screen stands back.

**Rule.** A screen that holds the step in an element does not stand back, and the other nodes do. A screen with a `step` of its own lights as a box does, with its ring on the sheet.

**Rule.** Under a part that stands back nothing else is decided, and under the ringed element nothing stands back.

**Rule.** A `modal` with a `step` is hidden until its step plays and stays up after it. A `modal` with none is always up.

**Rule.** The ring is on a modal's dialog and never on the scrim round it.

**Rule.** An arrow joins whole screens, so two screens and an arrow are a screen flow.

### Sharing

**Rule.** Save as web page and Save as image draw a screen as the canvas does: its own HTML in a `foreignObject`, with the kit's rules and the theme's token values written into the file.
Nothing is fetched.
Why: the browser that opens the file lays the screen out, so the file holds no measurements of it.

**Rule.** The web page holds the screen at rest and once for each step an element of it carries, and its player shows the one for the step playing. The image holds the screen at rest.

**Rule.** Save as image reads its picture from a data address, because Chromium taints a canvas for a `foreignObject` loaded from an object address.

**Rule.** Copy as Mermaid names a screen as one node with its title, or its id where it has none, and nothing inside it.

## Reading a sketch

**Rule.** With an `against` scene, the sketch is read as a change to it, by node and edge id: added is green, removed red, changed amber, and unchanged dimmed.
Why: a node that moved or was reworded counts as changed, and a removed part stays drawn where it was.

**Rule.** Nodes and edges carrying `step` play in that order from the rail.
A step lights its own nodes and the node each of its edges ends at, sends a dot along its edges, and dims the rest.
Groups and lanes never dim.
Why: a step is a distinct number, and what shares one plays together. An edge `step` must be a finite number, or the whole scene fails.

**Rule.** The panel plays on open and loops, with Play or Stop and Restart on the rail. It draws no step counter.

**Rule.** A step lasts the `--duration-pulse` token, and the dot takes three quarters of it.

**Rule.** With reduced motion nothing plays by itself, no dot is drawn, and the same act steps one step per press.

## Agent sketches

**Rule.** An agent's sketch is served by Fleet through `show_sketch`, and reaches Bridge twice: as an artifact on the Job ledger, and as the tool call in the thread.

| Where | Carries |
|---|---|
| Ledger | An artifact `sketch:<title>`, form `sketch`, with `detail` keys `by`, `form`, `title`, `scene` and optionally `against` |
| Thread | The `show_sketch` tool row, whose arguments are `title`, `scene` and optionally `against`, a title |

**Rule.** `detail` holds strings only: `scene` is the scene as JSON text, and `against` is JSON text of `{ title, scene }` with the scene as JSON text again.

**Rule.** The same title is the same ledger row, and a later call replaces it.

**Rule.** The thread draws a card at the point of the call, and a later card with the same title marks the earlier one Replaced.

**Rule.** A card's `against` is the latest earlier card of the title it names, else the ledger's `against` snapshot, else none.

**Rule.** A call with no sketch of its title on the ledger draws no card.
Why: the wire row carries no tool result, so the ledger standing for the title is how a refused call shows.

**Rule.** The card and the ledger row open the same panel, the panel reading as a change when it has an `against`, with a switch to the whole.

**Rule.** The card's thumbnail is the real `SketchScene` with `preview`: fitted, no rail and no pan or pick, inert. It shows the first step and plays while the card is hovered or focused.

**Rule.** Pan, zoom and fit are `GraphCanvas`'s, and the keys `+`, `-`, `0` and the arrows act on a focused sketch.

## What the owner can do to a sketch

**Rule.** His marks are kept apart from the scene: lines, boxes and joins of his, and the ids of the agent's parts he struck out. The agent's parts stay the agent's, and his are drawn in his colour.

**Rule.** The host passes `marks` and `onMarks` to offer the tools. Without them the sketch is read-only and only the step play remains.

| Act | Does |
|---|---|
| Draw | Drags a pen line over the sketch |
| Undo | Takes off his last line |
| Add a box | Places an empty box of his in free view space |
| Join | Joins the two boxes picked, or one box to the next pressed |
| Remove | Deletes his box and its joins, or strikes out an agent's part |
| Put back | Lifts a strike |
| Ask | Opens a small field on a part pressed; Enter sends |

**Rule.** His own boxes are edited and joined, never asked about.
