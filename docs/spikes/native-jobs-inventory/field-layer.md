# Native Jobs inventory: the Job view's Field layer

This is what the Electron and React Bridge draws today for the Field layer, so the native rebuild can match it.
It records what is, and proposes nothing.
The Jobs list, Needs you, the review sheet, the palette and notifications are in `list-needs-palette.md`; the Job view's head, thread and composer are in `job-view.md`.

Paths are relative to the repository root.
`SV` is `apps/desktop/src/renderer/src/session-view`.
`DK` is `apps/desktop/src/renderer/src/cockpit/deck`.
Pixel values are the Default density set, resolved from the token named beside them (the density table is at the top of `list-needs-palette.md`).
"Measured" values come from Bridge on the mock Fleet at 1440x900, walk `fieldExpandPlan` (`http://localhost:40024/?walk=fieldExpandPlan`), with the Job `Rotate the audit log` open; screenshots are under `.playwright-mcp/` (`field-panel.png`, `field-layer.png`, `field-layer-replay.png`).

## 0. What the Field layer is, and how it opens and closes

The Field layer is the Field map drawn large, as a layer over the Job's stage, with the Job's plan along its foot and a replay slider under that.
It is a sibling of the Job's thread, drawn after it, so the thread and composer stay mounted under it.

| Fact | Where |
|---|---|
| `open === "field"` draws `FieldLayer`; the other layers go through `Layer`, and the Field and the document draw themselves | `SV/SessionWorkspace.tsx:118-122` |
| `large` is placed after `children` in every arrangement of the workspace (paned, no story, story) | `SV/SessionWorkspace.tsx:455`, `:467`, `:473`, `:503` |
| Which layer is open is one value for the whole window, `{session, layer, from, page?}`; it belongs to the Job it was opened over and is dropped when that Job goes | `SV/layers.ts:20-100`, `SV/SessionWorkspace.tsx:123-128` (`dropLayer`) |
| `toggleLayer(id, "field")`: the same call again closes it | `SV/layers.ts:55-65` |
| Field has no pane tab: `TAB_OF` has no `field` entry, so in a Job drawn in panes the layer still opens over the whole Job | `apps/desktop/src/renderer/src/job-panes/focus.ts:~24-33` |
| Opened by the Expand button on the Field panel's band | `SV/FieldPanel.tsx:746-758` |
| Opened by `e` (registry `deck_field`, scope `deck`, verb `Expand the Field`, glyph `maximize-2`), only while a Job is open on the stage and a `[data-area="field"]` element is in the page | `crates/core-model/domain/actions.toml` (`[actions.deck_field]`), `DK/DeckHost.tsx:465-472` |
| Closed by `Esc` (window listener, ignored when a `[role="dialog"]` or `[role="alertdialog"]` is open), by the Back button, by `e` again, or by the panel's Collapse button | `SV/FieldLayer.tsx:21-35`, `:42-46` |
| After closing, focus goes back to the element that held it before the first layer opened, one tick later | `SV/layers.ts:closeLayer`, `from` taken by `focused()` |
| While the layer is open, the panel in the left column draws no canvas, only an empty `.sv-field.sv-field--plan`, so one canvas runs at a time; its Expand button reads Collapse | `SV/FieldPanel.tsx:775-780`, `:741` |
| On open, focus goes to the map (`.bf`), whose own keys pan and zoom it | `SV/FieldLayer.tsx:36-37` |

Measured: pressing Expand opens the layer; `Esc` closes it and focus lands on the Expand button again.

BuildCard (`DK/BuildCard.tsx`) is out of scope (it goes with the Cockpit map after switch day).
It draws the same `bf-build__plan` and `bf-build__seg` markup itself (`BuildCard.tsx:31-35`); it does not use `PlanBar`, whose only caller is `FieldPanel.tsx:687`.

## 1. The Field panel (the opener) and the small Field

The panel is where the Field normally sits; the layer is its large form.

| Fact | Where |
|---|---|
| In the Cockpit's lower left, under the Jobs list; with no Job open it holds the Cockpit's own map | `SV/FieldDock.tsx` |
| One panel of `SvPanel`: a band over a body; `label="Field"`, `area="field"`, `place` `left` | `SV/FieldPanel.tsx:759-781`, `SV/SvPanel.tsx` |
| Band state: `live` while any drone is working (`workingDrones` non-empty), else none | `SV/FieldPanel.tsx:739-745` |
| Band keycap `⌥F` (fold key), shown in the label's tooltip `Fold Field ⌥F` | `SV/story.ts` (`bandKeyFor("field")`, `BAND_KEYS`), measured |
| Before any work (no drones working) the body starts with Facts: `Bay` (a slot number), `Branch` (name) and `Model` (resolved model, else model), each only where held | `SV/FieldPanel.tsx:701-726` |
| While a turn is live, a `TurnAccountBlock` stands over the map | `SV/FieldPanel.tsx:770-774` |
| Panel size in the left column: `flex: 0.9 1 0`, `min-height: calc(var(--space-12) * 4)` (192px); body padding `--space-2` | `SV/flight.css:281-290` |
| Panel size in the Job's right column: `flex: 2 1 0`, `min-height: calc(var(--space-12) * 3)` (144px) | `SV/story.css:10-13` |

