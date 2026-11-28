# Scenes

**What it is:** A live prototype: a scene file an agent writes in HTML-shaped markup, which the native app draws from its own parts in a Job's Prototype tab and redraws on every save.

---

**Kind:** Surface.

**Part of:** [Native Bridge](native-bridge.md).

The agent shows one with `show_prototype` (`crates/ipc/operations/show_prototype.toml`).
The reader and the drawing are in `crates/bridge-native/src/prototype/`.
How long a save takes to draw is in [the scene draw time spike](../spikes/scene-draw-time.md).

*Prototype* is also the name of the spike workflow in `.armada/workflows/prototype.json`: Frame, Build, Write up.
A scene is not that workflow's output and needs neither.

## What the owner gets

**Rule.** A prototype is a screen he can click through, in place of a still mock.
Why: a mock shows one state, and a scene shows each state the agent declared and the presses between them.

**Rule.** A prototype shows flow and never the look, so it asks him nothing: it has no Approve and raises no Needs you item (owner, 27 Nov 2026).
Why: approving a prototype approves an assumption that an agent can carry it into the app's style, and he approves an exact look.
A screen he would see is approved on its native walk, `armada://walk/<name>` with the gate `walk_approved`, and Fleet refuses a plan group he would see that ends on `sketch_approved`.

| Where | What it does |
|---|---|
| The Prototype tab | Draws the scene in a frame: an accent hairline, and a band naming it a Prototype with its title |
| A view tile, or a part with `goes` | Steps to that state; a tile names the state by its `title` over its `says` |
| Pop out, or ⌥⌘P in the tab | Draws the scene in a window of its own |
| ⌥⌘A, in the tab, the review sheet or the window | Starts a note, which names the part by its `data-part` and goes to the prototype's own Job |
| A note | Reaches the Job as a message, as a note on a sketch that asks nothing does |

The state he pressed stays across a redraw, unless the agent's call names a new `state`.

## What it is not

**Rule.** A scene is data, not a code mod.
Why: native code mods wait for the WebAssembly kit (decision 3 in [Native Bridge](native-bridge.md)), so a scene cannot add behaviour.

| Real | Not real |
|---|---|
| The renderer, the theme and the tokens | Behaviour past the states the scene declares |
| The shared parts a scene names | Live Fleet data: fixtures only, for now |
| Keys, focus and VoiceOver labels | Drag, canvas, animation and rich text, which need Rust |

## Where it lives

**Rule.** A scene is `$ARMADA_SCRATCH/scenes/<name>.html`, with its fixtures beside it, and is never in the worktree.
Fleet keeps each revision with the artifact, so the approved one outlives the scratch folder, which goes when the Job ends.

Fleet watches the folder with `crates/adapters/src/folder_watch.rs`, from the first prototype a Job shows.
A save of the scene or a fixture it names waits for a quiet moment, then puts the part again as its next revision.
A part he has approved or withdrawn is left as it is.

## The act

**Rule.** `show_prototype {title, scene, state?, for?}` puts a scene under the artifact of `title`, and a call for a scene already shown is its next revision.
After the first call the agent edits the files and does not call again.

| Field | Meaning |
|---|---|
| `title` | The artifact's title, at most 120 characters |
| `scene` | A plain file name such as `ready.html`, at most 40 characters |
| `state` | The state to step him to; the scene's first where left out |
| `for` | One sentence on what it is for, at most 200 characters |

A call is refused with every broken rule named, and nothing is written.

| Refused for | Limit |
|---|---|
| A `scene` that is blank, a path, holds `..` or starts with a dot | |
| A scene or fixture that is missing, not text, a symlink or a directory | |
| A scene file | 64 KiB |
| The scene and the fixtures it names, together | 256 KiB |
| A fixture `src` that is a path | |
| A `state` | 80 characters |

The limits are `MOST_SCENE_BYTES` and `MOST_PROTOTYPE_BYTES` in `crates/ipc/src/prototype.rs`.
Fleet finds fixtures by a light scan of each `<fixture src>` and nothing more: the native app does the real parse.

## The vocabulary

**Rule.** A scene is one `<scene>` holding fixtures, states and parts, and each part is drawn by the part of the same name in `crates/bridge-native/src/parts/`.
A part outside every `<state>` draws in each state.
Words written directly inside a part draw as body text.

