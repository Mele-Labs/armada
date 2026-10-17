# Sketch pane

**What it is:** What the Overview draws to the left of the Now panel while something asks: a Drone's sketch, the asker's own view, or the canvas.

---

**Kind:** Concept.

The pane is drawn by `packages/surfaces/jobs/src/OverviewBoard.tsx`. Its views are `SketchScene` and `AskerView` in `packages/components/src/compositions/`, and the panel that picks between them is `NowPanel` beside them.

## Status

**Rule.** The Now panel's Drones, Checks, Judge, asks, issues and waiting are served by Fleet ([Job](job.md), *A Job's Now*). Its Plan interview, sketches and askers are still mock data, and a real Fleet draws none of them.
Why: Fleet publishes no plan interview. The shapes are in `packages/surfaces/jobs/src/draft/now.ts`.

**Rule.** What the owner marks on a sketch lives for the window and is lost on reload.

Gaps found in the sketch are tracked in #2052.

## The three views

| View | Draws | Drawn when |
|---|---|---|
| Sketch | The scene of the open ask | An ask holds a sketch |
| Asker | The asking Drone's or Judge's own view | An ask holds an asker |
| Canvas | The Job's canvas, or its approval | Nothing asks, or the panel is hidden |

**Rule.** With no pick, the sketch shows where the ask drew one, else the asker, else the canvas.

**Rule.** The owner's pick holds while something asks and is forgotten once nothing does.

**Rule.** A pick of a view nothing supplies falls back to the default, and hiding the panel returns the canvas.

## The Now panel's part

**Rule.** The panel draws no sketch and no asker. It reports the open ask's sketch and asker to the host through `onSketch` and `onAsker`, and reports `undefined` once none is open.

**Rule.** The panel head draws a switch only when a sketch or an asker is open and the host handles `onSketchView`.

| Toggle | Glyph | Drawn when |
|---|---|---|
| Sketch | `drafting-compass` | A sketch is open |
| Asker | `activity` | An asker is open |
| Canvas | `network` | The switch is drawn |

**Rule.** A Plan ask previews an option's sketch while the option is hovered, focused or picked, drawn as a change against the decision's own sketch.

**Rule.** A Judge's or a Drone's ask carries its sketch and asker directly, and a Plan ask reports neither once it is answered.

**Rule.** What the owner asks about a part comes back to the panel as a remark under the asks, and what he marked goes with a Plan answer as `edits`.

## The scene format

A scene is the sketch pad's drawing plus the kinds the pad cannot say. It is defined in `packages/components/src/compositions/SketchScene/scene.ts`, and a Drone's sketch and the owner's pad share it.

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

**Rule.** A `screen` node is a wireframe of any product's screen, in a neutral kit that takes none of Armada's hues. An agent says what is on the screen and never where: `w` is its width and its height comes from its contents.

| Part | Types |
|---|---|
| Layout | `row`, `column`, `grid` with `cols`; each has `children`, `gap`, `pad` and `border` |
| Frame | `window` with `chrome` of `desktop`, `browser` or `phone` |
| Parts | `nav`, `panel`, `tabs`, `list`, `table`, `button`, `field`, `text`, `image`, `avatar`, `badge`, `divider`, `spacer`, `chat`, `modal` |

**Rule.** Every element takes `id`, `step`, `size` and `align`. A `size` of `auto` takes the content, `fill` shares what is left, and a number is the weight of its share.

**Rule.** A `gap` or a `pad` is `none`, `sm`, `md` or `lg`. Nothing inside a screen is written in pixels.

**Rule.** An element's `icon` is a registry glyph from `SCREEN_ICONS`, kept apart from `SCENE_ICONS`, and any other name fails the whole scene. A bare string in `items` or `actions` is the words of a button, and `{ icon }` is a button of a glyph alone.

**Rule.** An element's `step` joins the scene's step order. The element is ringed while its step plays and the rest of the screen stands back. A `modal` with a `step` comes up when its step does.

**Rule.** An arrow joins whole screens, so two screens and an arrow are a screen flow.

**Rule.** Share on a scene holding a screen draws the screen's outer frame and its title in the image and names it in the Mermaid text. The elements inside are not exported.

## Reading a sketch

**Rule.** With an `against` scene, the sketch is read as a change to it, by node and edge id: added is green, removed red, changed amber, and unchanged dimmed.
Why: a node that moved or was reworded counts as changed, and a removed part stays drawn where it was.

**Rule.** Nodes and edges carrying `step` play in that order from the rail. A step lights its own nodes and the node each of its edges ends at, sends a dot along its edges, and dims the rest. Groups and lanes never dim.
Why: a step is a distinct number, and what shares one plays together. An edge `step` must be a finite number, or the whole scene fails.

**Rule.** The panel plays on open and loops, with Play or Stop and Restart on the rail. It draws no step counter.

**Rule.** A step lasts the `--duration-pulse` token, and the dot takes three quarters of it.

**Rule.** With reduced motion nothing plays by itself, no dot is drawn, and the same act steps one step per press.

## Agent sketches

**Rule.** An agent's sketch is served by Fleet through `show_sketch`, and reaches Bridge twice: as an artifact on the Session ledger, and as the tool call in the thread.

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

**Rule.** His marks are kept apart from the scene: lines, boxes and joins of his, and the ids of the Drone's parts he struck out. The Drone's parts stay the Drone's, and his are drawn in his colour.

**Rule.** The host passes `marks` and `onMarks` to offer the tools. Without them the sketch is read-only and only the step play remains.

| Act | Does |
|---|---|
| Draw | Drags a pen line over the sketch |
| Undo | Takes off his last line |
| Add a box | Places an empty box of his in free view space |
| Join | Joins the two boxes picked, or one box to the next pressed |
| Remove | Deletes his box and its joins, or strikes out a Drone's part |
| Put back | Lifts a strike |
| Ask | Opens a small field on a part pressed; Enter sends |

**Rule.** His own boxes are edited and joined, never asked about.

## The Asker view

`AskerView` draws only the asker; the panel lists everything else that runs.

| Frame | Holds |
|---|---|
| The asker's name | Drone or Judge mark, step, running or waiting mark, last actions, output tail |
| Work product | A Judge's work product under review |
| Checks it is reading | A Judge's Checks, each with state and an optional tail |
| Changed so far | Files added, changed or removed, the file asked about marked, optional diff lines |

**Rule.** A file row opens the file when the host passes `onOpenFile`, and is a plain row without it.