Measured (left column, Default): panel `x 12, y 496.6, 288 x 391.4`; band `286 x 24` (padding `0 4px 0 12px`; label 12px, weight 600, tracking 1.44px); lamp 6px at `x 25`, fill `rgb(79, 184, 217)`, aria-label `Live`; body `286 x 364.4`, padding 8px; field `270 x 348.4`; map `270 x 323.4`; replay row `270 x 25`; no plan bar (the plan bar is the layer's only).
Controls in the small Field: `Expand` (24px square at the band's end), `Zoom in`, `Zoom out`, `Back to this Job` (28px round buttons).
The small Field floats its cards over the app while the pointer or focus is inside it (`useEngaged(!large)`, `SV/field-engaged.ts`); the large one does not, because its cards already draw at full size.

## 2. Layout of the layer

```
.deck-pane                                     (position: relative)
  section.sv-layer [aria-label="Field"]        (absolute, inset 0, z-index 3, --bg-raised)
    header.sv-layer__head
       h3.sv-layer__title  "Field"      |  Back button (X, 16px glyph, tooltip "Back" + Esc)
    div.sv-layer__body.sv-fieldlayer [role=group, aria-label "Expanded Field"]
      div.sv-field.sv-field--plan [data-large]   (flex column)
        div.sv-field__map                        (flex 1; relative)
           ReplayMark                            (only while scrubbed back)
           .bf  (Battlefield)
              canvas, grain, haze, lens wires + card, hover callout, zoom column
        PlanBar [role=group aria-label "Plan", data-titled]   (only if the plan has groups)
        .sv-fieldreplay [role=group aria-label "Replay"]      (only if the Job has reads or writes in the window)
           .deck-replay__track [role=slider]
```

| Region | Size and type | File:line |
|---|---|---|
| Layer | `position: absolute; inset: 0; z-index: 3; display: flex; flex-direction: column; background: var(--bg-raised)`; it covers the Job's pane: measured `x 306, y 56, 1122 x 832`, fill `rgb(22, 28, 35)` | `SV/session-view.css:940-948` |
| Head | flex, centred, gap `--space-3` 12px, `min-height: var(--h-row-header)` 28px (measured 24 + 1px rule = 25px at the mock's density), padding `0 --space-3 0 --space-4` (0 12px 0 16px), bottom rule 1px `--border-subtle`; Back is pushed to the end (`margin-inline-start: auto` on its tooltip wrapper) | `SV/session-view.css:949-971` |
| Title "Field" | `--font-mono`, `--text-xs` 13px / `--leading-xs` 18px, weight 600, tracking `--tracking-caps` 0.04em (0.52px), uppercase, `--fg-muted` (`rgb(179, 191, 206)`) | `SV/session-view.css:958-967` |
| Back | ghost, small, icon-only button 24px square; glyph `X` 16px; tooltip `Back`, shortcut `Esc`; measured `x 1392, y 56, 24 x 24` | `SV/FieldLayer.tsx:42-46` |
| Body | `.sv-layer__body.sv-fieldlayer`: `gap: 0; padding: 0; overflow: hidden` (the base `.sv-layer__body` is flex 1, column, gap `--space-4`, padding `--space-3 --space-4 --space-4`, scrolls; this overrides it); measured `x 306, y 81, 1122 x 807` | `SV/field-plan.css:48-52`, `SV/session-view.css:990-999` |
| Field | `.sv-field`: `position: relative; display: flex; flex: 1 1 0; min-height: 0`; with `--plan` it is a column | `SV/session-view.css:134-139`, `SV/field-plan.css:3-5` |
| Map | `.sv-field__map`: `position: relative; display: flex; flex: 1 1 0; min-width: 0; min-height: 0`; measured `x 306, y 81, 1122 x 729` | `SV/field-plan.css:6-12` |
| Battlefield `.bf` | `position: relative; flex: 1 1 auto; overflow: hidden; cursor: grab; touch-action: none; overscroll-behavior: contain`; ground is a radial gradient at 50% 40% (85% x 75% ellipse) from `--accent` at 7% over `--bg-base` to `--bg-base` at 78%; `tabindex="-1"`; fills the map | `DK/battlefield.css:5-25` |
| Plan bar | `.sv-fieldplan[data-titled]`: flex, gap `--space-3` 12px, padding `--space-3 --space-4` (12px 16px), top rule 1px `--border-default`, fill `--bg-raised`; measured `y 810, 1122 x 53` (12 + 6 bar + 4 gap + 18 title + 12 + 1 rule) | `SV/field-plan.css:13-23` |
| Replay row | `.sv-fieldreplay`: flex, padding `0 --space-3` (0 12px), top rule 1px `--border-default`, fill `--bg-raised`; measured `y 863, 1122 x 25`; the track inside is 24px (`--space-6`) high | `SV/field-plan.css:54-60`, `DK/deck.css:2274-2281` |

The base `.deck-pane` is `position: relative` (`SV/session-view.css:944-946`); that is what the layer's `inset: 0` fills.
In a Job drawn in panes the layer's `data-in-pane` rule (`position: static`) is for the other layers; the Field layer draws itself and carries no such attribute (`job-panes/job-panes.css:451`).

## 3. Controls

| Control | Label / tooltip | Glyph | Action | Shown when |
|---|---|---|---|---|
| Back | aria `Back`; tooltip `Back` with `Esc` | `X` 16px | `closeLayer()` | always |
| Expand / Collapse (panel) | aria and tooltip `Expand` or `Collapse`, shortcut from `boundKeyFor("deck_field")` (`e`) | `Maximize2` 16px | `toggleLayer(id, "field")` | the panel's band; `Collapse` while the layer is open |
| Zoom in | tooltip and aria `Zoom in`, shortcut `+` | `Plus` 16px | zoom ×1.25 (`ZOOM_STEP`) about the pane's middle, eased | always; disabled at the most-zoomed limit |
| Zoom out | `Zoom out`, shortcut `-` | `Minus` 16px | zoom ÷1.25 | always; disabled at the least-zoomed limit |
| Locate | `Locate the selection` (a drone is selected), `Back to this Job` (the Field frames the Job's own drones), else `Back to the whole field`; shortcut `Home` | `Crosshair` 16px | `toSelection` | always |
| Plan segment | tooltip `<task text> · <state word>`, aria `<task text>, <state word>` | none | none (focusable only) | one per task |
| Slider | `role="slider"`, aria `When in the Job`, `aria-valuetext` `Live` or `<n> ago` | track, ticks, head | press or drag sets the head; keys step | the Job has reads or writes in the window |
| Replay mark | tooltip `Replay, not live`; `role="status"`, aria `Replay`, text `Replay` | `History` 16px | none | the slider is off the live end |
| Slider tick | tooltip = the event's `title` and `says` joined by a space | none | none | one per read or write in the window |
| Drone (another Job's) | hover card; a press opens that Job on the stage (a subagent's drone opens the Job that sent it) | canvas | `select({kind: "session", key})` | faded drones of other live Jobs in the repository; this Job's own drones open nothing |
| Lens card | `role="group"`, aria `Lens: <file basename>` | none | the card types out its newest hunk | a drone of the Job is writing a file and Fleet answered its change |

Zoom column (`.bf-zoom`): absolute, `top` and `right` `--space-3` 12px, `z-index: 1`, flex column with wrap-reverse, gap `--space-2` 8px, `max-height: calc(100% - var(--space-3) * 2)`; each button is round (`border-radius: 50%`), 1px `--border-glass`, fill `--bg-glass`, inset 1px `--border-highlight` plus `--shadow-card` (`DK/battlefield.css:245-263`).
Measured: three 28px buttons at `x 1388`, `y 93`, `129`, `165` (the first sits at the map's top, 12px in).
Mouse: drag pans; wheel and a ctrl-wheel pinch zoom about the pointer; a double press on the empty ground goes home (`DK/useCamera.ts:144-200`, `:265-268`).
Pressing the empty ground clears a pinned drone (`onClear`).

## 4. What the map draws

The Field passes the Cockpit's Battlefield (`DK/Battlefield.tsx`, a Pixi canvas on a tilted plane, with an invisible DOM layer for presses and focus) a single repository whose floor is every area Fleet names.
The Job's own plan scope and its drones' paths are outlined; everything is read from what Fleet recorded, never from what an agent said (`SV/FieldPanel.tsx:1-14`).

| Thing drawn | Source | Behaviour | Code |
|---|---|---|---|
| Floor (plots) | repository areas: `dir`, `kids` `[name, files, names?]` | one plot per directory with no kids, else per kid; shape never changes as the Job moves | `DK/areas.ts`, `DK/layout.ts`, `DK/pixi/ground.ts`, `terrain.ts`, `tiles.ts` |
| Touched plots | plan `scope` plus each live drone's `path`, matched to plots | outlined in the accent | `SV/field.ts:touchedPlots` |
| Planned plots | the plan's groups, `plotsOf(plan, plots)` | each named plot carries `{piece, said}` = the group titles joined by `, ` then ` · <piece word>` | `SV/field-plan.ts:plotsOf` |
| Files | `work.files` (`path`, `act`), plus every file with a glow, on the route, under a drone, or blamed by a light | a bar on the plot of its directory | `SV/FieldPanel.tsx:294-336` |
| Glow | read and write bursts from the timeline | fades over 10 minutes after a write and 5 after a read (`GLOW_MS`); redraws every 15s (`GLOW_STEP`) | `DK/afterglow.ts:33-36`, `SV/FieldPanel.tsx:72` |
| Own drone | `work.field` touch with no `agent`, else a resting drone at the strip | stands at the file of its newer last touch or burst; off the floor while not coding: on the pad while a check runs or a walk is open, risen over its file while it thinks, beaming to an outpost while it calls | `SV/field-now.ts`, `SV/field-drones.ts`, `DK/off-floor.ts` |
| Subagent drones | `work.field` entries with `agent`, joined to the ledger's `subagent` attachments | only those at work; a lift-off when one starts, a return when one ends, each once (4s, `PLAY_MS`); a running subagent with no touch rests on the strip | `SV/field.ts:workingDrones`, `SV/field-now.ts` |
| Other Jobs' drones | every other live Job in the repository (`dead`, `replacedBy`, `waits` all unset, same repository) | drawn faded, no beam, label or action; a press opens that Job; un-fades while it messages or is messaged by this Job | `SV/FieldPanel.tsx:388-405` |
| Message arcs | `session.messaged` timeline entries (`subject` is the sender, `to` the receiver) | an arc between the two drones with packets travelling from the sender | `SV/field-now.ts:talkOf`, `DK/off-floor.ts:messagesOf`, `DK/pixi/arcs.ts` |
| Outposts | `session.called` entries | one per service at the far edge, lit while a working Job's latest call is fresh | `DK/outposts.ts` (glyphs `forge`, `ticket`, `globe`, `plug`; `MOST_CALLS` 5) |
| Lights and walk | the Job's checks and open walk | the pad lights | `SV/field-now.ts:useMainNow` |
| Route | the Job's reads and writes up to the scrub time | a numbered route (`MOST_STOPS` 12), only while scrubbed back | `DK/afterglow.ts:routeAt` |
| Lens slabs | files a drone of the Job is writing, with Fleet's change for each | the file lifts as a slab with its minimap; counts at the slab | `SV/field-lens.ts`, `DK/lens.ts`, `DK/LensWires.tsx`, `DK/pixi/lens.ts` |
| Hover card (callout) | the thing hovered or focused | a card with label, title, hue, state, what it is doing (`Reading`, `Writing`, `Running`), the directories it touches (the biggest two), its bay, its last act and age; Esc puts it away | `DK/tip.ts`, `DK/TipLayer.tsx`, `DK/Battlefield.tsx:644-653` |
| Clash counts | `useClashesOf` per drone | marks on the canvas | `DK/Battlefield.tsx:ClashCount` |

Scenes the Field never fills: `pads`, `goals`, `prs`, `trails`, `facts`, `calls`, `clashes` are passed empty, so no bay pads with names, no flag, no pull request crates, and no off-pane call badges (`.bf-edge`) draw on the Field (`SV/FieldPanel.tsx:646-685`).
The canvas pixels (the plane's projection, plot art, drone sprites, glow colours) are drawn by `DK/pixi/*.ts` and were not inventoried pixel by pixel.

## 5. The plan bar (`PlanBar`)

`SV/FieldPlan.tsx:12`: one segment per task, grouped by the plan's groups, label `Plan`.

```
.sv-fieldplan  [role=group aria-label "Plan"]  (flex row, gap 12px)
  .sv-fieldplan__group [role=group aria-label <group title>]   (flex: <task count> 1 0, column, gap 4px, min-width 16px)
     ul.bf-build__plan   (flex row, gap 2px)
        li.bf-build__seg [data-piece] [tabindex 0 when titled] [aria-label "<task>, <state>"]   (6px high)
     span.sv-fieldplan__title  <group title>   (only when titled; the Field large is titled)
```

| Part | Value | File:line |
|---|---|---|
| Bar | flex, gap `--space-3` 12px; padding `--space-2 --space-3` (8px 12px) in the small form, `--space-3 --space-4` (12px 16px) when titled; top rule 1px `--border-default`; fill `--bg-raised` | `SV/field-plan.css:13-23` |
| Group | `flex: var(--tasks) 1 0` where `--tasks` is its task count (a group's width is proportional to its tasks); column; gap `--space-1` 4px; `min-width: --space-4` 16px | `SV/field-plan.css:24-30`, `SV/FieldPlan.tsx:21-25` |
| Task row | `.bf-build__plan`: flex, gap 2px (literal), no margin, padding or marker | `DK/build.css:26-32` |
| Segment | `flex: 1 1 0; min-width: 3px; height: 6px; box-sizing: border-box; border-radius: 2px` (literals); default fill `--fg-subtle` at 30% | `DK/build.css:33-40` |
| Group title | `--font-mono`, `--text-2xs` 12px / `--leading-xs` 18px, `--fg-subtle`, one line with ellipsis | `SV/field-plan.css:38-46` |
| Focus | a segment's focus ring is `1px --accent` outline, offset 1px (segments are focusable only when titled) | `SV/field-plan.css:34-37` |

Pieces and their fills (`data-piece`), with the state word the tooltip and aria name use (`PIECE_SAID`, `SV/field-plan.ts:14-20`):

| Piece | Word | Fill | Measured | File:line |
|---|---|---|---|---|
| `solid` | `Done, edits seen` | `--fg-default` at 70% | `color(srgb 0.894 0.914 0.937 / 0.7)` | `DK/build.css:41-43` |
| `hollow` | `Done, no edits seen` | transparent with a 1px `--fg-default` border | border `1px solid rgb(228, 233, 239)` | `DK/build.css:44-47` |
| `scaffold` | `Working` | `--status-escalated` | `rgb(238, 132, 80)` | `DK/build.css:48-50` |
| `ghost` | `Open` | `--fg-subtle` at 30% (the base) | `color(srgb 0.580 0.639 0.714 / 0.3)` | `DK/build.css:39` |
| `failed` | `Failed` | `--status-completed-failed` | not in the mock | `SV/field-plan.css:31-33` |

How a task becomes a piece (`SV/field-plan.ts`):

| Task `state` | `seen` | Piece |
|---|---|---|
| `failed` | any | `failed` |
| `done` or `handed_in` | `true` | `solid` |
| `done` or `handed_in` | not `true` | `hollow` |
| `open` | any | `ghost` |
| `working` | any | `scaffold` |

A group's own piece (`pieceOfTasks`): `failed` if any task failed; else, if all are done, `solid` when any was seen and otherwise `hollow`; else `scaffold` if any task is not `open`; else `ghost`.
A group with no tasks is not drawn (`groupsOf`).
No count is drawn beside the items it counts.
Where the bar is absent: the small Field (`!large`), and any Job whose plan has no group with a task (`FieldPanel.tsx:687`).

Measured on the mock (five groups, eleven tasks): groups at `x 322, 618.2, 819.6, 1115.8, 1317.3` with widths `284.2, 189.5, 284.2, 189.5, 94.7` (flex 3, 2, 3, 2, 1); each group 28px high; segments 6px high; titles `Rotate at midnight`, `Report the rotation`, `Show the rotation`, `Prune old files`, `Note it in the practice`; segment words in order `solid x4, scaffold x2, ghost x4, hollow x1`.
A segment's aria names, from the mock: `Rotate the log file at midnight, Done, edits seen`, `Mirror it in the TypeScript types, Working`, `Open a rotated file from the list, Open`, `Add the rotation to the audit doc, Done, no edits seen`.

## 6. The replay slider

`FieldReplay` (`SV/FieldPanel.tsx:506-553`) wraps `ReplayTrack` (`DK/ReplayBar.tsx:67-170`).

| Part | Value | File:line |
|---|---|---|
| Group | `role="group"`, aria `Replay` | `SV/FieldPanel.tsx:540` |
| Track | `role="slider"`, aria `When in the Job`, min `from`, max `end`, now `at`, text `Live` or `agoOf(at, end)` (for example `4m ago`); `tabIndex 0` | `SV/FieldPanel.tsx:541-550`, `DK/ReplayBar.tsx:126-142` |
| Range | the Job's own reads and writes in the last two hours (`WINDOW_MS`), at most 2000 (`MOST_EVENTS`); `from` is the first, `end` is `max(clock, newest)`; absent when there are none | `DK/replay.ts:29-30`, `:46-49` |
| Track box | `position: relative; flex: 1 1 0; height: var(--space-6)` 24px; `cursor: pointer; touch-action: none`; the rail is 2px (`--border-width * 2`) at 14% `--fg-default` | `DK/deck.css:2274-2292` |
| Fill | 2px, `--accent`, width tracks the head, transition `calc(var(--duration-sheet) / 2)` 110ms linear | `DK/deck.css:2293-2302` |
| Tick | 2px wide, `--space-3` 12px high, `--fg-subtle`; `--accent` once passed (`data-passed`); one per event; tooltip = title and says | `DK/deck.css:2303-2314` |
| Head | 12px circle, `--accent`, `box-shadow: 0 0 var(--space-3) var(--accent)` (12px glow, literal), `left` transitions 110ms linear; `pointer-events: none` | `DK/deck.css:2315-2327` |
| Focus | `--focus-ring`, offset inward by `--focus-ring-offset` | `SV/field-plan.css:61-64` |
| Reduced motion | fill and head transitions are `none` | `DK/deck.css:2328-2331` |

Behaviour: a press puts the head there and a drag moves it, at most once a frame; letting go within the last 2% of the track (`LIVE_END`) is Live again (`at` becomes undefined); the keys step from the current time (`SV/FieldPanel.tsx:520-538`).

| Key on the slider | Does |
|---|---|
| `ArrowLeft` | the previous read or write before the head (`neighbours.before`, else `from`) |
| `ArrowRight` | the next after the head (else `end`, which is live) |
| `Home` | `from` |
| `End` | `end`, live |

Replay mark (`ReplayMark`, `DK/ReplayBar.tsx:57-66`, `DK/deck.css:2189-2243`): absolute, `z-index: 4`, `top` and `left` `--space-3` 12px, flex, gap 12px, padding `--space-1 --space-4 --space-1 --space-3` (4px 16px 4px 12px), radius `calc(var(--radius-lg) * 2)` 16px, fill `--bg-base`, 1px `--border-glass`, inset highlight and `--shadow-card`, `--text-xs` 13px / 18px, glyph `History` 16px, text `Replay`.
Measured while scrubbed: `x 318, y 93, 109 x 28`, fill `rgb(15, 20, 25)`, border `1px solid rgba(255, 255, 255, 0.07)`, ink `rgb(74, 158, 219)` (the accent); slider `aria-valuetext` `4m ago`; head at `x 312`, `12 x 12`.

What scrubbing changes (`useJobField`, `SV/FieldPanel.tsx:161-503`): the clock becomes the scrub time (`now = at ?? clock`); the Job's drone stands at the file of its last read or write at or before that time (`droneAt`), its subagents are left off, and the route (`routeAt`) is drawn; messages are dropped; the lens is not asked for (`active: at === undefined && shown`); the glow is the glow as it stood.

## 7. The lens card

`FieldLensCard` (`SV/FieldLensCard.tsx`) is a DOM card over the map at the top right: the file a drone of the Job is writing, its counts, and the newest hunk of its change, with the earlier files behind it as faded tabs.
It was not shown on the mock Fleet (the mock does not answer `get_session_file_change`), so its sizes are from CSS.

| Part | Value | File:line |
|---|---|---|
| Position | `.bf-lens__card`: absolute, `top: --space-3` 12px, `right: calc(var(--space-12) + var(--space-3))` 60px, `width: min(calc(var(--space-12) * 6), calc(100% - var(--space-12) * 2))` = 288px (the small Field: `min(calc(var(--space-12) * 4), 50%)` = 192px) | `DK/battlefield.css:~363-368`, `SV/field-plan.css:67-69` |
| Left side | `data-side="left"`: `left: --space-3`, `right: auto`, where a hand has put the card's drone or slab under the card | `DK/battlefield.css:~370` |
| Wire | a dashed line (`3 3`, 1.5px, `--accent`, 80% opacity) from the card to its slab | `DK/battlefield.css:~355-361` |
| Card | `.sv-lens__card`: 1px `--accent` border, radius `--radius-md` 6px, fill `--bg-glass`, `--shadow-card` | `SV/field-plan.css:77-86` |
| Tabs | `.sv-lens__tab`: absolute, `height: calc(var(--space-12) * 2)` 96px, padding `0 --space-2`, `--font-mono` 12px / 16px, `--fg-subtle`; `data-k="1"` shifts right `--space-2` at 75% opacity; `data-k="2"` shifts `--space-4` at 45%; the stack's top padding is `calc(var(--leading-2xs) * var(--tabs))` | `SV/field-plan.css:70-103` |
| Head | flex, baseline, space-between, gap `--space-2`, padding `--space-1 --space-2`, mono 12px / 16px; name `--fg-default` with ellipsis; counts `--fg-muted` | `SV/field-plan.css:104-127` |
| Lines | mono 12px / 16px; each line flex, gap `--space-1`, padding `0 --space-2`, no wrap; `context` `--diff-context`, `added` fill `--diff-add-bg` ink `--diff-add-fg`, `removed` `--diff-del-bg` and `--diff-del-fg`; the sign column is `+`, `-` or a space | `SV/field-plan.css:128-165`, `SV/FieldLensCard.tsx:21-25` |
| Caret | 4px wide (`--space-1`), 16px high (`--leading-2xs`), `--accent`, drawn while a line is typing | `SV/field-plan.css:166-172` |
| Cut note | `diff.cut` text, `--fg-subtle`, 12px / 16px | `SV/field-plan.css:173-178` |
| Lines held | the newest 6 lines (`LINES`); 14 in the Field large (`LINES_LARGE`) | `SV/FieldLensCard.tsx:14-16` |
| Typing | only lines new since the last read type out, at 160 characters a second but never longer than 1200ms (`CHARS_PER_S`, `REVEAL_MS`), redrawn every 33ms; whole at once under reduced motion; while it types the map is told (`typing`) | `SV/FieldLensCard.tsx:17-70` |
| Floated | in the small Field while the pointer or focus is in it: `position: fixed`, `z-index: var(--z-menu)`, card `calc(var(--space-12) * 6)` wide, standing beside the map toward the thread (`right: calc(100% + var(--space-3))`), or on the right when there is no room; fill `--bg-overlay`, radius `--radius-lg`, `--shadow-overlay`. Never in the large layer | `DK/battlefield.css:~376-397`, `SV/field-plan.css:~72-76`, `SV/field-engaged.ts` |
| Counts at slabs | `.bf-lens__counts li`: absolute, `translate(-50%, -100%)`, padding `0 --space-1`, 1px `--border-glass`, radius `--radius-sm`, fill `--bg-glass`, mono 12px / 16px, no wrap | `DK/battlefield.css:~410-424` |

The card follows the newest writer, or the drone picked in the Drones menu; the card never moves the camera.
A refused file draws no lens and says nothing.
The lens is asked for only while the Field is live, shown (`useShown`) and a drone is writing; the ask is `get_session_file_change` (`GET /sessions/file_change?session_id=&path=&seen=`) on a `session.edited` entry, never on a timer (`packages/protocol/src/territory.ts`, `SV/field-lens.ts`, `SV/lens-asks.ts`).

## 8. Keys

| Key | Scope | Does | Where |
|---|---|---|---|
| `e` (`deck_field`) | deck, with a Job open and `[data-area="field"]` in the page | opens or closes the layer | `DK/DeckHost.tsx:465-472` |
| `Esc` | anywhere, unless a dialog or alertdialog is open | closes the layer (and the layer root keeps Esc: `data-keeps-escape`) | `SV/FieldLayer.tsx:21-35` |
| `Esc` | the map | puts the hover card away (capture phase, skipped while a HUD is open) | `DK/Battlefield.tsx:644-653` |
| `⌥F` | the Field panel | folds or opens the panel (`bandKeyFor("field")`) | `SV/story.ts:243` |
| `ArrowLeft`, `ArrowRight`, `ArrowUp`, `ArrowDown` | the map (`.bf` focused; with ⌘, Ctrl or ⌥ ignored) | pan by `KEY_PAN` = 90 world units, three times that with `Shift`, eased | `DK/useCamera.ts:270-307`, `DK/camera.ts:29` |
| `+` or `=` | the map | zoom in ×1.25 | `DK/useCamera.ts:299` |
| `-` or `_` | the map | zoom out ÷1.25 | `DK/useCamera.ts:300` |
| `0` | the map | back to the fit | `DK/useCamera.ts:301` |
| `Home` | the map | locate: the selection, or the Job's drones, or the whole field | `DK/useCamera.ts:302` |
| `ArrowLeft`, `ArrowRight`, `Home`, `End` | the slider | step through reads and writes (section 6) | `SV/FieldPanel.tsx:522-538` |
| `Tab` | the layer | reaches the map (`tabindex -1` is only for script focus), the plan segments (focusable when titled), the slider and Back | measured: the segments and slider take focus |

The map's keys stop propagation, so none of them reaches the deck's own keys (`useCamera.ts:269`).

## 9. States

| State | Drawn as | Code |
|---|---|---|
| No plan, or no group with a task | no plan bar | `FieldPanel.tsx:687` |
| No reads or writes in the last two hours | no replay row | `FieldPanel.tsx:688` |
| Resting (no drone working) | the Job's own drone at rest on the strip (`RESTING`); the small panel shows Facts above the map | `FieldPanel.tsx:61-67`, `:739-769` |
| Live | the slider at its right end, `aria-valuetext` `Live`; the lens asked for; messages drawn | `FieldPanel.tsx:520-550` |
| Scrubbed back | the Replay mark, the route, the drone at the older file, no lens, no messages, no subagents | section 6 |
| Panel with the layer open | the panel body holds an empty `.sv-field` and no canvas | `FieldPanel.tsx:775-780` |
| Job changes | the slider returns to live and the pin is dropped for the new Job | `FieldPanel.tsx:583-588` |
| Pin | the drone picked in the Drones menu keeps the card on it and the camera stays; `setPinned` is only ever called to clear it in this checkout (no Drones menu is drawn by `FieldMap`) | `FieldPanel.tsx:580-585`, `:670` |
| Typing in the lens | the map is told (`typing`), so it holds still | `FieldPanel.tsx:635`, `:667` |
| Reduced motion | the lens hunk shows whole; fill and head transitions stop; subagent lift-offs and returns are skipped (the subagent appears and goes at once) | `FieldLensCard.tsx:27`, `field-now.ts` (`still`) |

## 10. Strings

`Field`, `Expanded Field`, `Back`, `Esc`, `Expand`, `Collapse`, `Plan`, `Replay`, `When in the Job`, `Live`, `<n> ago` (`agoOf`), `Replay, not live`, `Zoom in`, `Zoom out`, `Locate the selection`, `Back to this Job`, `Back to the whole field`, `<title> needs you` (not drawn on the Field), `Lens: <file>`, the plan words `Done, edits seen`, `Done, no edits seen`, `Working`, `Open`, `Failed`, `<task>, <word>`, `<task> · <word>`, group titles as the agent wrote them, panel `Facts`, `Bay`, `Branch`, `Model`, the band's `Live` lamp, `Fold Field`, `Reading`, `Writing`, `Running`.

## 11. Roles and names

| Element | Role and name |
|---|---|
| Layer | `section`, aria-label `Field`, `data-keeps-escape` |
| Layer body | `role="group"`, aria-label `Expanded Field` |
| Plan bar | `role="group"`, aria-label `Plan`; each group `role="group"` named by its title; each segment `li` named `<task>, <state>` |
| Slider group | `role="group"`, aria-label `Replay`; the track `role="slider"` named `When in the Job` |
| Replay mark | `role="status"`, aria-label `Replay` |
| Lens | `role="group"`, aria-label `Lens: <basename>` |
| Zoom, locate, Back, Expand | buttons named as in section 3 |
| Panel lamp | `role="img"`, aria-label `Live` |

## 12. Fleet data the layer reads

Session fields (`packages/screens/src/draft/sessions.ts:1098`, built from the record by `packages/screens/src/session-work.ts`):

| Field | Type | Used for |
|---|---|---|
| `id` | string | keys the layer, the slider, the events |
| `manifestId`, `cwd` | string, optional | which repository the Job works in (found by manifest id, else by `cwd`) |
| `work.plan` | `WorkPlan` `{version?, summary?, scope: string[], groups: WorkGroup[], approved, setAside?, changes?, sketch?, after?}` | the plan bar, planned plots, touched plots |
| `WorkGroup` | `{id, agent, title, gives?, paths: string[], issue?, landsIn?, ownJob?, tasks: WorkTask[]}` | one group of the bar; `title` is its label; `paths` name the plots |
| `WorkTask` | `{id, text, state: "open" | "working" | "handed_in" | "done" | "failed", seen?: true, changedAt?, marked?, stale?: {why, since}}` | one segment |
| `work.field` | `WorkDrone[]` `{id, agent?, task?, path, doing: "read" | "write" | "run", file?, at?}` | the drones |
| `work.files` | `WorkFile[]` `{path, act?: "write" | "edit" | "remove"}` | the file bars |
| `turn.state` | `"working"` and others | whether the Job's own drone is working |
| attachments of kind `subagent` | `{id, call?, state: "running" | "done" ..., agentType?, task?}` | which subagent drones are at work |
| attachments of kind `slot`, `branch` | `slot` number, `name` | the panel's Facts |
| `resolvedModel`, `model` | string | the panel's Facts |
| `live` | live-turn data | the panel's `TurnAccountBlock` |
| other Sessions' `id`, `title`, `dead`, `replacedBy`, `waits`, `manifestId`, `cwd`, `work.field` | | the faded drones and message labels |

Wire records they come from (`packages/protocol/src/`):

| Type | Fields | Where |
|---|---|---|
| `SessionRecord.touches` | `FileTouch[]`: `{by?: {agent, id}, path, act: "write" | "edit" | "remove", at}`; `by` absent means the Session's own agent | `sessions.ts:204`, `session-plan.ts:287-298`; read by `fieldOf` (`session-work.ts:279`) into `work.field` and `work.files` (a drone's `path` is the first two directories of its file) |
| `SessionPlan` | `{version, summary, recorded_at, approved, approved_at?, set_aside_at?, scope: string[], groups, changes?, sketch?, held_by?, held?}` | `session-plan.ts:139-162`; read by `planOf` (`session-work.ts:158`) |
| `TimelineEvent` | `{id, at, kind, subject, title?, tone, says, detail?, items?, to?}` with kinds `session.read`, `session.edited`, `session.ran`, `session.called`, `session.messaged` | `timeline.ts:100-122`; read through `useFleetEvents()` and filtered to this Job (`FieldPanel.tsx:94-158`); `session.messaged` also matches `to === session.id` |
| `RepositoryAreas` | `{repositories: [{manifest_id, root, head, areas: RepositoryArea[], omitted}]}`; `RepositoryArea` `{dir, files, kids?: AreaKid[], names?}`; `AreaKid` `{name, files, names?}`; at most 40 areas | `territory.ts:6-45`, route `list_repository_areas` (`GET /repository_areas`) |
| `SessionFileChange` | `{stamp, diff, cut?, unchanged?}`; diff cut at 2,000 lines or 256 KiB | `territory.ts` (`get_session_file_change`) |
| Repositories | `RepositorySummary` with `manifest`, `root` | `repositories` prop; the repository's name is `manifest.repository` or the last part of `root` |

`DK/areas.ts` turns `RepositoryArea` into `Area` `{dir, kids: Kid[], names?, files?}` with `Kid` `[name, files, names?]`.
The glow, route and drone position are computed client-side from the events (`afterglow.ts`, `off-floor.ts`, `field-now.ts`, `field-drones.ts`); nothing here is read from what an agent said.

QUESTION: The map's canvas (floor plates, drone sprites, glow colours, arcs, outposts, slabs) is drawn by `DK/pixi/*.ts` and is not inventoried pixel by pixel here, only what the Field passes in and which module draws each thing; confirm whether the native Field needs its own scene inventory, or only the layer, the plan bar, the slider, the lens card and the controls above.
QUESTION: In a Job drawn in panes, `toggleLayer` asks `focusLayer`, which returns false for `field` (no tab), so the layer opens over the whole Job; I measured only the story layout of the mock, not a paned Job, so the layer's box in the paned arrangement is not measured.
QUESTION: The mock Fleet did not serve a lens, so the lens card, its tabs and typing were read from CSS and `FieldLensCard.tsx`, not measured; confirm that is enough.