| Part | Attributes | Draws |
|---|---|---|
| `scene` | `title`, `first`, `walk` | The root; `first` names the state shown first, and `walk` the walk that shows the built screen, for the shot check |
| `fixture` | `name`, `src`, `type` | Nothing; binds a JSON file to `name` |
| `state` | `id`, `title`, `says` | Its children, while it is the state shown; `title` and `says` are its view tile in the head |
| `pane` | `title` | A titled region |
| `card` | `title`, `tone` | A card, its head in the tone |
| `row` | `title`, `says`, `glyph`, `tone` | A list row; its children are controls at the end |
| `chip` | `tone`, `glyph` | A status chip holding its words |
| `button` | `kind`, `glyph`, `off` | A button; `kind` is primary, secondary or ghost |
| `tabs`, `tab` | `active`, `goes` | A tab strip; a tab is picked when its `goes` is the state shown |
| `list` | `each`, `as`, `limit` | Its children, or a set of them for each item of an array |
| `text` | `style`, `tone` | A line; `style` is body, heading, title, muted, small or mono |
| `box` | the box attributes | A plain column with no look of its own |
| `glyph` | `name`, `tone`, `size` | An icon from `packages/icons/icons`; `size` is a space step |

### Box attributes

Any part takes these, and `tone` takes a tone word or a colour token.

| Attribute | Value |
|---|---|
| `dir` | `row`, or `col` |
| `pad` | One space step, or two as vertical then horizontal |
| `gap` | A space step |
| `radius` | `sm`, `md`, `lg`, `full` or points |
| `fill`, `edge` | A colour token or a tone word |
| `align` | `start`, `center`, `end` or `stretch` |
| `justify` | `start`, `center`, `end` or `between` |
| `grow` | Written bare: takes the room left |
| `w`, `h` | Points, a percentage or `full` |

A space step is `0` or a `--space-N` token in `packages/tokens/tokens.json`.
A colour token is named there without its leading `--`, such as `bg-raised` or `border-subtle`.
The tone words are neutral, accent, success, danger (or failed), running, waiting and warning.

### Views

**Rule.** Each `<state>` carries a `title`, a plain name for the view such as `One need`, and a `says`, one line on what that view shows.
The head draws a tile for each state, its `title` over its `says`, and a press, Enter or Space on a tile steps to it.
A state with no `title` shows its `id`.
Past five states, the band holds one View menu in place of the tiles, each state by its `title` over its `says`.
Why: he could not tell what a chip named `1-do-this-first` changed, or how two states that overlap differ.

**Rule.** A scene draws no tabs, buttons or links whose only job is to switch between its own states.
`goes` is for a press the built screen has, such as a row that opens its detail or a Back button.
Why: the head's view tiles already switch states, and a second row that does the same tells him two things do it.

### Binding, goes and data-part

**Rule.** `{name.path}` in a part's words or its `title`, `says`, `glyph` and `data-part` reads a fixture.
The name is a fixture's `name` or the `as` of an enclosing list, and the path is keys and array places joined by dots, such as `{jobs.sessions.0.title}`.
A binding that names nothing stays as written, and its reason is a trouble line.

**Rule.** `goes="id"` on any part makes a press, Enter or Space step to that state, and a `goes` that names no state is trouble.

**Rule.** `data-part="Name"` makes the part a labelled element, so VoiceOver reads it and a note names it.
A `pane`, `card` or `row` with no `data-part` takes its `title`, and a part with `goes` takes its words.

### Fixtures

A fixture's `type` is one of jobs, thread, needs-you, issues, preview-offers or json, which is the default.
The text must decode as the wire type that word names, so a fixture that has drifted from the wire fails in the scene and not on his screen.
`crates/bridge-native/fixtures/prototype-jobs.json` is a `jobs` fixture to copy a shape from.

### What a mistake does

**Rule.** A name the app does not know is never drawn as if it were right.
An unknown part draws as its own `<name>` in a magenta outline, and an unknown colour token draws magenta.
Each also adds one line to the red band under the head, with Fleet's own trouble lines: a missing fixture, a state the call named that the scene lacks, an unknown glyph, a `kind`, `style`, `dir`, `align`, `justify`, `pad`, `radius` or size it cannot read.
`crates/bridge-native/fixtures/prototype-trouble.html` holds one of each.

