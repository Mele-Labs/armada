# Sketch pane

**What it is:** What the Overview draws to the left of the Now panel while something asks: a Drone's sketch, the asker's own view, or the canvas.

---

**Kind:** Concept.

The pane is drawn by `packages/surfaces/jobs/src/OverviewBoard.tsx`. Its views are `SketchScene` and `AskerView` in `packages/components/src/compositions/`, and the panel that picks between them is `NowPanel` beside them.

## Status

**Rule.** Nothing here is served by Fleet yet: the Now panel's asks, sketches and askers are mock data, and a real Fleet draws no panel.
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
| Node | `id`, `kind`, `x`, `y`, optional `w`, `h`, `title`, `body`, `icon`, `lang`, `wire`, `step` |
| Edge | `id`, `from`, `to`, optional `label`, `flow` |
| Stroke | `id`, `points` of `x` and `y` |

| Node kind | Draws |
|---|---|
| `box` | A frame with a title band and a mono body |
| `group` | A ground region; needs `w` and `h` |
| `lane` | A swimlane ground; needs `w` and `h` |
| `label` | Bare words |
| `code` | A snippet, with `lang` in the header |
| `wire` | A wireframe block named by `wire`, never a control |

**Rule.** A scene arrives as `unknown`, and `parseScene` is the only way in.

**Rule.** A scene that does not validate draws an empty frame: no partial drawing and no repair.
Why: ids are unique per list, every edge joins two nodes the scene holds, and an unknown `kind`, `wire` or `icon` fails the whole scene.

**Rule.** A node `icon` is a registry glyph from `SCENE_ICONS`, and any other name fails the scene.

**Rule.** `sceneOfDrawing` takes a pad's drawing as it is, so nothing the pad can draw is outside the format.

## Reading a sketch

**Rule.** With an `against` scene, the sketch is read as a change to it, by node and edge id: added is green, removed red, changed amber, and unchanged dimmed.
Why: a node that moved or was reworded counts as changed, and a removed part stays drawn where it was.

**Rule.** Nodes carrying `step` play in that order from the rail, one lit at a time and the rest dimmed.
Why: with reduced motion the same act steps one node per press.

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