## An example

`jobs.json` beside the scene:

```json
{"sessions": [
  {"title": "Fix the merge line chip", "repo": "armada", "state": "waiting", "summary": "Asked which colour the stale state takes"},
  {"title": "Move Settings to a sheet", "repo": "armada", "state": "running", "summary": "Drawing the first take"}
]}
```

`$ARMADA_SCRATCH/scenes/jobs.html`:

```html
<scene title="Jobs list" first="list">
  <fixture name="jobs" src="jobs.json" type="json"></fixture>

  <state id="list" title="List" says="Two Jobs, one waiting on you">
    <pane title="Jobs" data-part="Jobs pane">
      <list each="jobs.sessions" as="job" limit="4" data-part="Jobs list">
        <row title="{job.title}" says="{job.repo}" glyph="circle-dot" tone="running" goes="detail" data-part="Job row">
          <chip tone="running" glyph="circle-dot">{job.state}</chip>
        </row>
      </list>
    </pane>
  </state>

  <state id="detail" title="One Job" says="The first Job opened, with its summary">
    <card title="{jobs.sessions.0.title}" tone="accent" data-part="Job card">
      <text style="muted" data-part="Job summary">{jobs.sessions.0.summary}</text>
      <box dir="row" gap="2">
        <button kind="primary" glyph="arrow-left" goes="list" data-part="Back button">Back to list</button>
      </box>
    </card>
  </state>
</scene>
```

The mock sample, with its fixture, is `crates/bridge-native/fixtures/prototype-scene.html`.

## Shot check before an offer

**Rule.** A Job that built the screen of a scene the owner approved runs the shot check before `offer_preview`, and offers the branch only when every state matches.
Why: a build that has drifted from the approved scene reaches the Job first, and not the owner on his preview.

Three things pair the scene with the build:

1. The scene names the walk that shows the built screen: `<scene title="Jobs list" walk="jobs-list" first="list">`.
2. The walk has a step for each state, and the step carries the state's id: `step(…).id("list")` in the walk's source (`Step::id` in `crates/bridge-native/src/walk/mod.rs`).
   Most steps carry none.
3. The Job runs `armada check crates/bridge-native:shot_check <scene>`, where `<scene>` is the file in `$ARMADA_SCRATCH/scenes` without `.html`.
   The bare Check runs every scene there that names a walk, and passes where none does, so it is also what the pre-PR run asks for.

The command underneath is `bridge-native --shot-check <scene.html> --out <dir> [--fixtures-dir <dir>]`.
It plays the walk headless on its mock data, draws each state of the scene alone, and sets each state against the walk's step of that id.
Both are drawn at the size the walk gives its screen, at the same scale, and the walk's picture is cut to the screen without its own bar.
It writes `report.json` and, for each state, `<state>-side-by-side.png` (the scene, then the build) and `<state>-difference.png` to `--out`.

| State | Meaning | Fails the check |
|---|---|---|
| matched | The two pictures are within tolerance | No |
| drifted | Past tolerance, with its score | Yes |
| missing | No step of the walk carries the state's id | Yes |

A scene that names no walk, or one this build does not play, stops the check with the reason and no report.

**Rule.** The comparison averages each picture down to a grid 32 cells across, and a pair has drifted when the mean difference passes 0.003 or more than 0.5% of the cells differ by over 0.02.
Why: antialiasing and a shift of a point or two wash out, and a changed tone, a moved part or a missing one does not.
The figures are measured: [the scene draw time spike](../spikes/scene-draw-time.md), *Shot check tolerance*.
It can miss a change smaller than a cell where nothing around it moves, such as one glyph changing tone, so the difference picture is what to read.

What a Job does on a drift:

- Read the difference picture, then fix the build to match the scene.
- Where the scene was wrong, that is the owner's to change: say so in the offer, and do not edit an approved scene to make the check pass.

`--against <other.html>` draws another scene's states in the walk's place.
It is how the tolerance was measured, and how a revised scene is set against the approved one.

## Shared parts

**Rule.** The parts live in `crates/bridge-native/src/parts/`, and a part an agent composes from boxes and the owner approves becomes a named part when it is built.
Today's native screens do not use the shared parts yet: only the scene reader draws from them.
