# The open Job's view: inventory of today's Electron Bridge

Spec for the native build. Records what IS. Line numbers are against `native/jobs-screens` at the time of writing (slot-10). Paths are repo-relative.

**One tweak the owner named for every native screen (9 Nov): more room than Electron.** Padding and gaps are larger, controls are taller, and rows and controls have rounded corners where Electron's are square.
The values are the space, control-height and radius tokens Foundation's merged Jobs list uses, never literals, so the Job view matches the list.
Layout, controls, states and keys still follow this inventory.

**Which build is described.** The shipped Job view is a Job drawn **in panes**: `SessionView` (`apps/desktop/src/renderer/src/sessions.tsx:1703`) renders `SessionWorkspace` (`session-view/SessionWorkspace.tsx:88`) which, when `paned` (`:132`: `hosts !== undefined && hosts.zones === undefined && work !== undefined && draft !== undefined`), draws `JobPanes` with the Thread as one tab; `SessionFrame` is then `headless` (`sessions.tsx:2905-2975`, prop `headless={onTab}`) and the Job's head is the **Thread tab** in the pane's tab strip (`job-panes/job-head.ts:1-3`: "the Job's view publishes them and draws no head of its own; the tab is the head").
The thread is the spacious take, always on (`apps/desktop/src/renderer/src/main.tsx:14` sets `document.body.dataset.threadTake = "spacious"`; `sessions.tsx:1722`).
Where the non-paned path exists (Job without `work`, or `hosts.zones` set) the `SessionFrame` head row is drawn instead; it is recorded at 1.8.

## 0. Composition of the open Job (shipped, paned)

```
Title bar (TitleBar.tsx:625)  ... [ Layout control (PanelsTopLeft) ] ...        <- [data-job-slot], `.armada-title-bar__job`
+------------------------------------ JobPanes area (.job-panes__area) ----------------------------------------+
| pane A (default 62% wide)                                   | pane B (38%)                                          |
| +-- .job-pane__strip (tab strip, h >= --h-row-header) ----+ | +-- strip: Open | Done | Deferred  (+) ---------+ |
| | [mark][Job title][More ...][x] | [Add tab +]            | | |                                               | |
| +---------------------------------------------------------+ | +-----------------------------------------------+ |
| | .job-pane__body  (role=tabpanel)                        | | | body                                          | |
| |  SessionFrame (Card, headless)                          | | |                                               | |
| |   .armada-session-frame__body                           | | +-- pane B2 (below, 60%): Now -----------------+ |
| |    .armada-session-frame__centre (flex column,gap s-3)  | |                                                     |
| |      [ForkedFrom] [StartedBy] SessionThread (rows)      | |                                                     |
| |      CallChip                                           | |                                                     |
| |      .sv-dock                                           | |                                                     |
| |        .sv-bands (absolute, over the box's top edge):   | |                                                     |
| |          ToolsBand | NeedsYouPanel | HandoffBand |      | |                                                     |
| |          SessionEndBand | WaitingBand                   | |                                                     |
| |        .sv-start-strip (held Job only)                  | |                                                     |
| |        SessionComposer  (QuickBar on its top edge)      | |                                                     |
| |        SessionToolbelt (repository, Aa, place, Job acts)| |                                                     |
+---------------------------------------------------------------------------------------------------------------+
```

Default arrangement: `job-panes/store.ts:40-59` `defaultArrangement`: row split ratio 0.62; left pane = [Thread]; right = column split ratio 0.4 of [Open, Done, Deferred] over [Now].

### Tokens used by sections 1, 2 and 4 (values from `packages/tokens/tokens.css`; Default spacing set)

| Token | Value | Token | Value |
|---|---|---|---|
| `--space-1/2/3/4/6/8/12` | 4/8/12/16/24/32/48 px (`:605-611`) | `--radius-sm/md/lg` | 3/6/8 px (`:626-628`) |
| `--border-width` | 1px (`:623`) | `--h-badge` | 20px (`:660`) |
| `--h-row-header` | 28px (`:632`) | `--h-control-sm / --h-control / --h-control-lg` | 24 / 28 / 32 px (`:657-659`) |
| `--text-2xs/xs/sm/base` | 12/13/14/15 px; leading 16/18/20/22 (`:572-575`) | `--weight-medium / --weight-heading` | 500 / 600 (`:589-590`) |
| `--tracking-caps` | 0.04em (`:592`) | `--font-sans / --font-mono` | IBM Plex Sans / IBM Plex Mono (`:58-59`) |
| `--gap-panel` / `--deck-gutter` | 6px (`:669`) / `var(--gap-panel)` (`deck.css:11`) | `--focus-ring` | `1px solid var(--accent)`, offset 2px (`:1149-1150`) |
| `--duration-fast/base/sheet/pulse` | 120/180/220/1600 ms | `--palette-width / --sidebar-default` | 560 / 200 px |
| `--status-not-started / running / awaiting-review / completed-failed / handed-in` | #8C97A6 / #4FB8D9 / #E5A93D / #E97878 / #4FB4AA (`:147-263`); light theme differs (`:1211+`) | `--step-waiting / --step-advanced` | = awaiting-review / = completed-success |

Density: `data-spacing="compact"` makes `--space-1..12` 3/6/9/12/18/24/36 and control heights 24/20/28; `roomy` makes `--space-1` 5, `--space-2` 10 ...
(`tokens.css:1236-1275`).
Colour tokens (`--bg-raised`, `--bg-sunken`, `--bg-hover`, `--fg-default/muted/subtle/inverse`, `--border-default/subtle/glass/strong`, `--accent`) come from the active theme file under `packages/tokens/themes/`.

Spacious-take variables set on `.armada-session-frame` (`spacious-thread.css:4-22`): `--spacious-column: clamp(680px, 70cqi, 960px)`; `--spacious-bleed: calc(--space-4 + --space-1)` = 20px; `--spacious-outer: column + 2*bleed`; `--spacious-gutter: --space-4`; `--spacious-body: 15px`; `--spacious-leading: 24px`; `--spacious-meta: 13px`; `--spacious-radius: --radius-lg`; and `--radius-sm/md/lg` re-pinned to 3/6/8px inside the frame.

---

## 1. The Job's head

The head is spread over five places. There is **no plan progress bar and no title row** in the paned head; see 1.9 QUESTION.

### 1.1 The Thread tab as the Job's head

| | |
|---|---|
| Component | `job-panes/PaneView.tsx:223-530` `TabButton` (job branch `job = tab.kind === "thread" ? head : undefined`, `:244`); head data from `useJobHead` (`job-panes/job-head.ts:42`), published by `usePublishJobHead` in `sessions.tsx:2847-2873` |
| CSS | `job-panes/job-panes.css:152-262` (`.job-pane__tab`, `.job-pane__label`, `.job-pane__name`, `.job-pane__close`, `.job-pane__rename`) |
| Published | `{ state, said: "<said> · <address ?? id>", title?, onRename?, onNameForMe?, menu: {entries, onSelect} }`; published only when `inPanes(session.id)` (`sessions.tsx:2845`) |

**Layout (tab button, left to right):** `[state mark 16px] [Job title text, ellipsised]` inside one `<button role="tab">`, then `[More (…) 24px ghost]` (Job tab only), then `[Close (x)]`; both in `.job-pane__close` wrappers that keep their width always and are `visibility:hidden` unless the tab is active, hovered or has focus within (`job-panes.css:169-177`).
Tab label button: `display:inline-flex; align-items:center; gap:--space-2; min-height:--h-control-sm; padding:0 --space-1 0 --space-3; max-width:28ch; white-space:nowrap; overflow:hidden; text-overflow:ellipsis` (`:204-225`).
Type: `--font-mono`, `--text-xs`/`--leading-xs`, weight `--weight-heading`, `letter-spacing:--tracking-caps`, `text-transform:uppercase`, colour `--fg-muted`; hover `--fg-default` on `--bg-hover`; active tab colour `--fg-default`, transparent background (`:231-238`).
(So the Job's title is drawn UPPERCASE in mono; the tooltip carries the title as written.) Tab chrome: no radius, `border:0`; tab separators are inset 1px box-shadows (`:152-167`).

| State | How drawn | Condition |
|---|---|---|
| Active tab | tab ground `--bg-raised`, side hairlines `--edge`, label `--fg-default`, × shown | `pane.active === tab.id` (`PaneView.tsx:246`) |
| Inactive | on the strip ground `--bg-sunken`, label `--fg-muted`, right hairline `--border-subtle`, `tabIndex=-1` | else (`:431`) |
| Dragging | opacity 0.5 | `data-dragging` (`job-panes.css:178`) |
| Renaming | the name becomes `<input class="job-pane__rename" aria-label="Rename <label>">` in the label's place (field-sizing:content, min-width `--space-4`, same type, no box); mark, More and Close stay; Enter keeps, Esc/blur: Enter and blur save, Esc cancels (`PaneView.tsx:533-572`); a save only calls `onRename` if trimmed value non-empty and differs (`:306-312`) | `renaming` state; entered by double-click on the tab (only if `job.onRename` present, `:433-437`) or the menu's Rename (opened 60 ms after the menu shuts, `:327`) |
| Title absent | label text is the kind's label `Thread` (`labelOf`, `:67`) | `job.title === undefined` (`:245`) |

Mark = `SessionMark` (see 1.2).
Name tooltip (`Tooltip label={label}`) shows the whole title (`:454`).
Strings: aria-label of the tab button `"Thread"` or `"Thread: <title>"` (`:428`); More: tooltip + aria-label `"More"`, `aria-haspopup="menu"`, glyph `Ellipsis` 16px stroke 2; Close: tooltip `"Close"`, aria-label `` `Close ${label}` ``, glyph `X`.
Closing the Thread tab is allowed (the + menu then offers Thread again, `PaneView.tsx:594`).

**Name given by Fleet:** the title is `session.title`; absent until the first turn names it.
`onNameForMe` (menu item "Name it for me") has Fleet name the Job from its first message and returns when settled.

### 1.2 State mark and its tooltip

`SessionMark` (`packages/components/src/compositions/SessionFrame/SessionFrame.tsx:138`): `<span class="armada-session-mark" role="img" aria-label={said} data-bucket data-pulsing>` inside a Tooltip with the same `said`; 16px lucide glyph stroke 2; `working` draws `WorkingMark` (turning arc) instead of a glyph.
`limited` draws `LimitMark` (`BatteryLow`, bucket `waiting`).

Bucket colour (`SessionFrame.css:252-273`): needs `--status-awaiting-review` (only this pulses via `data-apparent` for the tools mark), working `--status-running` (inner dot breathes, `armada-badge-pulse`, `--duration-pulse`; off under reduced motion), waiting `--status-handed-in`, idle `--fg-muted`, draft `--status-not-started`.
`bucketOf` `SessionFrame.tsx:117`.

State -> glyph (`SessionFrame.tsx:87-108`) and the sentence (tooltip = `said + " · " + (address ?? id)`, `sessions.tsx:2851`).
State is computed in TypeScript by `jobStateOf` (`packages/screens/src/session-state.ts:103-266`), first match wins; `readSince` from `cockpit/viewed.ts`:

| # | Condition | state (glyph) | said |
|---|---|---|---|
| 1 | `paused && !dead` | paused (`CirclePause`) | `Paused` |
| 2 | `dead==="ended"` | ended (`Unplug`) | `Ended` |
| 3 | `dead==="quiet"` | quiet (`Unplug`) | `Not reachable` |
| 4 | `startFailed` | unstarted (`X`) | `Failed to start: <cause>` |
| 5 | `stopped` | stopped (`Square`) or, if `stopped.gaveUp`, stopped_asks (`OctagonAlert`) | `stoppedSaid` plus ` · <repair/restart said>` |
| 6 | `waits` | after (`Link`) | `startsWhenSaid(waits)` |
| 7 | `work.proposal.reading` | working | `Reading the request` |
| 8 | `start` | draft (`MessageSquareDashed`) | `Not started` |
| 9 | `limitedUntil` | limited | `Limit reached · resumes <time>` (`limitHeldSaid`) |
| 10 | `asked` | waiting (`Eye`) | `Waiting on you` |
| 11 | owes: slot_choice(not creating) / delivery / plan / task_done / sketch / document | waiting | `Waiting on you` |
| 12 | standing stuck | stuck (`Clock`) | `Stuck` |
| 13 | `turn.state==="working"` | working | `Working`, or `Woken by <title>` |
| 14 | standing working: subagent running -> subagent (`Split`) `Subagent working`; background cmd / Check running -> running (`SquareTerminal`) `Running: <task or check name>` (or waiting if owes) ; else working `Working` | | |
| 15 | blank | blank (`CircleDashed`) | `Blank: no slot, no branch` |
| 16 | `owes>0 || lapsed` | waiting | `Waiting on you` |
| 17 | `waitsOf(session)` non-empty | held (`Radar`) | the waits' `said` joined with ` · ` |
| 18 | standing replied and unread | replied (`Reply`) | `Unread reply` |
| 19 | standing done | done (`Power`) | `Done` |
| 20 | open PR with failed Checks | failing (`ShieldX`) | `Checks failed on #<n>` |
| 21 | else | idle (`Check`) | `Idle` |

Plus a separate mark when the mod is out of date: `ModMark` (`RotateCw`, tooltip `Mod out of date: run /reload-plugins`) replaces the state mark when state is `quiet && modOutOfDate` (non-panes head only).

### 1.3 The Thread tab's menu (More ... / right-click / Shift+F10 / Menu key)

Opened by: More button (anchored at its bottom-left), right-click anywhere on the tab (at the pointer), `ContextMenu` key or `Shift+F10` on the focused tab (anchored at the tab's bottom-left) (`PaneView.tsx:398-404, 438-447, 471-477`).
`DropdownMenu triggerLabel="Tab actions"`, portalled.

Order built by `headEntries` (`:175-221`): `[Rename] [Name it for me]` (each only if its handler exists) - rule - the Job's acts (below, up to its `end-rule`) - rule - the tab's own entries - then `End job` (the part after the Job's own `end-rule`; the rule itself is dropped).

Job's acts (`headMenu`, `sessions.tsx:2777-2850`, via `menuWithFresh` `SessionFrame.tsx:356`):

| Entry (label) | Glyph | id | Shown when | Does |
|---|---|---|---|---|
| `Stop` | `Square` | deck act `stop` | `subject.stoppable && acts.can.stop` (`cockpit/deck/panes.tsx:185`) | `acts.stop(subject)` |
| acts that the head has no room for (`actEntries`, `job-panes/acts.tsx:128`) | the act's glyph | `act:<id>` | measured by `JobActsSlot` (see 1.5) | runs the act; disabled with `hint` = why when off |
| rule | | `acts-rule` | any of the above | |
| `New Job` (hint `Starts with a short handoff from this one`; disabled while handing) | `MoveRight` | `fresh` | `fresh` defined: `dead===undefined && (context.tokens > 300_000 || hours since trail.started >= 24)` (`sessions.tsx:329-330, 2077-2098`) | writes a handoff, starts a new Job on it, opens it |
| `Fork` | `GitFork` | `fresh-fork` | `fresh.onFork` (needs `draft.forkAt` and a last row) | `forkAt(session, lastRow.id)` and opens it |
| `Fork` | `GitFork` | `fork` | `draft.fork !== undefined && session.dead !== undefined` (an ended Job only) | `draft.fork(session.id)`, opens it |
| `Retro` / `Writing the retro` (disabled) | `Rewind` | `retro` | `retro.press !== undefined` | `retro.press(session, true)` |
| `End job` | `PowerOff`, danger | `end` | `draft.close !== undefined && session.dead === undefined` | `askToEnd` (1.7) |

Tab's own entries (`tabEntries`, `PaneView.tsx:87-167`; for the Thread: no terminal items, not editable): `Move to window` (disabled with hint = reason when the kind has no window, `windows.ts`), `Zoom` / `Restore` (shortcut `boundKeyFor("pane_zoom")` = ⌥⌘Enter), `Split right` (⌘D), `Split down` (⌘⌥D), rule, `Close` (danger).

Rename and Name-it-for-me: items `Rename` (`job-rename`) and `Name it for me` (`job-name`).
Pause/Resume, Auto land, Judge, Report, Release, Requests, Checks, Subagents are Job acts shown as icons (1.5) and move into this menu only when the toolbelt is narrow.

### 1.4 What the person can do to the Job from the head: pause, resume, end

- **Pause / Resume:** icon act in the toolbelt (1.5) id `pause`; label `Pause`, or `Resume` while `session.paused === true`; glyph `circle-pause` (`CirclePause`) while not paused, `Play` while paused; tooltip `Pause` / `Paused` (`SessionTiles.tsx:460`); `aria-pressed = paused`; lit (`data-on`) while paused; presses `draft.pause(id)` or `draft.resume(id)` (`SessionTiles.tsx:236-241`); while Fleet has not answered (`session.pressed` includes `paused`) the tooltip reads `Sending`.
  Off reasons: `Fleet does not offer this` (no `draft.pause`), `The Job ended` / `The terminal is not listening` (`mute()`, `session-tiles.ts:168-173`).
  Key: bare `p` (`PAUSE_KEY`, `session-tiles.ts:152`) presses it while the Job is open and `p` is the key on the tile (`flight-keys.ts:47-52`); the palette also offers Pause/Resume ⌘P.
- **Resume-and-send:** typing to a paused Job asks first: dialog `Job is paused`, body `Resuming takes a slot again. The note goes after.`, confirm `Resume and send` (initial focus on confirm), cancel gives the box its words back (`paused-send.tsx`).
- **End job:** menu entry `End job` (1.3) or the done band's End button.
  `askToEnd` (`apps/desktop/src/renderer/src/end-job.tsx:41-52`): if the Job holds nothing in flight it closes at once (`draft.close(id)`); else opens `InFlightDialog` (`session-view/InFlightDialog.tsx`): `Dialog tone="destructive" title="In flight" confirmLabel="End job" cancelLabel="Keep going"`, body `<ul class="sv-inflight" aria-label="In flight">` rows `"<label>: <state>"` (just `<label>` when state is empty).
  The list is worked out in TypeScript from `session.lanes.job` (`inFlight()` in `packages/components/src/compositions/LaneCanvas/build.ts:735`: instances not `ended` and not in a lifecycle state with `end:true`, label = instance label, state = lifecycle state label or `unmapped` word or empty).
  **The wire already carries it as `SessionRecord.in_flight: Vec<InFlight {lifecycle, key, label, state}>` (`crates/ipc/src/sessions.rs:1443`, `lifecycles.rs:398`; "since 9 Nov 2026"), but Electron does not read that field** (grep: only `end-job.tsx`/`sessions-wired.test.ts`/protocol types mention in-flight; the dialog reads `lanes`).
- Stopped Job: `Stop` menu entry (above) is the deck's act.

### 1.5 The toolbelt under the composer (the rest of the head)

`SessionToolbelt` (`SessionFrame.tsx:490`) draws `<div class="armada-session-frame__toolbelt" role="group" aria-label="Job tools" data-room>` after the composer inside `.sv-dock` (`sessions.tsx:3531`).
CSS: `SessionFrame.css:72-86` `display:flex; flex:none; align-items:center; gap:--space-2; min-width:0; min-height:--h-control-sm; color:--fg-subtle`; spacious (`spacious-thread.css:566-600`): `margin-top:--space-2; padding:--space-1 --space-2 0; gap:--space-3`; every `.armada-button` in it is `--h-control` x `--h-control` (28px square), transparent border/ground, `--fg-subtle`, hover `--bg-hover`/`--fg-default`.
Contents, in DOM order (`SessionFrame.tsx:~656-696`):

1. **Repository** (`SessionRepositoryOf`, `apps/desktop/src/renderer/src/session-move.tsx:15`; `packages/components/src/compositions/SessionRepository/SessionRepository.tsx`): drawn only when Fleet serves `move` and the Job's manifest is found among served repositories.
   A chip: `FolderGit2` 12px + repository name + `ChevronDown` 12px; tooltip `Move to another repository` (or the name when `icon`-only), aria-label `Repository <name>`; a popover `Move to another repository` lists the other repositories (`aria-label="Repositories"`), a pick shows a caution Alert (what stays, what ends) and a `Move to <name>` button.
   Becomes a plain non-menu `Repository` chip when already moved (`to=[]`).
   Goes to icon-only (`icon`) when the head has no room (`useActsPlace().chip`).
   In a live terminal Job rows are disabled with tooltip `IN_A_TERMINAL`.
2. **Aa - Thread look** (`lookHere.head` = `ThreadLookButton`, `packages/components/src/compositions/ThreadSettings/ThreadSettings.tsx:507`): ghost sm button text `Aa`, tooltip/aria-label `Thread look`, `aria-haspopup="dialog"`; opens a Popover (align end, label `Thread look`) holding `ThreadSettings`: `Agent name` text field; switches `Show agent name`, `Show times`; selects `Text size`, `Spacing`; section `Quick responses` (buttons per response, `Edit <name>`, `Add response`); section `Quick replies` (list of lines with `Edit/Save/Pin/Unpin/Move up/Move down/Delete <text>`; `New quick reply` field + `Add`; switch `Learn from my messages`; ⌥↑/⌥↓ moves a line); section `Voice` (select `Voice`, `Whisper model` row with `Install` / progress bar `Installing Whisper model` / `Installed`).
   Look is stored by `thread-look-here.tsx` (setting `thread.look`, saved 400 ms after a change).
3. **Runs place** (`marked`, `sessions.tsx:2752`): `RunsPlaceControl` (`apps/desktop/src/renderer/src/RunsPlaceControl.tsx:42`): ghost icon button (glyph by place `PLACE_GLYPH`), tooltip/aria-label `Armada configuration kept <placePhrase>`, opens a menu starting with the line `Where Armada keeps this repository's configuration: how its tests, build and dev server run.`; J/K move, Enter picks, Esc closes.
   Absent while the card form (`RunsPlaceCard` in the thread's lead) is up (`cardLead`) or the thread is unread.
4. **Job acts** (`JobActsSlot`, `job-panes/acts.tsx:292`): `<span class="job-acts" role="toolbar" aria-label="Job acts">`; flex, gap `--space-1`; empty = `display:none`.
   Buttons are drawn into this slot by `SessionTiles` (`cockpit/deck/SessionTiles.tsx:796-840`, mounted `Deck.tsx:1734-1749` with `head`) as ghost sm icon-only 16px buttons (`drawIcon`, `:730-795`).
   Order: the drawn built-in tiles `land, pause, judge, report, release, requests` (`flight-order.ts:9-17`; `terminal` and `browser` are removed and kept as hidden buttons whose keys open those tabs), then the owner's own tiles, then the views `checks`, `subagents` (inserted at `customAt`, `:529-545`).

   | Act id | aria-label | Glyph (registry name -> lucide via `flight-glyphs`) | Tooltip (when on) | Off reason(s) | Press |
   |---|---|---|---|---|---|
   | `land` | `Auto land` | `plane-landing` | `Auto land on` / `Auto land off` | `Fleet does not offer this`; mute reasons | toggles `draft.autoLand(id, !on)`, then sends the agent LAND_ON/OFF words (`session-tiles.ts:158-161`); `aria-pressed = autoLand` |
   | `pause` | `Pause` / `Resume` | `circle-pause` / `Play` | `Pause` / `Paused` | see 1.4 | pause/resume |
   | `judge` | `Judge` (via `JudgeAct`, `judge/JudgeAct.tsx`) | `scale` | `Send a judge to read the work` | `Fleet does not offer this` | opens the Judge for the Job; running state while reading |
   | `report` | `Report issue with job` | `bug` | `Report an issue with the Job` | `The repository is not known` | opens `ReportBox` |
   | `release` | `Release everything the Job holds` | `unplug` | `Release or hand over everything the Job holds` | `Fleet does not offer this`; `The Job holds nothing` | opens `ReleaseHoldsDialog` (names slots, branches, open PRs; buttons `Release all` / `Release` / `Hand to <title>`) |
   | `requests` | `Requests` | `arrow-right-left` | `Send a request with your cookie for its host` | | opens Requests in the side panel |
   | `checks` | `Checks` | `shield-check` | `The Checks the agent ran. Each result went back to it as the run ended` / running: `Checks running. The agent gets its result when it ends` | `The Job has run no Checks` | toggles the Checks layer/tab; glyph blue + pulsing while a Check runs; `aria-pressed = layer==="checks"` |
   | `subagents` | `Subagents` | `drone` | `Every subagent` / `Subagents running` | `The Job has no subagents` | toggles Subagents layer/tab; blue + pulsing while any subagent running |

   Off acts keep their tooltip (the reason), `aria-disabled=true`, `pointer-events:auto`, colour `--fg-subtle` (`job-panes.css:~424-428`). A lit act (`aria-pressed=true`/`data-on`): `--bg-hover` ground, `--fg-default`, glyph `--accent` (`:416-436`). Running: glyph `--accent`, opacity pulse `calc(--duration-pulse * 1.5)` (stills under reduced motion and `data-idle`, `styles/index.css:119`). Pressed-but-unanswered pause/land shows tooltip `Sending`. Tooltips carry the act's key as a shortcut (`flightKey`): `⌥⇧1`..`⌥⇧9`, `⌥⇧0` in `KEY_ORDER` = artifacts, system, checks, subagents, slot, land, judge, report, release, terminal, requests (`flight-order.ts:33-47`, `session-tiles.ts:tileKey`); pause = `p`. An owner's tile may carry its own key and glyph/name (`flight-controls.ts`). Custom tile tooltips: `Send the Job: <words>`, `Type <words> to the Job`, `Run <words> in the Job's bay`, `Open <words>` (+ `. Hides the repository's <x>`).
   **Narrowing:** `JobActsSlot` measures the `data-room` row (`settle()`, `acts.tsx:200-247`): first the repository chip goes icon-only, then acts leave one at a time in `LEAVES` order (`requests, release, report, subagents, judge, checks, land, pause`; an unnamed owner act leaves first) into the Thread tab's menu; widening reverses. The name's floor is 18ch. Left-over acts are kept drawn out of sight (`<div hidden class="job-acts">`) so their keys still press them.
5. **Running Checks** (`RunningChecks`) and **Context ring** (`ContextFigure`) are SessionFrame props that `sessions.tsx` does NOT pass in the paned view; the context ring is drawn instead in the composer bar (`beforeSend`, 4.3).
6. **Loadout mark** (`LoadoutMark`, only when `session.loadout` is set): quiet mark, hover opens a card of counts and the latest turn, press opens the whole Loadout (`goes.onOpenLoadout(session.id)`).
7. **Marks** `<span class="armada-session-frame__marks">` (empty = `display:none`): `ModMark` (when `modOutOfDate && state !== "quiet"`; `RotateCw`; tooltip `Mod out of date: run /reload-plugins`) and `ToolsMark` (`Unplug` 16px; tooltip/aria-label = `toolsMissingSaid(session.toolsMissing)`; pulses amber in the needs-you hue (`data-apparent`, 1.6 s opacity 1 -> 0.35) when no `ToolsBand` is up (`toolsApparent: !toolsBand`); with `toolsFixes` it is a button opening a popover `ToolsMenu`: a head sentence, one row per piece (`said` + button with `act`: `Restart the agent` (mod), `Reconnect` (door); `Working` spinner while running; error text in the row)).

Marks that appear in non-paned heads only (`!headless`): `Terminal` ghost button (if `docked.terminal`), and the `Attachments` / `Show attachments` panel-right buttons (tooltips `Open what this Job holds` / `Show attachments`, `PanelRightOpen`), the Ledger aside with `Ledger` heading and `Hide attachments` (`PanelRightClose`).

### 1.6 Marks, bands and rows that say the Job came from, or went to, another

| Mark | Component / lines | Layout and strings | Condition |
|---|---|---|---|
| Forked from | `packages/components/src/compositions/ForkedFrom/ForkedFrom.tsx:9` (+ `.css`), mounted `sessions.tsx:3054-3064` above the thread | `<details class="armada-forked-from" aria-label="Forked from <title>">`; closed `<summary role="button">Forked from <title></summary>` (padding `--space-2 --space-3`, `--text-xs`, `--fg-muted`; box `--bg-raised`, radius `--radius-md`); open shows the origin Job's rows read-only in `.armada-forked-from__rows` (`max-height: calc(--space-12 * 6)`, padding `0 --space-3 --space-3`); onToggle open calls `draft.watch(origin.id)` to read the thread | `origin !== undefined && !carried` where `origin` = attachment `forked_from` of kind (`sessions.tsx:1887-1895`); `carried = forkedFrom.moved !== undefined` |
| Started by | `StartedBy.tsx:5`; `useStartedBy` (`session-started.tsx:48`) | `<button class="armada-started-by">Started by <title></button>` opens that Job. Same hook seeds the held Job's first message into the composer draft (`seedDraft`) and forgets it if untouched when the start message goes away | attachment `started_by` |
| Moved/forked-to rows | `session-started.tsx:20-31` labels `Moved to`, `Moved from`, `Forked to`, `Forked from`, `Started`, `Started by` | thread rows (agent A) | |
| Held notice | `HeldNotice`, `operator/marks.tsx:65`; passed as `notice` to `SessionFrame` (renders even when headless: `{notice}` after the header, `SessionFrame.tsx:~770`) | `<div class="armada-operator__held" role="group" aria-label="Held by the Operator">` `Clock` 12px + `Held until <waitSaid>.[ <why>]` + secondary sm button `Start now` (`store.act({kind:"start", line:id, by:"press"})`) | Operator's line has a row for this Job |
| Handoff failed | `HandoffBand`, `NeedsYouPanel/HandoffBand.tsx:20` | see 1.7 | `session.handoff === "failed" && failedAway !== session.id` |

### 1.7 The bands on the composer's top edge (`.sv-dock` / `.sv-bands`)

`.sv-dock` (`session-view.css:1365`): `position:relative; flex:none`; spacious: `width: calc(100% - 2*--spacious-gutter); max-width: --spacious-outer; margin-inline:auto` (`spacious-thread.css:30-34`).
With `data-needs` (any band up) it takes `margin-block-start: calc(--h-badge + --space-3)` and the composer's top corners go square (`:1382-1388`).
`.sv-bands` is absolutely placed with `inset-block-end:100%`, `flex-direction:column`, stacking bands in this order top to bottom: ToolsBand, NeedsYouPanel, HandoffBand, SessionEndBand, WaitingBand (`sessions.tsx:3296-3381`).

Shared band chrome (`NeedsYouPanel.css`): band = `height:--h-badge` (20px), radius `--radius-md --radius-md 0 0` (`--radius-lg` when open), background = the band hue, text `--fg-inverse`; trigger `padding:0 --space-3; gap:--space-2`; label type `--text-2xs/--leading-2xs`, `--weight-heading`, `--tracking-caps`, UPPERCASE; fold chevron (`ChevronUp` 12px) rotates 180deg when open (`--duration-sheet`); body reveals via grid-rows 0fr->1fr over `--duration-sheet`; body `padding: --space-2 --space-3; gap:--space-3; border 1px solid hue (no top/bottom); background --bg-raised; max-height: calc(var(--thread-h,55vh)*0.95 - --h-badge); overflow:auto`.
Hues: waiting-for-you `--step-waiting` (amber), end `--step-advanced`, failed `--status-completed-failed`, waiting `--status-handed-in`.
Opening a band dims the thread behind it (`.armada-session-thread__stage::after`) and a press on the dim, or Esc, folds it (`useScrimFold`, `useEscapeFold`).

| Band | Source | Strings, glyph, controls | Shown when (`sessions.tsx`) |
|---|---|---|---|
| Tools missing | `NeedsYouPanel/ToolsBand.tsx` | band label `Tools missing` (`Unplug` 12px), `aria-label` = the missing-tools sentence; right-end `Dismiss` button (tooltip + aria-label `Dismiss`, `X` 12px); body always open: `ToolsMenu` (sentence + per-piece rows with acts) not focused on draw; Esc inside = dismiss; stays away until the missing set changes | `toolsBand = toolsFixing !== undefined && toolsAway.shown` (`:2761`) |
| Needs you | `NeedsYouPanel.tsx` | **covered by the thread-cards section B7**; its inputs: `rows=needs`, `open=panelOpen`, `question`, `cards=notOurs`, `review=reviewWaiting` | `waits` (`:2642`); open when `holdsDelivery` or the person opened it and it lists something (`:2659`) |
| Handoff failed | `HandoffBand.tsx` | band `Failed to hand off to new job` (`X` 12px: a press dismisses; no chevron); body always open: reason line (`--text-xs`, `--fg-muted`), buttons secondary sm `Retry` (`RotateCw` 12px, `Kbd` = `r`, plus `ThreadKbd` ⌥↵) and `Dispatch job to fix` (`Kbd` = `d`). Keys `r` / `d` while focus is in the band (`handoff_retry`, `handoff_dispatch`, `actions.toml:1230-1249`); the band also registers as a waiting thread card whose Yes = Retry. Shown also as a failure toast (`failureShown`) | `failedShown = session.handoff === "failed" && failedAway !== session.id` (`:2684`); `reason = session.handoffReason` |
| Nothing left to do | `SessionEndBand.tsx` | band `Nothing left to do` (`Check` 12px, chevron, `aria-expanded`); body `DoneBody` (agent A); opens on coming into the Job until folded | `endShown = doneProp !== undefined && !waits && !endInThread` (`:2682`); in the spacious take (`endInThread`) the ending is the thread's last entry instead (`<section class="armada-session-end" aria-label="Nothing left to do">` `<h3>` `Check` 14px `Nothing left to do`) so this band is not drawn in the shipped take |
| Waiting | `WaitingBand.tsx` | band `Waiting` (`Radar` 12px) + one 12px glyph per wait (`Radar` watch, `GitMerge` merge, `CircleDot` checks; each a `role=img` with tooltip = its sentence) + chevron; body `<ul aria-label="Waiting on">` rows (glyph + sentence) each with a dismiss button (tooltip/aria-label `Stop the watch` for a watch, else `Stop waiting on it`; label `"<away>: <sentence>"`; `X` 12px) | `state==="held" && !waits && !endShown && !failedShown && !toolsBand` (`:2763`); folded until pressed (`waitOpen`) |

Start strip (held Job): `.sv-start-strip` (`session-view.css:1390`): `display:flex; flex-wrap:wrap; align-items:center; gap:--space-2; padding:--space-1 --space-2; border:1px solid --border-default (no bottom); radius --radius-md --radius-md 0 0; background: color-mix(accent 12%, --bg-sunken)`; hosts the Workflow / Delivery selects (`StartPicks`, thread section B).
Selects `--h-control-sm`, `--text-xs`.
Start's busy spinner `sv-start-spin` 1 turn per `--duration-pulse`.

### 1.8 The non-paned head (`SessionFrame` header, when not `headless`)

`SessionFrame.tsx:738-775`.
`<Card class="armada-session-frame" role="region" aria-label="Job <address ?? id>">` -> `<header class="armada-session-frame__head">` (flex, `align-items:center`, `gap:--space-2`, `color:--fg-muted`, `SessionFrame.css:35-41`): `[state mark] [SessionTitle: flex:1, gap:--space-3, colour --fg-default, weight --weight-medium; one line ellipsised; Tooltip = whole title; a button when renameable: aria-label "<title>, rename", click edits in place via Input aria-label "Job name", Enter saves, Esc/blur cancels; empty title: SquarePen ghost button "Rename"; wand ghost button "Name it for me" (WandSparkles, pending while naming)] [repository] [actions] [marks]` then More (`Ellipsis`, `triggerLabel="More"`) then `back` (`BackAct`, `cockpit/deck/panes.tsx:40`: ghost icon `X`, tooltip `Back to the Job` | `Back to the bay` | `Back`, shortcut Esc).
`data-take="rows"|"overflow"` variants are mock-only (`SessionFrame.css:43-46`, `acts.tsx:19`).

### 1.9 Layout control in the title bar

`LayoutControl` (`job-panes/LayoutControl.tsx:31`), portalled into `[data-job-slot]` in the title bar (`.armada-title-bar__job`: flex, `align-items:center`, `gap:--space-3`, hidden when empty, no-drag; `TitleBar.css:366-375`).
`<span class="job-layout">` with a `Tooltip` and a `DropdownMenu` trigger glyph `PanelsTopLeft`; tooltip + trigger label `Layout: <layout>` plus ` · edited` when the arrangement differs from the saved layout; when edited a 6px dot (`.job-layout__mark`: absolute `top/right: --space-1`, `calc(--space-1*1.5)` square, round, `--fg-default`, 1px `--bg-base` ring) is drawn.
Menu: one item per layout (`take:<name>`, `selected` = current) - rule - `Update <layout>` (act, disabled unless edited), `Save as…` (act), `Reset` (act, disabled unless edited) - rule - `New Jobs open on` with the current value as `detail` and a submenu of layouts.
`Save as…` turns the menu into a name field (menu label `Save as`, or `A layout has that name.` when taken; search label `Name`; Enter saves, Esc leaves; name kept via `useKept("job-layout-name:<jobId>")`).
Base layout is named `Default` (`store.ts:37`).
Key `⌘L` (`layout_menu`) opens it.

### 1.10 Keys that act on the head

| Key | Scope | Does | Bound |
|---|---|---|---|
| `p` | Job open, pause tile wears `p`; not in text | Pause/Resume | `cockpit/deck/flight-keys.ts:47-52`, `DeckHost.tsx`; `PAUSE_KEY` `session-tiles.ts:152` |
| `⌥⇧1`..`⌥⇧9`, `⌥⇧0` | anywhere on the Cockpit with a Job open; in a text box only with ⌘/⌃ held (`chordTileFor`) | press the act whose seat it is (see 1.5 order) | `flight-keys.ts:12-37` (`.job-acts [data-key]`) |
| `⌥M` | anywhere in the Job's view | focus the composer (`.sv-dock [role="textbox"][aria-label="Message"]`), select all then collapse to end | `SessionWorkspace.tsx:207-217` (`message_job`) |
| `⇧⌘A` | Job view | toggle Artifacts layer/tab | `story.ts:259`, `SessionWorkspace.tsx:218` |
| `⌥W`, `⌥1`..`⌥4`, `⌥F` | Job view | band/tab keys: workflow, Done, Now, Open(next), Deferred, field | `story.ts:206-241`, `SessionWorkspace.tsx:233-254` (in panes: `focusKind(id, tab)`) |
| `⌥N`, `⌥L`, `⌥⇧L` | Job view | whole plan toggle; Done grouping; Lanes | `story.ts:247-268` |
| `⌘D`, `⌘⌥D`, `⌥⌘Enter`, `⌘L`, `⌘Z`, `⇧⌘Z`, `⌃Tab`, `⌃⇧Tab`, `⌘⌥←↑→↓` | Job area (anywhere; ⌘Z/⇧⌘Z/zoom not in a text box) | split right, split down, zoom pane, Layout menu, undo/redo arrangement, next/prev tab, focus neighbour pane | `job-panes/keys.ts:55-104`; ids in `crates/core-model/domain/actions.toml:152-260` |
| `Enter`/`Esc`/`Shift+F10`/`Menu` | on the tab | rename commit / cancel; open the tab menu | `PaneView.tsx:438-447, 559-568` |
| `⇧⌘C`, `⌘⌫`, `⌥⌘F`, `⌘P`, `⇧⌘F` | palette only (Job row) | Copy Job id, Stop, Fork, Pause/Resume, Pin | `docs/contracts/design-system/keyboard-and-command-palette.md:539` |
| `Esc` | Job on stage, non-paned head only has the visible Back button (`BackAct`, tooltip shortcut Esc) | Back to the list/bay | `cockpit/deck/panes.tsx:40-57`; the deck's own Esc handler for the paned view was not traced |

### 1.11 Fleet data driving the head

| Part | Fields read (TS `Session`; origin in `SessionRecord`/`SessionRow`) |
|---|---|
| State mark | `paused`, `dead`, `startFailed`, `stopped{gaveUp,...}`, `waits`, `work.proposal.reading`, `start`, `limitedUntil`, `asked`, `owes` (`waiting_for`), `standing`, `turn.state/wokenBy`, `attachments` (subagent/background/pull_request), `rows` (check running), `lapsed`, `lastTurnAt`, `held_open_by` (`waitsOf`) <- `SessionRecord.state/standing/stopped/waits/waiting_for/held_open_by/running/last_turn_at`. **The state and sentence are computed in TypeScript (`jobStateOf`), not by Fleet** (the wire has `SessionStanding` only). |
| Title / rename / name-for-me | `title` <- `SessionRecord.title`; acts are `draft.rename`, `draft.nameForMe` |
| Menu | `dead`, `context.tokens`, `trail.started` (= `ThreadHead.trail.started`, `ThreadTrail`), `draft.fork/forkAt/close/retro` |
| Acts | `autoLand` <- `SessionRecord.auto_land`; `paused`; `attachments` (slots/branches/PRs -> release enable and `heldOf`); `rows` has a `check`; `attachments` subagent count; `pressed` (optimistic flags) |
| In flight | `lanes.job.instances` + lifecycle copy (TS); wire `in_flight` unused |
| Marks | `modOutOfDate` <- `mod_out_of_date`; `toolsMissing` <- `tools_missing`; `loadout` <- `loadout_summary`; `context`/`contextBreakdown` <- `context_breakdown` |
| Bands | `handoff`/`handoffReason`, `heldOpenBy`, `watches`, `done{since,summary}` <- `done_since/done_summary`, `waiting_for` |

QUESTION: There is no plan progress bar in the Job's head in the paned view.
The only plan bar is `PlanBar` (`session-view/FieldPlan.tsx:12`, one segment per task, `role="group" aria-label="Plan"`), drawn in the Field panel's large layer (`FieldPanel.tsx:687`) and the map card (`cockpit/deck/BuildCard.tsx`).
Which one did the owner mean by "the plan progress bar" in the head?
Not determined.

QUESTION: `Fleet` `Session` is assembled in `packages/screens/src/draft/sessions.ts` and a mock/wired layer from `SessionRecord`+`SessionRow`; this pass did not read that mapping file in full, so some TS-only derivations (for example `owes`, `lapsed`, `standing` defaults, `pressed`) are named above from usage, not from their definitions.

---

## 2. The Job panes tab bar

| | |
|---|---|
| Components | `job-panes/PaneView.tsx` (`PaneView` `:674-945`, `TabButton` `:223`), `job-panes/JobPanes.tsx` (cells, split handles `:140-388`), `LayoutControl.tsx` |
| CSS | `job-panes/job-panes.css:1-262` (area, cell, pane, strip, tab), `:268-345` (body, drop zones) |
| Kinds | `job-panes/kinds.ts:41-205`, `tree.ts:5-56` |

### 2.1 Layout

| Region | Rule |
|---|---|
| `.job-panes` | `display:flex; flex-direction:column; flex:1 1 0; gap:--space-1` (`job-panes.css:4-11`) |
| `.job-panes__area` | `position:relative; flex:1 1 0; margin: calc(--deck-gutter / -2)` (= -3px) |
| `.job-panes__cell` | absolute, `padding: calc(--deck-gutter / 2)` = 3px, so adjacent panes are 6px (`--gap-panel`) apart; placed by `left/top/width/height` as % of the area |
| `.job-pane` (section) | `border:1px solid var(--edge)`; `--edge` = `--border-glass`, `--border-subtle` when focused; `radius --radius-lg`; background `--bg-raised`; `overflow:hidden`; no shadow (`:99-119`) |
| `.job-pane__strip` | `display:flex; align-items:stretch; flex:none; min-height:--h-row-header (28px); background:--bg-sunken; box-shadow: inset 0 -1px 0 var(--edge)` (hairline under); `overflow-x:auto; scrollbar-width:none` (`:134-146`); children `.armada-button`/`.armada-dropdown-menu` (the + and the Restore/Close-pane buttons): `align-self:center; margin-inline:--space-1` |
| `.job-pane__tab` | `display:flex; flex:none; align-items:stretch`; separators as inset shadows; active: `--bg-raised` ground, breaks the hairline |
| Split handle | `.job-panes__line` 8px wide/high (`--space-2`) centred on the split (offset `--space-1`); inside `DockHandle quiet` label `the panes`; double-click/reset sets ratio 0.5; each side keeps min extent (`RATIO_MIN 0.12`, `MIN_PANE_W 160`, `MIN_PANE_H 120`, `tree.ts:120-150`) |
| Body | `.job-pane__body` `position:relative; flex:1 1 0; flex-direction:column; isolation:isolate; overflow:hidden`; inside it Thread/panels draw flush: `.armada-session-frame, .sv-panel, .armada-terminal, .job-browser, .sv-layer` get `border:0; radius:0; box-shadow:none` (`:279-288`) |

Tab strip contents, left to right: the pane's tabs (`pane.tabs` order), the **Add tab** `+` menu, then (only when zoomed) **Restore**, then (only when the pane has no tabs) **Close pane**.

### 2.2 Which tabs the strip can list and how each looks

Tab labels use `KINDS[kind].label` unless the tab has its own `label` (a Terminal's name, a browser's host, a Custom tab's mod name via `customLabel`/`tabTitle`) (`PaneView.tsx:67-70`).
Every non-Thread tab shows its kind's 16px glyph (stroke 2) in the mark slot; a Custom tab shows the mod's glyph (`tabGlyph(custom.glyph)`, fallback `PanelTop`); while its Mod Job builds or works, the glyph is a turning `LoaderCircle` (tooltip = the Mod Job's title or `Mod Job`; `role=img`); while the Mod Job waits on the person the mark is `SessionMark state="waiting" said="Waiting on you"` pulsing in the needs-you amber (`.job-pane__asks`, 1.6 s opacity 0.35); a **Walk** tab not yet seen gets a tint+pulse overlay (`.job-pane__tab[data-owes]::after`: `--status-awaiting-review` at opacity 0.06 -> 0.14, 1.6 s, runs under reduced motion too).

| Kind (`TabKind`) | Label | Glyph | In the + menu | Notes |
|---|---|---|---|---|
| thread | `Thread` | `MessageSquareText` (replaced by the state mark and the Job's title) | only when no Thread tab is open | one per Job (`hasKind` guard `:873`) |
| browser | `Browser` | `Globe` | yes | label becomes the page's host |
| terminal | `Terminal` / `Terminal <n>` | `SquareTerminal` | yes: `Terminal` (new shell), then one `Open <name>` per shell the Job has with no tab | rename, Send to agent, Copy terminal ID in tab menu |
| done | `Done` | `CircleCheck` | yes | default pane B |
| now | `Now` | `Activity` | yes | |
| open | `Open` | `Circle` | yes | |
| deferred | `Deferred` | `CircleDashed` | yes | |
| changes | `Changes` | `Diff` | yes | |
| plan | `Plan` | `ListChecks` | yes | |
| pulls | `Pull Requests` | `GitPullRequest` | yes | |
| issues | `Issues` | `Ticket` | yes | |
| jobs | `Jobs` | `Link` | yes | |
| ledger | `Ledger` | `Files` | yes | |
| files | `Files` | `FolderTree` | yes | |
| request | `Request` | `Send` | yes | |
| subagents | `Subagents` | `drone` registry glyph (`glyphOf("drone","zap")`) | yes | |
| checks | `Checks` | `shield-check` | yes | |
| workflow | `Workflow` | `Workflow` | yes | |
| system | `System` | `Cpu` | yes | |
| bays | `Bays` | `LayoutGrid` | yes | |
| spine | `Spine` | `Waypoints` | yes | |
| judge | `Judge` | `Scale` | yes | |
| custom | `Custom` (or the mod's name) | `PanelTop` / mod glyph | under a rule: one item per enabled valid tab mod (`mod:<name>`, label `tabTitle(name)`), then `Custom` | |
| draft, walk, document | `Draft`, `Walk`, `Document` | `NotebookPen`, `Eye`, `FileText` | not offered (opened from where each is listed) | |

(The + menu lists `family.kinds` minus custom/draft/walk/document, and minus thread when one is open; `addEntries` `:575-638`. Order is `JOB_TAB_KINDS` order `tree.ts:5-30`: thread, browser, terminal, done, now, open, deferred, plan, changes, pulls, issues, jobs, ledger, files, request, subagents, checks, workflow, system, bays, spine, judge.) Only the Thread, Open, Done, Deferred and Now tabs are in the default layout.

### 2.3 Controls

| Control | Label / tooltip | Glyph | Does | Shown |
|---|---|---|---|---|
| Tab | label text | kind glyph | press: bring to front (`activate`) | always |
| Close tab (×) | tooltip `Close`, aria-label `Close <label>` | `X` | closes; a Terminal with a running program asks first (Dialog title `End <label>`, body `A program is running in <label>.`, confirm `End terminal`); closing a terminal tab ends its shell; focus returns to the pane | visible on active tab, hover, focus-within |
| More (Thread tab only) | `More` | `Ellipsis` | menu (1.3) | same visibility rule |
| Add tab | trigger label `Add tab` | `Plus` | menu of kinds (above); on open lists untabbed shells (`listUntabbed`) | always |
| Restore | tooltip `Restore` + shortcut `⌥⌘Enter`, aria-label `Restore`, `aria-pressed` | `Minimize2` | un-zoom | pane is zoomed |
| Close pane | `Close pane` | `X` | closes the empty pane | `pane.tabs.length === 0` |
| Tab menu (right-click / Shift+F10) | trigger label `Tab actions` | | entries in 1.3 (for non-Job tabs: Terminal extras `Send to agent`, `Copy terminal ID`, `Rename`, rule; then `Move to window`, `Zoom`/`Restore`, `Split right`, `Split down`; Custom built tabs add `Edit with its Job`, `Open the Job that built it`; rule; `Close`) | |
| Middle-click on tab | | | closes it | |
| Double-click on strip (not on a tab's buttons/menus) | | | zoom/restore the pane | |
| Drag a tab | | | to another strip (insert index by pointer vs tab midpoints), to a pane edge (split) or middle (join); body shows 5 drop zones in a grid (`25% 1fr 25%` columns, `25% 1fr 25%` rows) with a preview of the half it would take; gaps between panes and the outer edges light a `--fg-muted` bar (`--space-1` thick) as drop targets | |

### 2.4 States

| State | Drawn | Condition |
|---|---|---|
| Pane focused | border `--border-subtle` instead of `--border-glass` | `arrangement.focus === pane.id` (`PaneView.tsx:813`) |
| Drag over pane | whole pane washed `color-mix(--fg-default 6%)` + `2px solid --border-strong` inset outline | `data-hot` (`job-panes.css:121-130`) |
| Zoomed | pane lifted to `position:fixed` over the area box, z-index 20; split handles hidden; Restore button | `usePaneZoomed` |
| Empty pane | `aria-label="Empty pane"`, body `aria-label="Empty"`, body shows `empty?.(pane)` if given | no tabs |

### 2.5 Keys

`←`/`→` move among tabs, `Home`/`End` go to first/last (the reached tab comes to the front and takes focus) when focus is on a tab (`PaneView.tsx:750-772`); roving tabindex (`tabIndex=0` on the active tab only).
Global pane keys in 1.10. Focus into a pane goes to its active tab or `Add tab` (`keys.ts:focusIn`).

### 2.6 Accessibility

`section.deck-pane.job-pane` `aria-label="Pane: <active label>"` (or `Empty pane`); strip `role="tablist" aria-label="Tabs"`; each tab `button role="tab" aria-selected tabIndex`; body `role="tabpanel" aria-label="<active label>"`; drop zones `aria-hidden`.

### 2.7 Data

Arrangement/layouts are local state (`job-panes/store.ts`, kept per Job id and per family `job`; saved layouts shared across Jobs; `forAll` default).
Nothing on the wire.
Tab labels for terminals mirror Fleet shells (`shellsOf(window.armada)`); Custom tabs follow the mod list.

---

## 3. The thread

The thread is `SessionThread` (`packages/components/src/compositions/SessionThread/SessionThread.tsx`, 4238 lines) drawn with the spacious take (`apps/desktop/src/renderer/src/spacious-thread.css`, always on, `main.tsx:14`), mounted by `sessions.tsx:3058-3278` as the Thread tab's pane.
It is fed raw `SessionRow`s plus the `SessionRecord`; it folds them itself.
Part A covers the container, messages, folded calls, subagents, Checks, Fleet turns, other ledger rows and fork-from-selection.
Part B covers the cards (sketch, file, document, plan, draft, asked question, picks, open call, lapsed, notices), the four thread keys and the Needs you dock.

### Part A: Job view — THREAD, part A (as built in Electron; record of what IS)

Scope: thread container, person/agent/Fleet messages, folded calls, subagents, Checks, Fleet turns, ledger-ish rows, fork-from-selection.
Part B (not covered): plan card, draft card, asked question card / picks / QuestionDeck / AskCard, start picks, open call, lapsed questions, sketch/file/document cards, thread-keys (`⌥↵ ⌥E ⌥R ⌥X`).
All paths are repo-relative `path:line`. `SC` = `packages/components/src/compositions/SessionThread`. `SPA` = `apps/desktop/src/renderer/src/spacious-thread.css`.

## Tokens used

Values from `packages/tokens/tokens.css` (dark is default `:root`; light = `[data-theme="light"]`).
Density default; `data-spacing="compact|roomy"` on root rescales `--space-*`, `--h-control*` (tokens.css:1245-1281) — default set below.
`takes.css` (`:root[data-look="new"]`, always set by `packages/components/src/look.ts`) lifts Dark `--fg-muted`/`--fg-subtle` and fixes shadows.

| Token | Dark value (shipped default) | Light | Source |
|---|---|---|---|
| `--space-1/2/3/4/6/8/12` | 4 / 8 / 12 / 16 / 24 / 32 / 48 px | same | tokens.css:605-611 |
| `--text-2xs/xs/sm/base/lg/xl` | 12 / 13 / 14 / 15 / 18 / 23 px | same | tokens.css:572-578 |
| `--leading-2xs/xs/sm/base/lg` | 16 / 18 / 20 / 22 / 28 px | same | tokens.css:572-576 |
| **Inside `.armada-prose`** `--text-2xs/xs/sm/base` + `--leading-*` | 13/14/15/16 px; leading 18/20/22/24 | same | Prose.css `.armada-prose` (one step up; inside `.deck` 12/13/14/15, lead 16/18/20/22) |
| `--weight-body/medium/heading` | 400 / 500 / 600 | | tokens.css:588-590 |
| `--tracking-caps` | 0.04em | | tokens.css:592 |
| `--font-sans` | "IBM Plex Sans", ui-sans-serif, system-ui, sans-serif | | tokens.css:58 |
| `--font-mono` | "IBM Plex Mono", ui-monospace, "SF Mono", Menlo, monospace | | tokens.css:59 |
| `--border-width` | 1px | | tokens.css:623 |
| `--radius-sm/md/lg` | 3 / 6 / 8 px (inside `.armada-session-frame` the spacious CSS re-declares exactly these, SPA:1-21) | | tokens.css:626-628 |
| `--h-control` / `-sm` / `-lg` | 28 / 24 / 32 px | | tokens.css:657-659 |
| `--h-row-unit` | 36 px | | tokens.css:633 |
| `--focus-ring` / `--focus-ring-offset` | `1px solid var(--accent)` / 2px | | tokens.css:1149-1150 |
| `--edge-active` | 2px | | tokens.css:1001 |
| `--duration-press/fast/base/sheet/travel` | 90 / 120 / 180 / 220 / 300 ms | | tokens.css:1069-1078 |
| `--duration-exit/enter/pulse/spin` | 140 / =base 180 / 1600 / 1100 ms | | tokens.css:1082-1096 |
| `--ease` / `--ease-out` | cubic-bezier(0.2,0,0,1) / cubic-bezier(0.16,1,0.3,1) | | tokens.css:1079,1088 |
| `--tooltip-delay` | 400ms | | tokens.css:1089 |
| `--z-menu` / `--z-tooltip` / `--z-modal` | 40 / 60 / 70 | | tokens.css:990-992 |
| `--w-tooltip` | 340px (max bubble width) | | tokens.css:969 |
| `--shadow-overlay` | `0 8px 24px rgb(0 0 0 / .5)` (takes.css:84) | `0 8px 24px rgb(15 23 42 / .16)` | takes.css:84,113 |
| `--bg-base` / `--bg-sunken` / `--bg-raised` / `--bg-overlay` / `--bg-hover` | #0F1419 / #0B0F13 / #161C23 / #1D242D / #212A34 | #F6F8FA / #EDF0F3 / #FFFFFF / #FFFFFF / #E8ECF0 | tokens.css:68-72, 1179-1183 |
| `--fg-default` | #E4E9EF | #18202A | tokens.css:79,1189 |
| `--fg-muted` | **#B3BFCE** (takes.css:107; tokens.css:80 is #93A1B1 and is overridden on Dark) | #4D5B6B | |
| `--fg-subtle` | **#94A3B6** (takes.css:108; tokens.css:81 #7E8CA0 overridden on Dark) | #5A6878 | |
| `--fg-inverse` | #0F1419 | #FFFFFF | tokens.css:82,1192 |
| `--border-subtle` / `-default` / `-strong` | #232B35 / #2E3946 / #3D4A5A | #E3E8ED / #D0D7DE / #AEB8C3 | tokens.css:74-76 |
| `--accent` / `-hover` / `-muted` / `-faint` | #4A9EDB / #5FB0E8 / #1C3A52 / `color-mix(accent 10%, transparent)` | #1B68A8 / #145389 / #D6E7F5 | tokens.css:85-87,109 |
| `--you` / `--you-edge` | #E8C547 / `color-mix(you 55%, transparent)` | #8F7000 | tokens.css:124-125 |
| `--operator-edge` | `color-mix(operator 38%, transparent)`; `--operator` #8E95FF | | tokens.css:115-117 |
| `--status-running` (= `--slot-held`, `--step-running`, `--run-running`) | #4FB8D9 | #1B657C | tokens.css:148,385 |
| `--status-completed-success` (= `--step-advanced`, `--run-passed`) | #4FAF7C | #2F694A | tokens.css:151 |
| `--status-completed-failed` (= `--step-failed`, `--run-failed`) | #E97878 | #B51D1D | tokens.css:152 |
| `--status-awaiting-review` (= `--step-waiting`) | #E5A93D | #7D5711 | tokens.css:149 |
| `--status-handed-in` | #4FB4AA | #2C6761 | tokens.css:263 |
| `--status-rejected` | #B489DA | #7F3BBB | tokens.css:153 |
| `--status-not-started` | #8C97A6 | #555F6D | tokens.css:147 |
| `--status-killed` (= `--run-stopped`) | #9BA3AC | #575F69 | tokens.css:154 |
| `--tool-look` / `--tool-change` / `--tool-run` | #8FA3D9 / #D98BB5 / #C2B48A | #3B5AAE / #9F346D / #6A5D37 | tokens.css:536-540 |
| `--surface-card` / `--surface-well` / `--edge-card` / `--text-meta` | = bg-raised / bg-sunken / border-default / fg-subtle | | tokens.css:1126-1139 |
| `--scrim` | `color-mix(bg-sunken 62%, transparent)` | `color-mix(fg-default 40%, transparent)` | tokens.css:1051 |

Colours quoted as `x N%` mean `color-mix(in srgb, x N%, transparent)` unless a second colour is named.

## A0 Orientation

### A0.1 Which take is shipped
| Fact | Where |
|---|---|
| The spacious take is ON for every Job in the shipped app. | `apps/desktop/src/renderer/src/main.tsx:14` sets `document.body.dataset.threadTake = "spacious"`; `main.tsx:6` imports `spacious-thread.css` after the base CSS. |
| `SpaciousTake` React context (`SC/spacious-take.ts:7`, default `false`) is provided as `useContext(SpaciousTake) \|\| body.dataset.threadTake === "spacious"` | `apps/desktop/src/renderer/src/sessions.tsx:1722`, provider `sessions.tsx:2874`. |
| Every spacious CSS rule is scoped `:root[data-look] [data-thread-take="spacious"] .armada-session-frame …` (so only inside the Job view's `.armada-session-frame`), and beats the base rules on specificity. | SPA throughout |
| Components branch on the context for structure: no agent name head, `MessageBar`, live calls list, lease as one line, `Thinking` word on the working line, `Exit N` on failed runs, undrawn tool filter. | `SessionThread.tsx` (marked `spacious` below) |
| A thread drawn elsewhere (call card, subagent pane, `ForkedFrom` origin thread `sessions.tsx:3048`, mock sheets) lacks the context and draws the BASE look. This spec records the SHIPPED (spacious) look and notes base differences in `Base` columns where a native build might need them. | |

### A0.2 Row kinds: every case of `Row` (`SessionThread.tsx:2036-2500`) in switch order, and who draws it
`Item` kinds are first (`SessionThread.tsx:1064`); `Row` receives `Item.kind === "row"` only.

| # | Item / row kind | Drawn by | Section | Part |
|---|---|---|---|---|
| i1 | item `calls` (folded tool run) | `Calls` 1231 | A3 | A |
| i2 | item `fleetTurn` | `FleetTurn` | A6 | A |
| i3 | item `subagent` | `SubagentRow` | A4 | A |
| i4 | item `checks` | `CheckRuns` 1389 | A5 | A |
| 1 | `retro` | `SessionRetroRow` (`packages/components/src/compositions/SessionRetroRow`) | A7.14 | A (not named by B; recorded here) |
| 2 | `woke` | inline `<li>` 2047 | A7.1 | A |
| 3 | `not_started` | `StartFailedRow` | A7.2 | A |
| 4 | `act` | inline 2076 | A7.3 | A |
| 5 | `window` (a walk shown) | inline 2090 | A7.4 | A (not named by B; recorded here) |
| 6 | `file` (show_file) | inline 2105 | — | **B** (file card) |
| 7 | `ran` | `RanRow` 1634 | A3.6 | A |
| 8 | `draft` | `DraftRow` | — | **B** |
| 9 | `requested` | `RequestedRow` | A7.5 | A |
| 10 | `saved` | `SavedRow` | A7.6 | A |
| 11 | `sketch` | `SketchRow` | — | **B** |
| 12 | `document` | `DocumentRow` | — | **B** |
| 13 | `started` | `StartedRow` | A7.7 | A |
| 14 | `moved` | `MovedRow` | A7.8 | A |
| 15 | `judged` | inline 2189 | A7.9 | A |
| 16 | `divider` | inline 2205 | A7.10 | A |
| 17 | `waiter` | `WaiterRow` | A7.11 | A |
| 18 | `linked` | `LinkedRow` | A7.12 | A |
| 19 | `operator` | `OperatorRow` | A7.13 | A |
| 20 | `disputed` | `DisputedRow` | A7.15 | A |
| 21 | `change` | inline 2210 | A7.16 | A |
| 22 | `suggest` (plan changes sent) | inline 2223 | A7.17 | A |
| 23 | `lease` / `release` | inline 2261 | A7.18 | A |
| 24 | `command` (person's terminal command) | inline 2303 | A7.19 | A |
| 25 | `compaction` | inline 2310 | A7.20 | A |
| 26 | `reasoning` | inline 2331 | A7.21 | A |
| 27 | `sent` (old Helm "words sent to another Job") | inline 2358 | A2.7 | A |
| 28 | `message` `from:"session"` | `FromSession` 1820 | A2.5 | A |
| 29 | `message` `from:"you"` whose text matches `Approved:`/`Re:` | `Did` 1544 | A2.4 | A |
| 30 | `message` `from:"you"` | `YourMessage` 1726 | A2.1 | A |
| 31 | `message` `from:"agent"` | inline `li.armada-thread-row` ~2418 | A2.3 | A |
| — | cards the thread draws that are NOT rows: plan card / plan record, sketch pin, ask cards, lapsed deck, trailing, inline `ThreadCard`s, `lead` | | | B (card contents); A1.4 places them |
| — | `browser_reminder` | filtered out before the thread (`session-thread-rows.ts:238`); the Job browser draws it | | not in thread |
| — | wire `plan`, `delivery` rows | no case in `rowsOfThread` (`packages/screens/src/sessions-wire.ts:1094`), so the Electron thread never draws them | | not drawn |

### A0.3 Where each fold/derivation is worked out (TS vs wire)
| Derivation | Electron | Wire |
|---|---|---|
| Folding tool runs / Check runs / Fleet turns | `itemsOf` `SessionThread.tsx:1113` in the renderer | Fleet ALSO folds the same way: `crates/fleet/src/thread_view.rs:128 fold` → `ThreadItem` (`crates/ipc/src/job_thread.rs:195`). Electron does not read `ThreadItem`; native can. |
| Differences native must keep in mind | Electron passes `keep` (disputed notices; and the thread's last Fleet turn while the Job `asking` or `waitingOnYou`) so those notices are NOT folded (they draw as `FromSession` bands). Rust fold has no `keep`. | Rust leaves a settled `ask` as `Row{Ask}` ("no row is worded here"); Electron words it as a tool line (A0.3b). |
| Run `names` (tool names, once each, first-call order) | `namesOf` 1201 | `ThreadItem::Calls.names` |
| Calls line (`Edited foo.ts`, `Ran 3 checks`) | `calls-line.ts` (TS only) | none |
| Row timestamps flip, ages | `Stamp` (TS) | `at` RFC 3339 |
| Optimistic person's message (`delivery`, `stableKey`) | `outgoing.ts` (client only) | none (not on wire) |
| `queued` | wire `Message.queued` | present |
| `batch` (Fleet notices batched by a taught rule) | `batchNotices` `apps/desktop/src/renderer/src/teach.ts:339` | none |
| `moved` rows | `session-moved.ts` from `forked_from` attachments | none |
| `divider`, carried "earlier Job" rows (ids prefixed `<earlier-id>:`) | `sessions.tsx:1905-1925` | none |
| `judged` rows | `judge/thread.tsx:36 useJudgedRows` from judge records | none |
| `change` rows | `sessions-wire.ts:2043 withChanges` from timeline events (`session.workflow|model|effort`, `what` = `Workflow|Model|Effort`, `from`/`to` split on `→` or `->`) | Rust `ThreadItem::Change` (`thread_view.rs:378`) same rule |
| `suggest` rows | `sessions-wire.ts:1405` from wire `plan_changes` row (`lines[]` verb/place/from/text → `verb/where/from/text`) | `SessionRow::PlanChanges` |
| `not_started` row | `session-thread-rows.ts:50` from `session.startFailed` (`why = "Did not start: " + cause`, id `not-started`, `at` = last row's at, else `lastTurnAt`, else epoch) | `ThreadItem::NotStarted{cause,log}` — Rust carries `cause` only; the `Did not start: ` prefix is TS |
| `reasoning`, `act`, `woke`, `operator` rows | TS row types exist (`SessionThread.tsx:232-312, 314-330`); the only producers found are mocks/stories; no wire variant | none |

A0.3b settled `ask` row → one tool line (so it folds inside a calls run): text `"<command>: <word>"`, `fromPhone` when `via == "phone"`, word map `packages/screens/src/sessions-wire.ts:1064`: ran_unasked "ran without asking"; allowed_once "allowed once"; allowed_and_remembered "allowed and remembered"; always_allowed "always allowed"; always_blocked "always blocked"; allowed_for_session "allowed for this job"; refused "refused"; unanswered "no answer"; kept "kept for you"; session_gone "job ended".
`window_ask` row → tool line `Asked in the window: <question>` + (answer ? ` — <answer with \n→" · ">`).

### A0.4 What `sessions.tsx` passes to `<SessionThread>` (`apps/desktop/src/renderer/src/sessions.tsx:3071-3262`)
Wrapped in `RequestOpen`, `SavedOpen`, `ProseCodeColour` (= `colourCode` when spacious, else null), and (outside this file) `SessionRefs` (prose references), `StampSurface` is inside the thread.

| Prop | Value / condition |
|---|---|
| `sessionId` | `session.id` |
| `rows` | `withJudged` = threadRows(session) → carried rows/moved rows/divider (1905-1950) → `withWorkflowCards` → `batchNotices` → `started` rows given `start` (when the started Job is held on a start card) → judged lines |
| `lead` | `RunsPlaceCard` (cardLead) and/or `ModEmpty` (modEmpty); absent otherwise |
| `transcript` | `session.transcript` (`"unread"`/`"refreshing"`) if set |
| `inline` | `inlineCards` if non-empty (mod cards) |
| `onOpenRetro` | `openRetro` |
| `onRetry` | `draft.retry(session.id,rowId)` if `draft.retry` |
| `onStartAgain` | `draft.startAgain(session.id)` if defined |
| `onReport` | `askToReportJob(session.id)` always |
| `onForkAt` | if `draft.forkAt`: `opening(forkAtPick(draft.forkAt, session.id, pick, setFocusFor), onOpen)` |
| `onTeachAt` / `onTeachRow` | if `teachOpen` defined: `teachOpen("words", wordsOfQuote(quote))` / `teachOpen(kind,text,at,ids)` |
| `onMuteNotice` | if `draft.muteNotice` |
| `onReplyAt` | if `session.dead === undefined`: `quoteIntoDraft(session.id, quote)` + focus the composer |
| `approval`, `sketching` | B |
| `waitingOnYou` | `true` when `session.asked !== undefined` |
| `onOpenWindow` | `draft.openWindow?.(id,url)` |
| `onOpenSketch`, `onOpenDocument`, `onAnswerSketch`, `onWithdrawSketch`, `onAnswerDocument`, `onDraft`, `onWithdrawDraft` | B |
| `onOpenFile` | video → `draft.openFile`; else open in side panel |
| `onReadMedia` | `readShownMedia(...)` |
| `noted` | `noted` (NotedOpen) |
| `onOpenCheck` | `goes.onOpenCheck(session.id, run)` (Checks page) |
| `onOpenSubagent` | `read({kind:"subagent", id})` |
| `working` | `session.turn.state === "working"` |
| `onStop` | `draft.stop(session.id)` if defined |
| `trailing` | `trailing` and/or the `Nothing left to do` end section when `doneProp && !waits && spacious` (B) |
| `runner` | if defined |
| `onCommand` + `commandNames` | only when `session.terminal === true && draft.command` |
| `onAnswer` | no-op |
| `onOpenSession` | `onOpen` |
| `...lookHere.thread` | `look {showTimes, showAgentName, textSize, spacing}`, `agentName` only if the Aa name is non-empty, `responses` (quick-response list), `onResponses` (edit list) — from `thread-look-here.tsx:352` |
| `onPickLine` | only when `lookHere` defined and `session.dead === undefined`: `ask` kind → quote goes into the composer + focus; else `draft.send(session.id, {text: pickedLine(text,response), files:[], sketches:[], tags:[]})` |
Not passed: `agentName` when blank (default `"Agent"`), `askRef`, `onOpenSent`.

## A1 Thread container

### A1.1 Where it lives
| What | File:line |
|---|---|
| Component `SessionThread` | `SC/SessionThread.tsx:3513-4238` |
| Base CSS | `SC/SessionThread.css:4-33` (container, stage, rows), `SC/ThreadRows.css:8-60` (row list, agent row) |
| Spacious CSS (shipped) | `SPA:1-143` (column, padding, mask, scrollbar, gap), `SPA:620-720` (live list, end card) |
| Skeleton | `SessionThread.tsx:2502` / `SC/SessionThread.css:1603-1625` |
| Mounted by | `apps/desktop/src/renderer/src/sessions.tsx:3071` inside `ThreadKeys` inside `.armada-session-frame__centre` (flex column, `gap: var(--space-3)`, `SessionFrame.css:392`) |

### A1.2 DOM / region tree (top to bottom)
```
div.armada-session-thread                       flex column, flex 1, min-w/h 0, gap --space-4 (16)   SessionThread.css:4
  [ForkFromSelection bar — portal to <body>, only when onForkAt|onReplyAt|onTeachAt]                   A8
  div.armada-session-thread__stage              relative, flex 1, column, min 0                       css:14
    div.armada-session-thread__rows  (the scroller)  role=region aria-label="Thread" tabindex=0         css:24
      StampSurface (all stamps in it flip together)
        ol.armada-thread-rows                   flex column, list-style none, margin/padding 0, min-w 0   ThreadRows.css:8
          li.armada-session-thread__lead        (lead prop)
          li.armada-session-thread__skeleton    (transcript === "unread")
          li.armada-session-thread__earlier     (Show earlier, when items hidden)
          …items…  (+ plan record li, + inline card li after the item they follow)
          li.armada-session-thread__refreshing  (transcript === "refreshing")
          li.armada-session-thread__tail        (asking: plan card / sketch pin / ask cards)  [B inside]
          li.armada-session-thread__working     (working line)
          li.armada-session-thread__subagents   (SubagentsCard, sticky)
          li.armada-session-thread__trailing    (trailing prop)
```
`.armada-thread-rows` gets `aria-busy="true"` while `transcript === "unread"` (`SessionThread.tsx:3932`).

### A1.3 Geometry (spacious = shipped). Pane width `W` = width of the scroller (= stage = thread = centre column of the Job view).
| Quantity | Rule | File:line |
|---|---|---|
| Container-query base | `.armada-session-frame` is `container-type: inline-size`; `cqi` = 1% of the FRAME's inline size (not the thread's) | SPA:3 |
| `--spacious-column` | `clamp(680px, 70cqi, 960px)` — width of the words | SPA:5 |
| `--spacious-bleed` | `space-4 + space-1` = 20px | SPA:7 |
| `--spacious-outer` | `column + 2*bleed` (= column + 40) — width of cards, composer, Needs you | SPA:9 |
| `--spacious-gutter` | `space-4` = 16px | SPA:11 |
| `--spacious-body` / `-leading` / `-meta` | 15px / 24px / 13px | SPA:12-14 |
| `--spacious-radius` | `--radius-lg` = 8px | SPA:15 |
| Scroller | `width: 100%`, `overflow: auto` + `overflow-x: hidden`, `isolation: isolate`, `flex: 1`, `min-height: 0` | css:24-29, SPA:38-44 |
| Scroller padding | top `space-4` (16), right/left each `max(16px, (100% − outer)/2) + 20px`, bottom 0. (`100%` = stage width.) Words column = `W − 2*padX`: when `W ≥ outer+32` the words are exactly `--spacious-column` wide and centred; narrower, words = `W − 72`. | SPA:38-44 |
| Row list | `gap: var(--spacious-turns)` = `space-6 + space-1` = **28px** (Aa spacing compact: `space-4` 16, roomy: `space-8 + space-2` 40); `padding-bottom: space-6` (24) | SPA:~138-141,170-183 |
| Message room | A message (`.armada-thread-row` agent or `.armada-session-you`) adds `margin-top: var(--thread-message-room, space-2)` = 8px above itself when it follows anything, and above the row after it (Aa spacing compact 4 / roomy 16). Margins add to the 28px gap. | ThreadRows.css:24-29; css:700-708 |
| Cards break out | `.armada-session-sketch`, `.armada-session-end`, `.armada-session-thread__ask`, `.armada-session-thread__subagents`, `.armada-session-subagent` get `margin-inline: −20px` so their edges are the composer's | SPA:46-58 |
| Subagent/ card paddings | `.armada-session-subagent__press`, `.armada-subagents__toggle`, `.armada-subagents__row`: `padding-inline: 20px` | SPA:74-85 |
| Fade mask | scroller `mask-image: linear-gradient(to bottom, transparent 0, #000 32px, #000 calc(100% − 16px), transparent 100%)` (`--spacious-fade-top: space-8`, `-foot: space-4`) | SPA:126-139 |
| Scrollbar | webkit: width 6px (`space-1 + space-1/2`); track transparent; thumb transparent, becomes `fg-default 14%` while the pointer is over the scroller, `28%` on the thumb; thumb radius `radius-md` 6px | SPA:92-124 |
| Focus ring on scroller | `outline: 1px solid var(--accent); outline-offset: −1px` | css:1463 |
| Frame body padding | `--body-pad: 0 0 var(--space-3)` | SPA:23-25 |

Base (non-spacious) differences: no max width or centring, no padding on the scroller, row gap `var(--thread-gap, space-4)` 16, message room 8, no mask, native scrollbar (`ThreadRows.css:8-30`).

### A1.4 Item order top to bottom (code order, `SessionThread.tsx:3970-4235`)
1. `lead` node (if any) — `li.armada-session-thread__lead`, no styling (`css:1570`).
2. Transcript skeleton if `transcript === "unread"` (A1.7).
3. `Show earlier` row if `hidden > 0` (A1.6).
4. Items oldest → newest.
   Each `Item` is drawn as: `calls`→`SteadyCalls` (A3), `fleetTurn`→`FleetTurn` (A6), `subagent`→`SubagentRow` (A4), `checks`→`CheckRuns` (A5), `row`→`SteadyRow` (A0.2).
   React key = `stableKey ?? id` for rows, `item.id` otherwise.
5. After an item, in this order: (a) the **approved plan record** (`li.armada-session-thread__record` holding `PlanCard`) after the row whose id equals `approval.after` (only when `approval.approved`); (b) each `inline` card whose `afterWhich` index equals this item.
6. `refreshing` spinner row.
7. `li.armada-session-thread__tail aria-label="Waiting on you"` when `asking` (= unapproved plan, or sketch pin, or `asked`, or `lapsed`): contents are plan card, sketch pin, ask cards in that order [B].
   `display:flex; flex-direction:column; gap: space-3 (12); list-style none` (css:1583).
8. Working line (A1.9) when `working && no running subagents && (!endsInCalls || asking)`.
9. `SubagentsCard` (A4.3) when any subagent is running.
10. `trailing` node (host's: e.g. unsent turn, or the end section) in `li.armada-session-thread__trailing`; the thread follows its growth (ResizeObserver).

Ordering rules inside items:
| Rule | Code |
|---|---|
| Rows are in wire order (oldest first). | `rows` prop |
| A tool run is consecutive `tool` rows with no subagent; any other row ends it; a `tool` with `subagent` is its own `subagent` item and also ends the run. | `itemsOf` 1113-1196 |
| A Check row starts a new `checks` item unless the previous item is a `checks` item of the same `name`. (A Check after a tool call starts its own item; Check A, Check B, Check A = three items.) | 1155-1171 |
| A Fleet notice (`message`, `from:"session"`, `sender.id === "fleet"`, id not in `keep`) swallows the following consecutive `tool` rows and `agent` messages as its `reply`. The LAST agent message of that reply is lifted out and drawn after the fold as a normal agent row. `handled` = reply non-empty. A `you` message, or any other kind, ends it. | 1113-1150 |
| `keep` = ids of disputed notices (`disputed` rows' `notice`), plus the last Fleet turn's notice while `asking || waitingOnYou` and the turn is the last item (or the second-last followed by an agent message). Kept notices draw as `FromSession` bands, not folds. | 3751-3786 |
| Spacious: rows that are `tool` calls named `TaskCreate`, `TaskUpdate`, `TaskList`, `TaskGet`, `ToolSearch`, `show_window` (after stripping an MCP prefix `a__b__tool` → `tool`) are removed before folding. | `UNDRAWN` 1215, filter 3766 |
| The sketch pinned as a card (`sketching`) removes its own non-replaced sketch row from the list. | 3755 |
| Inline card placement (`afterWhich` 736): index of the last item whose `timeOf(item) <= card.at`; first item if earlier than all; last item if card has no/invalid `at`. `timeOf` is the item's last row `at` (`Date.parse`; words give NaN). | 723-744 |

### A1.5 Scroll anchoring / stick-to-bottom / keys
| Behaviour | Code |
|---|---|
| `pinned` ref (starts true): thread opens at the newest row with no animation and stays at the end while the person is within `NEAR_END = 24`px of it. Scrolling up unpins; scrolling back within 24px repins (`onScroll`: `scrollHeight − scrollTop − clientHeight <= 24`). A scroll caused by the box's height changing (keyboard shortening) is ignored (`scrollerHeight` ref). | 3641, 3962-3972, 2543 |
| Reset on `sessionId` change: pinned = true, drawn = 40. | 3643-3646, 3789-3792 |
| A new message from the person (`rows` whose newest `you` message key (`stableKey ?? id`) changed for the same session) sets pinned = true → thread jumps to end. | 3650-3667 |
| After every layout where `rows`, `sessionId`, `working`, approval state (0/1/2), `sketching.sketch`, `asked.command` changed: if pinned, `scrollTop = scrollHeight`. | 3668-3678 |
| Trailing entry growth: ResizeObserver on `li.trailing` re-pins. | 3680-3700 |
| Opening: ResizeObserver on the scroller (box shortened) and on the `ol` (content grows without a row, e.g. a web font arriving) re-pin while pinned; the `ol` observer is dropped after `OPENING_MS = 5000` or the first `pointerdown`/`keydown` on the scroller. | 3703-3735 |
| Reading higher up: rows arriving at the end while NOT pinned widen the drawn window by the same count, so nothing above drops out. | 3796-3802 |
| Show earlier preserves position: before drawing 40 more it records `{scrollHeight, scrollTop}`, then sets `scrollTop = top + newHeight − oldHeight`. | 3805-3827 |
| Pane Back keeps place: leaving a Job notes the thread's `scrollTop` unless within 40px of the end, and restores it two animation frames after rows draw (`apps/desktop/src/renderer/src/session-view/place.ts:17-60`). | place.ts |
| **There is no jump-to-latest control.** The only floating jump control is `CallChip` (below). | |
| Keys on the scroller itself (only when `event.target === scroller` and no ⌘/Ctrl/Alt): PageUp = `scrollBy −0.9*clientHeight`; PageDown = `+0.9*clientHeight`; Home = `scrollBy −scrollTop` (to top); End = `scrollBy +scrollHeight` (to bottom). `behavior: "smooth"` unless `prefers-reduced-motion: reduce` → `"auto"`. Arrow keys and Space use the browser default. | 3938-3961 |

`CallChip` (`apps/desktop/src/renderer/src/session-view/CallChip.tsx`) — the one jump control: a pill centred horizontally over the composer, `position:absolute; inset-block-end: calc(space-8 + space-2)` = 40px above the zero-height `.sv-callchip` box, `inset-inline-start: 50%; translate −50% 0`; height `--h-control-sm` 24, padding `0 space-3`, border 1px `border-default`, radius `radius-md`, bg `bg-raised`, text colour `status-awaiting-review`, gap `space-1`; label is the word `Plan` (only when an unapproved plan is waiting and `planAsked`), then `ArrowDown` 12px.
Shown only while `li.armada-session-thread__tail`'s top is below `scrollerBottom − 24`; press scrolls the tail into view (`block:"end"`, smooth unless reduced motion) and focuses its first `button, textarea, input`.
Hover bg `color-mix(fg-default 6%, bg-raised)`; focus-visible: `box-shadow: inset 2px 0 0 var(--accent)` no outline (`session-view.css:1331-1361`).

### A1.6 Windowing
| Fact | Value | Code |
|---|---|---|
| Items drawn at first | `DRAWN = 40` ITEMS (folded items, not rows) — newest 40 | 2550, 3789 |
| `hidden` | `max(0, all.length − drawn)`; `items = all.slice(hidden)` | 3807 |
| Show earlier control | `li.armada-session-thread__earlier` (flex, justify center, list-style none) holding `Button variant=ghost size=sm aria-label="Show earlier"` with `ChevronUp` 12px/2; tooltip `Show earlier` | 3990-4003; css:1565 |
| Auto-press | IntersectionObserver on that `li`, root = scroller, `rootMargin: "100% 0% 0% 0%"` (`EARLIER_AHEAD = "100%"`): when it is within one screen of the top, it draws 40 more | 3828-3850 |
| Wire window | Fleet serves `THREAD_ITEMS_SERVED = 200` items, up to 1000 (`crates/ipc/src/job_thread.rs:25-27`); older via `before` (`JobThread.older`); Electron today holds all rows and windows in the renderer. | |

### A1.7 Skeleton and refresh
| State | Drawn | Code |
|---|---|---|
| `transcript === "unread"` | `li.armada-session-thread__skeleton` (`aria-hidden`; list has `aria-busy`): flex column gap `space-3`. Two turns (flex column gap `space-3`): a "you" frame (`padding space-3; border 1px border-default; radius radius-md; bg bg-raised`) holding `SkeletonText` with bar widths `[70%,45%]` then `[55%]`; then an agent block (flex column gap `space-3`): one `Skeleton` width 15% and `SkeletonText` widths `[92%,80%,86%,40%]` (turn 1) / `[88%,64%]` (turn 2). | 2502-2540; css:1603-1625 |
| Skeleton primitive | each bar height `space-2` (8), width given/100%, bg `fg-default 7%`, masked into 7px×8px rounded cells (cell width `space-2 − border-width` = 7px), a lit copy (`fg-default 26%`) slides in from the left in 16 steps over 4.8s infinite (delays 0 / −1.2s / −2.7s on children 3n+2 / 3n); no motion under reduced motion. `SkeletonText` = flex column gap `space-2`, width 100%, `role=status aria-label="Loading" aria-busy`, with a `WorkingMark` (14px, `fg-subtle`) above the bars. | `primitives/Skeleton/Skeleton.css` |
| `transcript === "refreshing"` | `li.armada-session-thread__refreshing` (flex, `fg-muted`) with `LoaderCircle` 12px/2 in `span[role=img aria-label="Loading new messages"]`, tooltip `Loading new messages`, turning `armada-session-you-turn` 1600ms linear infinite (none under reduced motion) | 4170-4182; css:1627-1646 |

### A1.8 Arrival animation
Rows added after the first draw: if ≤ 4 new `li` children appear in one commit, each gets `data-arrived`; CSS `.armada-thread-rows > [data-arrived] { animation: armada-thread-arrive var(--duration-enter) var(--ease-out) backwards }`, keyframe from `opacity 0; translateY(space-2 = 8px)` (`css:1649-1658`).
The first draw, a "Show earlier" batch (>4) and `transcript === "unread"` (resets the tracking) get none.
Code `SessionThread.tsx:3589-3602`.

### A1.9 Working line (`li.armada-session-thread__working`)
Shown when `working === true && running subagents == 0 && (!endsInCalls || asking)` where `endsInCalls` = last drawn item is a `calls` item or a `reasoning` row with `live`.
| Part | Draw |
|---|---|
| Container | flex, align center, gap `space-1` (base) / `space-2` (spacious), padding `space-1 0`, colour `fg-muted`; spacious font 13px/20px (css:734, SPA:605) |
| Mark | `SessionMark state="working" said="Working"` → `span.armada-session-mark[role=img][aria-label="Working"][data-bucket="working"]` colour `status-running`, containing `WorkingMark` 16px; tooltip `Working` |
| Word | spacious only: `span.armada-session-thread__working-said` text **Thinking** |
| Stop | `SessionStop` when `onStop` (A1.10) |
`WorkingMark` (`primitives/WorkingMark`): 16×16 viewBox, stroke 1.6 currentColor, ring `r=6` at 30% opacity + a dot `r=1.4` orbiting (group rotates 360° per `--duration-spin` 1100ms linear; none under reduced motion).
**Spacious rewrites it** (SPA:~930-960): ring opacity 1, `stroke-dasharray: 11 27`, `stroke-linecap: round`, dot hidden, whole `svg` rotates 360° in 0.9s linear infinite (`will-change: transform`; none under reduced motion).

### A1.10 `SessionStop` (`SC/SessionStop.tsx`)
`Button variant=ghost size=sm iconOnly aria-label="Stop"`, tooltip `Stop`, glyph `Square` 16px/2 (filled square: "stop what is running").
Click `preventDefault + stopPropagation` (does not toggle a folded group) then `onStop()`.
Button box = 24×24 (`--h-control-sm`), ghost: transparent, `fg-muted`, hover bg `bg-hover` + `fg-default`, active bg `bg-sunken` + border `border-strong`, radius `radius-md`.

### A1.11 Timestamps (`RowStamp` 587 → `Stamp`, `packages/components/src/primitives/Stamp/Stamp.tsx`)
| Aspect | Rule |
|---|---|
| Default text | clock in the owner's format: `DEFAULT_TIME_FORMAT = {hour12:false, seconds:true, date:false}` → `HH:MM:SS` (locale `Intl.DateTimeFormat([], {hour:"2-digit",minute:"2-digit",second:"2-digit",hour12:false})`). A stamp from an earlier local day than now shows `Mon D ` prefix (`Intl {month:"short",day:"numeric"}`), e.g. `Oct 12 14:02:09`. `date:true` always prefixes. `hour12:true` → `numeric` hour with AM/PM. `packages/components/src/time-format.ts:17-136` |
| Age reading | `<1m ago`, `Nm ago`, `Nh ago`, `Nd ago` (floor; ahead-of-now reads `<1m ago`); redrawn every 15s |
| Flip | a stamp is a `<button class="armada-stamp">` (no fill, no underline, inherits font/colour; `cursor:pointer`); a press flips EVERY stamp inside the thread's `StampSurface` between clock and age; tooltip shows the other reading; the press does not reach the row (stopPropagation) |
| Non-instant `at` | text drawn as it came, no button, no tooltip |
| `still` | plain `<span>` (used inside buttons/summaries: from-band, started, linked, waiter, judged, check run `__at`, operator, saved, start-failed, disputed); it still follows the surface flip and format |
| Hidden by Aa | `data-times="hidden"` hides every `.armada-stamp` in the rows list (`css:674`); the message bar's own `__at` too (SPA:1552) |
| Wire | `at` is `Instant` RFC 3339 on every row |

### A1.12 Copy on select (`useCopyOnSelect`, 2562)
On `mousedown` (button 0) inside the thread set `dragging`; on `mouseup` anywhere (document), if dragging and a non-collapsed selection whose common ancestor is inside the thread and not inside `textarea, input, [contenteditable]:not([contenteditable='false'])`, and the focused element is not such a field inside the thread, then `navigator.clipboard.writeText(selection.toString())` silently (errors swallowed).
No toast, no mark.
It reads, never owns, the selection.

### A1.13 Aa panel look (`look` prop → attributes on `div.armada-session-thread`)
| Setting | Attribute | Effect (spacious) |
|---|---|---|
| `showTimes === false` | `data-times="hidden"` | all stamps in rows hidden |
| `showAgentName === false` | `data-who="hidden"` | base only (agent head isn't drawn in spacious anyway) |
| `textSize` ≠ base | `data-text="xs|sm|lg"` | `--spacious-body`/`-leading`: xs → `--text-xs` (13px)/20px; sm → `--text-sm` (14px)/22px; lg → `--text-lg` (18px)/28px; base 15/24 (SPA:149-168) |
| `spacing` ≠ default | `data-spacing="compact|roomy"` | `--spacious-turns` 16 / 40 (default 28); `--thread-message-room` 4 / 16 (default 8) |
Defaults: `DEFAULT_LOOK = {showTimes:true, showAgentName:true, textSize:"base", spacing:"default"…}` (`packages/components/src/compositions/ThreadSettings/thread-look.ts:208`), saved under setting `thread.look`.
Note: the body-size attribute does not size a plain (tag-bearing) sent paragraph `p.armada-session-said`, which inherits `--text-sm` 14px/20px (or `--thread-text` when `data-text` is set) from `.armada-session-you__body` / `.armada-thread-row__message`.

### A1.14 Accessibility of the container
| Element | Role / name |
|---|---|
| Scroller | `role="region" aria-label="Thread"`, `tabindex=0` |
| `ol.armada-thread-rows` | `<ol>` (list), `aria-busy` while unread |
| Skeleton | `aria-hidden` |
| Tail `li` | `aria-label="Waiting on you"` |
| Show earlier | `aria-label="Show earlier"` |
| Loading new messages | `role=img aria-label="Loading new messages"` |
| Fork bar | `role="toolbar" aria-label="Selection"` (A8) |

### A1.15 Wire for the container
`JobThread{session_id, head: ThreadHead{asked, open_call, lapsed, fleet_acts[], trail}, items[], older}` (`crates/ipc/src/job_thread.rs:50`).
Part A reads only `items` (+ `head` for working/asked state elsewhere).
`working` comes from `SessionRecord`'s turn state (`session.turn.state === "working"`), `transcript` from the client's load state.
Electron's `endsInCalls`, windowing, pin, skeleton, arrival animation are all renderer-local.

## A2 Messages

Common: all message `<li>`s sit in `ol.armada-thread-rows` (A1).
Glyph sizes are px with `strokeWidth 2` unless said.
`FORK_ROW` (`data-fork-row="<row id>"`) marks the text block of a person, agent or Fleet-band message when the thread is `forkable` (= any of onForkAt/onReplyAt/onTeachAt given; `sessions.tsx` always gives onReplyAt unless the Job is dead, onTeachAt when teach is open, onForkAt when the host serves `forkAt`) — it is what makes selected words in that block reachable by A8. Wire row: `SessionRow::Message{from, text, files, sketches, tags, queued, ...}`; `delivery` and `stableKey` are client-only (A0.3).

### A2.1 The person's message (`YourMessage`, SessionThread.tsx:1726; `li.armada-session-you`)
Location: right-aligned in the row list. Used by `Row` for every `from:"you"` message that is not an act (A2.4).

| Aspect | Spacious (shipped) | Base |
|---|---|---|
| Outer `li` | `role="region" aria-label="Your message"`, `data-delivery`; flex column; `align-self: flex-end`; `gap: space-2` (8); `max-width: 80%` of the words column; `margin-inline-end: −20px` (reaches the bleed, so the bubble edge is the code-block edge); no border, transparent bg | `max-width: 75%`, border 1px `border-subtle`, radius `radius-lg*1.5` = 12px, bg `fg-default 5%`, overflow hidden |
| `__body` (the bubble) | padding `space-3 (12) / 20px` (`space-4 + space-1`), radius 8px, bg `fg-default 7%` (no edge); inner flex column gap `space-2` | padding `12 16 8`, `text-sm` 14/20 |
| Words | Prose at 15/24 (`--spacious-body/-leading`) with paragraph gap `space-4`; or `p.armada-session-said` (A2.8) | `text-sm` |
| Band under bubble | `div.armada-session-you__band`: flex, align center, **justify end**, gap `space-2`, padding `0 space-2` (spacious) / `0 space-4 space-2` (base), colour `fg-subtle`; `display:none` when empty | |
| Band content (left→right, all right-aligned) | Delivery mark (A2.2) · Queued mark (A2.2) · stamp `armada-session-you__at` (sans 13px/18, tabular, `fg-subtle`; base: mono 12/16) | |
| Sending pulse | `[data-delivery="sending"]` whole `li` animates opacity 1 → 0.45 → 1, `1600ms*1.6 = 2560ms` ease-in-out infinite; under reduced motion no animation, `opacity: .6` | |
| Sent attachments | `Sent` (A2.6) inside the bubble under the words | |
| Retry | only when `delivery === "failed"` and the host passes `onRetry`: `div.armada-session-you__retry` (flex) with `Button size=sm variant=secondary` label **Retry** → `onRetry(row.id)` (`sessions.tsx:3095` gives `draft.retry(session.id, rowId)`) | |
| Debug info | A2.10 | |
| Notes message | A2.11 | |

Content derivation: `splitDebugInfo(text)` → `said` (before the line `## Debug info`) and `debug`.
`placeTags(said, tags, openTag)` (A2.8) → if any tag found in the words, draw `p.armada-session-said`; else `Prose` (or `NotedMessage` when the text parses as a notes message and an `opens` context exists).
Prose here is wrapped so Run and slash-command sends are OFF (`ProseRunner=null`, `ProseCommand=null`).

Keys: none on the bubble (not focusable). Selecting words → A8; Aa text-size does not apply to a tag-bearing `p.armada-session-said` (A1.13).

### A2.2 Delivery, Queued, Retry (band marks)
| Mark | Condition | Draw | Tooltip / aria |
|---|---|---|---|
| Delivery sending | `row.delivery === "sending"` (optimistic row not yet taken by Fleet) | `LoaderCircle` 12px in `span.armada-session-you__delivery[data-state="sending"]` role=img, inline-flex; svg turns `armada-session-you-turn` 1600ms linear infinite (none under reduced motion) | `Sending` |
| Delivery failed | `row.delivery === "failed"` | `X` 12px, same span, `data-state="failed"` (band ink, no extra colour) | `Not sent` |
| none | `delivery` undefined (Fleet's row) | nothing | |
| Queued | `row.queued === true` (wire `Message.queued`) | `span.armada-session-you__queued` role=img: `Clock` 12px + word **Queued** (root `--text-2xs` 12px / `--leading-2xs` 16px, `fg-muted`, gap `space-1`; the bubble is not inside `.armada-prose`) | `Queued until the subagent returns` |
`delivery` clears when Fleet's own row arrives; the row keeps `stableKey` so the DOM element survives (`outgoing.ts`).
Sending→failed rule and Retry resend live in `apps/desktop/src/renderer/src/outgoing.ts` (client).

### A2.3 The agent's message (`li.armada-thread-row[data-from="agent"]`, SessionThread.tsx ~2418)
| Part | Spacious (shipped) | Base |
|---|---|---|
| `li` | flex column, `gap: space-4` (16) between message and bar; `min-width:0` | gap `space-1` |
| Head (name + stamp) | **not drawn** | `div.armada-thread-row__head`: agent name (`agentName` prop, default `Agent`, `text-2xs` 12/16 weight 500 `fg-muted`) + stamp (mono 12/16 `fg-subtle`); `data-who="hidden"` hides the name |
| `div.armada-thread-row__message` | flex column gap `space-2`, `max-width:100%`, `text-wrap: pretty`, `overflow-wrap:anywhere`, colour `fg-default`, Prose at 15/24; `ProsePick` provider is `null` (the base's per-paragraph pick bar is OFF) | `text-sm`, per-paragraph `ProsePick` |
| `MessageBar` | drawn (A2.3a) | not drawn |
| `Sent` | `Sent row tags=placed.loose` under the bar (agent messages seldom carry any) | |
Plain tag-bearing agent text: `p.armada-session-said` (pre-wrap, `overflow-wrap:anywhere`, margin 0) at 14/20 (A1.13 note).
Agent text is Prose (A2.9) with the thread's ProseRunner/ProseCommand providers ON (code blocks can Run, slash commands can be sent) and with `ProseRefs` (A2.9) so `#123`, Job ids and paths become links.
Message-room: `margin-top: --thread-message-room` (8) before and after (A1.3).

**A2.3a `MessageBar`** (`MessageBar.tsx`; CSS SPA:1023-1100, 1553, 1718)
| Aspect | Value |
|---|---|
| Element | `div.armada-message-bar role="toolbar" aria-label="Message"`, `data-held` while a response menu or editor is open |
| Layout | flex, align center, `gap: space-1`, `margin: −8px 0 0 −8px` (pulled up under the words and out 8px to the left so the first button's glyph lines with the text); height of children 28px |
| Visibility | `opacity: 0`, `transition: opacity var(--duration-fast)=120ms var(--ease)`; `opacity: 1` while `.armada-thread-row` is `:hover` or `:focus-within`, or `[data-held]`. Its room is always kept (no layout shift). No reduced-motion override is declared for this fade (see Open questions). |
| Buttons | all `Button variant=ghost size=sm iconOnly`; in the bar overridden to 28×28 (`--h-control`), `border-radius:999px`, transparent border/bg, colour `fg-subtle`; hover bg `bg-hover` colour `fg-default`; svg 15×15; focus ring `1px solid accent`, offset 2px (Button.css:250) |
| 1. Copy | `CopyAct label="Copy message"`: `Copy` 16 glyph (css-resized 15) → press writes the agent's raw markdown `row.text` to the clipboard; on success the glyph becomes `Check` and label/tooltip `Copied` for 1500 ms (`COPIED_FOR`), then back; a refused write changes nothing. Tooltip + aria-label `Copy message`. |
| 2. Reply | shown only if the thread has `onReplyAt` (the `MessageReply` context = `spacious ? onReplyAt : null`); `Reply` glyph; tooltip + aria `Reply`; press → `onReplyAt({rowId, quote: quoteOf(row.text)})` — host puts the quote in the Job's composer draft (`quoteIntoDraft`) and focuses it. `quoteOf` = each line prefixed `> ` (blank line → `>`), trimmed, followed by `\n\n`. |
| 3. Quick responses | `span.armada-message-bar__responses` (inline-flex, gap `space-1`); drawn only if `picks != null && responses.length > 0` (picks given by the host via `onPickLine`; `sessions.tsx` gives them unless the Job is dead). One ghost icon button per response: aria-label + tooltip = response `name`; the glyph is the response's emoji in `span.armada-message-bar__emoji` (`text-sm` 14px, `filter: grayscale(1)`, `opacity .7`; full colour and opacity on hover or `aria-pressed=true`). `aria-pressed` = the emoji the person already sent to this message. Press → `picks.pick(plainOf(row.text).trim(), response)`: the whole message is quoted. Defaults (`primitives/Prose/responses.ts`): ✅ "Pick this" (send), 👍 "Yes to this" (send), 👎 "No to this" (send), ❓ "Ask about this" (**ask**: puts the quote in the box and sends nothing; the caret goes on the blank line under it). Max 6 (`MAX_RESPONSES`), min 1. Stored in settings (user-editable; owner asked for emoji 31 Oct 2026). |
| Response menu (only when `picks.onAct` given) | right press, ContextMenu key, or Shift+F10 on a response button opens `DropdownMenu` (`triggerLabel="Quick response <name>"`) at the pointer / under the button with entries: `Edit`; `Add response` (disabled with hint `Six at most` when 6); separator; `Remove` (disabled when only one) — see `responseEntries` `PickLine.tsx:82`. Edit/Add open `ResponseEditorLayer` anchored on the bar (label `Edit <name>` or `Add response`; saves emoji + name; an Edit with >1 responses also offers Remove). |
| 4. Stamp | `RowStamp className="armada-message-bar__at"`: `margin-inline-start: space-2`, sans 13px (root `--text-xs`) tabular `fg-subtle`; flips clock/age with all stamps; hidden by `data-times="hidden"` |
Note the stamp font: `.armada-message-bar__at` uses `var(--text-xs)`; the bar is NOT inside `.armada-prose` so it is the root 13px.
Tab order: Copy, Reply, each response, stamp (a `button`), in DOM order; the bar is fully in the tab order even at opacity 0 (so `:focus-within` reveals it).

### A2.4 The person's act (`Did`, 1544; `li.armada-session-did`)
Condition: a `from:"you"` message whose whole text is `Approved: <address>` (regex `^Approved: (\S+)$`) or starts `Re: <first line>` (`^Re: ([^\n]+)\n?([\s\S]*)$`).
`Re:` body has Fleet's closing line stripped (`Fleet took this off your waiting list.` or `This settles the item, so Fleet took it off your waiting list. Do not add it back.`) and is trimmed.
Not shown as a bubble.
| Aspect | Value |
|---|---|
| `li` | `role="region" aria-label="You approved"` / `"You answered"`; grid `auto minmax(0,1fr) auto`, `align-items:center`, gap `space-1 / space-3`, padding `space-2 / space-3`, radius `radius-md`, **bg `fg-default`, text `fg-inverse`** (a filled light bar on Dark; dark bar on Light). Not overridden by spacious. |
| Eyebrow | `span.armada-session-did__eyebrow`: **Approved** or **Answered**; `text-2xs`(12/16) weight 600 uppercase tracking 0.04em |
| Chip | `span.armada-session-did__chip role="img" aria-label="<Approved|Answered>: <target>"`: inline-flex, gap `space-1`, padding `0 space-2`, border 1px `fg-inverse`, radius `radius-sm`, mono `text-2xs` / `leading-xs`(18); `Check` 12/2 (Approved) or `Reply` 12/2 (Answered) + `span.armada-session-did__target` (ellipsis, nowrap, min-width 0). |
| Shown target | Approved: if the address contains `?walk=…`/`&walk=…`, shown is `?walk=<id>` (the query only), else the address; Answered: the `Re:` line. |
| Tooltip | wraps the chip with the full target when `shown !== target` or `target.length > 40` (`TARGET_TRUNCATES`) |
| Said | Answered with non-empty body: `span.armada-session-did__said` in grid row 2 spanning all columns, `text-sm` 14/20, `overflow-wrap:anywhere`, plain text (no markdown) |
| Stamp | `Stamp at` (flippable) `armada-session-did__at` in column 3, mono 12/16 |

### A2.5 Another Job's message or a Fleet notice not folded (`FromSession`, 1820; `li.armada-session-from`)
Drawn for `from:"session"` rows NOT swallowed into a Fleet turn (A6): any other Job's message (`sender.id` ≠ `fleet`), a disputed notice, a kept notice (A1.4), or a Fleet notice with `keep`.
| Aspect | Value (spacious has NO overrides for this class: base look ships) |
|---|---|
| `li` | `role="region" aria-label="Message from <sender.title>"`; flex column; border 1px **`accent`**; radius `radius-md` 6; bg `bg-raised`; overflow hidden |
| Band | `button.armada-session-from__band type=button aria-label="Open Job <sender.title>"`: flex, align center, gap `space-2`, padding `space-1 space-3`, no border, bg `accent-faint`, `fg-default`, left aligned; hover bg `accent-muted`; focus ring inset (`outline 1px accent`, offset −2px). Press → `onOpenSession(sender.id)`. Content: `SquareTerminal` 12/2 · `span.__id` (mono `text-2xs` `fg-muted`) = `sender.id` · `span.__title` (flex 1, weight 500) = `sender.title` · `DisputedMark` if `disputed` · `Stamp still` (mono 12/16 `fg-muted`). |
| Body | `div.armada-session-from__body` padding `space-2 space-3`, `text-sm` 14/20 (Prose root-scale → 15/22 inside `.armada-prose`; spacious sets prose to 15/24 inside `.armada-thread-rows`), contains `Prose text`; carries `data-fork-row` when forkable. |
| `DisputedMark` | `MessageSquareWarning` 12/2 in `span.armada-session-disputed__mark role=img aria-label=<DISPUTED>`; tooltip same. (the string is `Disputed`, `DisputedRow.tsx`) |
| Context menu | enabled when the row has `notice && onMuteNotice` ("muting") or `sender.id==="fleet" && onTeach` ("teaching"): right press (anywhere on `li`) or ContextMenu / Shift+F10 on the `li` opens `DropdownMenu` `triggerLabel="Fleet notice"` with `noticeEntries`: [if teaching] `Teach` (GraduationCap icon); [if teaching && muting] separator; [if muting] label `Disable <NOTICE_NAMES[notice]>` then items `For job`, `For repo`, `For 1 hour`. Closing returns focus to the band button. Teach → `onTeach({kind:"notice", text, at, ids:[notice]?})`; a mute choice → `onMuteNotice(notice, "job"|"repo"|"hour")`. `NOTICE_NAMES` map (`FleetTurn.tsx:59-104`) is the heading's name list (44 entries). |

### A2.6 What a message carries (`Sent`, 1001; `SentTo.tsx`)
`div.armada-session-sent`: flex wrap, align start, gap `space-2`. Drawn only if files + sketches + loose tags > 0. In order:
1. each file: has `src` (a picture) → `Enlargeable` button with `img.armada-session-sent__picture` (max-width `48*4 = 192px`, max-height `144px`, border 1px `border-default`, radius `radius-md`); no `src` → `AttachmentChip filename=<name>`.
2. each sketch → `AttachmentChip filename=<title> from="Sketch"`.
3. each tag NOT placed inline → `AttachmentChip filename=<tag title> from=<TAG_KIND[kind]>` where `TAG_KIND`: session "Job", pull_request "Pull request", branch "Branch", issue "Issue", report "Reported bug", terminal "Terminal".
`AttachmentChip` (read-only: no remove): `span.armada-attachment-chip` inline-flex, align center, gap `space-2`, min-height 24, padding `space-1 space-2`, border 1px `border-default`, radius `radius-sm`, bg `bg-sunken`, sans `text-xs` 13/18 `fg-default`, max-width 100%; `from` rendered BEFORE the name (`span.__from`, quiet); name `span.__name` ellipsis nowrap; optional kind word after (`link`, `Studio node`, none for file) in `fg-subtle`.
`Enlargeable` (primitives/Enlargeable): thumbnail is `button.armada-enlargeable aria-label=<name>` → opens a modal `role=dialog aria-modal aria-label=<name>` portalled to body via `ZoomablePicture` (zoom, Close button `ViewerClose`); Escape closes (capture phase), Tab is trapped inside, a press outside the picture at fit closes, focus returns to the thumbnail.
Picture is the same `src` (no second fetch).
No text drawn.
`SentTo` (`div.armada-sent-to`): flex, align center, gap `space-2`, `text-2xs`/`leading-xs`, `fg-muted`: label **Sent to** · `button.armada-sent-to__chip type=button aria-label="Sent to Job <title>"` (inline-flex, gap `space-1`, padding `0 space-2`, border 1px `border-default`, radius `radius-sm`, bg `bg-sunken`, `fg-default`; hover border `operator-edge`) with `SquareTerminal` 12 (`fg-muted`), title (ellipsis), repository (mono, `fg-muted`, flex none).
Press → `onOpenSent({kind:"session", id})`.
`target.slot` hovering draws the Session card (not in this file).

### A2.7 `sent` row (words sent to another Job from a box not this thread's own; 2358)
Wire row `sent` (`sessions-wire.ts:1133`: words an old thread sent on to a Session or Job; `target{id,kind,title,manifest_id→repository}`, `files`, `tags`).
Draws as `li.armada-session-you[role=region][aria-label="Your message"]` with the bubble: words (tag-bearing → `p.armada-session-said`, else Prose), `Sent`, then `SentTo`; band with stamp.
No Delivery/Queued/Retry, not forkable.

### A2.8 Inline tags (`InlineTag.tsx`; css `SessionComposer.css:291-366`)
A message's `tags` (kind session|pull_request|branch|issue|report|terminal, `id`, `title`, optional `url`) are drawn inline where the words contain `@<title>` (first match from the previous position, in tag order); the matched `@title` text is REPLACED by the chip; unmatched tags go to `Sent` as chips (A2.6).
If no tag matched, the message draws as Prose instead.
| Variant | Draw |
|---|---|
| Read-only | `span.armada-inline-tag`: inline-flex, align center, gap `space-1`, padding-inline `space-1`, border 1px `border-default`, radius `radius-md`, bg `bg-raised`, `fg-default`, `text-xs` 13/18, nowrap, baseline-aligned, `user-select: all`. Glyph 12/2 (`fg-muted`) from `TAG_GLYPH`: session `SquareTerminal`, pull_request `GitPullRequest`, branch `GitBranch`, issue `Ticket`, report `Bug`, terminal `Terminal`; then `tag.title`. |
| Link (host provides `ProseRefs` and the kind is session/pull_request/issue) | `a.armada-inline-tag.armada-inline-tag--link href="#<kind>"`, colour `accent` (hover `accent-hover`), glyph inherits, no underline, `user-select:auto`; tooltip `<Job|Pull request|Issue|…> · <title without leading "#123 " or issue key>`; click `preventDefault` then `refs.open({kind,id,title})` (opens the layer). branch/report/terminal tags are never links. |

### A2.9 Markdown (`Prose`, `primitives/Prose/Prose.tsx`; css `Prose.css`)
Parser: remark-parse + remark-gfm (+ breaks); block flow (margins between siblings, never flex).
All sizes below are the Prose scale (`--text-sm`=15/22, `--text-xs`=14/20, `--text-2xs`=13/18, `--text-base`=16/24); in the spacious thread paragraphs and `.armada-prose` are forced to 15/24 and the prose gap is `space-4` (16); `p + p` extra top margin `space-6` (24) (SPA ~330).
| Element | Draw |
|---|---|
| Paragraph | `p.armada-prose__paragraph`, `text-wrap: pretty` |
| Headings h1–h6 | all become `p.armada-prose__said[data-level]`: weight 600, `fg-default`; levels 1–2: `text-lg` 18/28 + 1px `border-default` bottom rule + `space-1` bottom padding; level 3: `text-base` (16/24); 4–6 body size. Extra top margin `space-6` after a sibling. |
| Bold | `strong.armada-prose__strong`: weight 600, `fg-default`. Italic default (em). `del`: `fg-subtle` |
| Lists | `ul/ol.armada-prose__list`: margin-inline-start `space-4`, padding-left `space-4` (ordered `space-6`; nested `space-6`), `list-style` disc / decimal; item gap `space-1`; markers `accent` (ordered markers mono + weight 600). Task list: checkbox `input` before the text (accent-color `accent`, margin-right `space-2`, vertical-align middle, read-only), no marker. |
| Links | web `http(s)` only; drawn as `a.armada-prose__link` (accent, no underline, hover `accent-hover` + underline, offset 4px, `title=<address>`) ONLY where the host gives `ProseLinks` (click → host opens; never navigates); otherwise the link text renders as plain text. Armada-ref links (`armada-ref:` scheme) become `ReferenceLink`s. Walk links (`http://localhost|127.0.0.1…?walk=`) open the Job's walk and show tooltip `Opens in the walk window`. Middle click is swallowed. |
| References | `ProseRefs.scan(text)` (host) returns spans naming a pull request/issue number, a Job id, a file path in the checkout, a Job document page, or a taught web address; they are rewritten into `armada-ref:` links and drawn by `ReferenceLink` with tooltip `<Pull request|Issue|Job|File|Document|Link> · <title>`; unknown numbers stay plain. |
| Inline code | `code.armada-prose__code`: mono, padding `0 space-1`, border 1px `border-default`, radius `radius-sm`, bg `bg-overlay`, `fg-default`, wraps anywhere. A web address in code is a link. An inline code that is exactly one of the Job's slash-commands (terminal Jobs only; `ProseCommand`) becomes a pressable `SlashCommand` that SENDS it. |
| Fenced block | `div.armada-prose__run > div.armada-prose__run-block > pre.armada-prose__block` + `span.armada-prose__run-acts`: block mono, padding `space-2 space-3`, border 1px `accent-muted` with 2px `accent` left, radius `radius-md`, bg `bg-sunken`, `pre-wrap`, wraps anywhere (spacious: no border/left bar, bg `fg-default 4%` + inset 1px ring `fg-default 7%`, padding `space-3 space-4`, radius 8, `spacious-meta` 13/20, syntax colouring by language via `ProseCodeColour` = `colourCode`). Acts over the corner: **Copy** (`CopyAct`, always); **Run** (`Play` 16, tooltip + aria `Run`) when the block is a shell fence (or reads as shell) AND the thread provides `ProseRunner`; while running `Square` **Stop**; after a run `RotateCw` **Run again**. A run draws output below (`RunOutput`) or docks a terminal. Acts are `opacity` shown on hover/focus-within of the block (`.armada-prose__block[data-acts]` reserves right padding for 1 or 2 acts). |
| Blockquote | `blockquote.armada-prose__quote`: padding `space-2 space-3`, 2px `accent` left bar, radius `0 3px 3px 0`, bg `accent-faint`, `fg-default`. |
| Rule | `hr.armada-prose__rule` 1px `border-subtle` |
| Table | `table.armada-prose__table`: collapse, 1px `border-subtle`, width = content; `th`: padding `space-2`, bottom 1px `border-default`, bg `bg-overlay`, `fg-muted`, 13/18 weight 500 UPPERCASE tracking 0.04em; `td`: padding `space-1 space-2`, bottom 1px `border-subtle`, top aligned; breaks only at words (paths still break). |
| Images | `img` → `Picture` (inline thumbnail via `ProseAttachments`/Enlargeable when the host gives bytes; media posters for video) — draws only where the host resolves it. |
| Line picks | `ProsePick` (per-paragraph hover bar with the quick responses) is OFF in the Job view (spacious): `picks === null || spacious → null`. The CSS for `p[data-pick]` hot/focus fills exists in SPA ~290-335 but nothing sets `data-pick` here (see Open questions). |

### A2.10 Debug info (inside the person's bubble)
Condition: the message text contains a line exactly `## Debug info`.
`details.armada-session-you__debug` after the words (Prose of the facts under it): summary `summary.armada-session-you__debug-head` — flex, gap `space-1`, `fit-content`, pointer, no marker, `text-2xs` `fg-muted` — `ChevronRight` 12/2 (rotates 90° when open, 120ms) + text **Debug info**.
Open body = Prose of everything after the heading (Run/commands off).

### A2.11 Notes message (inside the person's bubble)
Condition: no inline tags AND the `NotedOpens` context is given AND `notedOf(said)` parses (every line belongs to numbered blocks `N. <ref>` with indented bodies; numbering must run 1..n, any stray line returns `undefined`).
Draws `ol.armada-noted[aria-label="Notes"]` (flex column gap `space-2`, no marker): each `li.armada-noted__note` = head (`span.__n` "N." `fg-muted` tabular · tooltip + `button.armada-prose__link.armada-noted__ref[data-kind]` (accent link look; `file` kind mono) whose text is `notedSaid(ref, root)`) and, if the note has a body, `div.armada-noted__body` (padding-inline-start `space-4`) holding Prose.
`notedSaid`: document `"<section>: <target>"`; sketch `"<title>: <part labels, comma>"`; file `"<path rel. to Job checkout or ~…>:<line|from-to>"`; picture `"<name>"` or `"<name>, region"`.
Tooltip (`opensSaid`): document `Open <page>, revision <n>`; sketch `Open <title>, part picked` / `parts picked`; file `Open <name>, line N` / `lines A-B`; picture `Open <name>` + `, region marked`.
Press → `opens.open(ref)`.

Addenda to A2.9 (spacious Prose geometry, SPA:330-345, 1240-1300): `> p + p` margin-top `space-6` (24); a paragraph next to a fenced block or table: `margin-top: space-8` (32) both ways; fenced blocks, `div.armada-prose__run` and tables get `margin-inline: −20px` (bleed) and `padding-inline: 20px`; a table is `width: calc(100% + 40px)` with first/last cell padding 20px.
The Copy/Run acts of a long block ride (`position: sticky`) down the block's top edge as the thread scrolls, under the head fade, stopping at the block's foot (`.armada-prose__run-block {display: grid}`, SPA ~1180-1225).

## A3 Folded calls

### A3.1 Item and folding rule
A `calls` item = a run of consecutive `tool` rows without `subagent`; id = first row's id (`itemsOf`, A1.4).
Rows removed first in spacious: `UNDRAWN` (A1.4).
A tool row's `text` is `"<ToolName> <input>"` as Fleet wrote it (`Edit crates/store/src/audit.rs`, `Bash cd x && cargo check`, `mcp__server__tool …`).
`namesOf(rows)` = first whitespace-separated word of each row, each once, in first-call order (used only for aria-labels).
Wire: `ThreadItem::Calls{id, rows, names}`.

### A3.2 Three drawings of a calls item
| Case | Condition | Draw |
|---|---|---|
| Live list | `working === true && !asking && it is the LAST item` AND spacious | A3.3 |
| Folded line, working | same condition, base take | `details` closed; mark = `SessionMark working` + `SessionStop` after it (if `onStop`) |
| Folded line, settled | otherwise | `li.armada-session-fold.armada-session-fold--calls > details > summary.armada-session-fold__head` |
`working` passed to `Calls` is `working && !asking && at === items.length − 1` (`SessionThread.tsx:4010`).

### A3.3 Live list (spacious, last item while the agent works)
`li.armada-session-fold.armada-session-fold--live` `aria-label="Tool calls: <names, comma-joined>"` containing `ol.armada-session-live` (flex column, gap `space-2` (8), margin/padding 0, no marker).
One `li.armada-session-live__call` per call in order: flex, align center, gap `space-2`, `spacious-meta` 13px / 20px, colour `fg-subtle`; the newest (`data-now`) `fg-default`.
| Part | Draw |
|---|---|
| Mark cell `span.armada-session-live__mark` | inline-flex, flex none, width `space-4` (16), centered; holds `SessionMark state="working" said="Working"` (16px `WorkingMark`, A1.9) ONLY on the newest call; empty (16px gap kept) for the others |
| Line `span.armada-session-live__line` | min-width 0, single line, ellipsis. Text: a `Bash <cmd>` call → `Running <cmd>` (newest) or `Ran <cmd>` (older) with the WHOLE command (the line is cut at the column's end by ellipsis); any other call → `callLine(row, isNewest)` (A3.5) |
| Stop | after the newest line's text, `SessionStop` (A1.10) when `onStop` |
Not a press, not focusable, no keys.
Earlier calls in the run stay listed (the list grows as calls arrive; when the next non-call item arrives or the agent stops, the item becomes the folded line A3.4).

### A3.4 Folded line (`summary.armada-session-fold__head`)
| Aspect | Value |
|---|---|
| Element | `details` (native disclosure; Enter/Space toggle; summary is focusable) |
| Summary | flex, align center, gap `space-1`, `width: fit-content`, `max-width: 100%`, pointer, no marker; spacious: `spacious-meta` 13px/20px, colour `fg-muted`; hover `fg-default`; open `fg-default` (SPA:809); `aria-label="Tool calls: <names>"` |
| Chevron | `ChevronRight` 12/2 — **hidden in spacious** (`display:none`); base rotates 90° when open, 120ms |
| Mark | settled: `Wrench` 12/2; working (base only): `SessionMark working` |
| Line text | `span.armada-session-fold__names.armada-session-fold__names--line`: sans, ellipsis, nowrap = `callsLine(rows, working)` (A3.5) |
| Focus | `outline: 1px solid accent`, offset 2px |
| Opened | `ol.armada-session-fold__calls`: flex column gap `space-1`, margin-top `space-1`, padding-left `space-4`, no marker; each call is a `Command` (A3.6) |

### A3.5 The line text (`calls-line.ts`) — the exact algorithm
Inputs: the rows' `text`.
`toolOf`: tool = text before first whitespace (if it contains `__`, the part after the last `__`), detail = rest trimmed.
Families by tool name: `Edit, Write, MultiEdit, NotebookEdit`→edit; `Read`→read; `Grep, Glob`→search; `Bash`→run; `Agent, Task`→agent; `WebFetch, WebSearch`→web; `start_watch`→watch; `AskUserQuestion`→ask.
Anything else = unknown.
Verbs [running, finished]: edit [Editing, Edited]; read [Reading, Read]; search [Searching, Searched]; run [Running, Ran]; agent [Briefing, Briefed]; web [Reading, Read]; watch [Watching, Watched]; ask [Waiting for you to answer, You answered].
Rule: take the LAST call (in the group) whose family is known → `last`; if none known, the line is the LAST call's tool name (`""` if empty).
`of` = the calls of family `last`.
If `last = agent` and the last such call has a non-empty detail, the line is that detail clipped to 40 chars (`clip`: >40 → first 39 + `…`).
Otherwise `verb <about>` where:
| Family | about |
|---|---|
| edit, read | set of `baseName(detail)` (first word of detail, last `/` segment) over `of`; >1 distinct → `multiple files`; exactly 1 → that name; none → `a file` |
| search | `clip(last.detail)` |
| run | distinct details >1: every one a check (`/^(armada check|cargo (check|test|clippy)|pnpm (test|exec (vitest|tsc))|npm test|vitest|tsc|pytest)\b/` on the first non-scene-setting segment) → `<n> checks`, else `multiple commands`; one detail → `clip(commandOf(detail))` = first word (basename of path) plus the second word if the first is in {git, cargo, pnpm, npm, yarn, npx, bun, armada, gh, make, go, uv, brew} and the second does not start with `-`; shell segments split on `&&`, `||`, `;`, newline; leading words that are `NAME=value`, and segments starting `cd pushd popd export set unset source` are skipped |
| agent | `an agent` |
| web | `the web` |
| watch | `clip(last.detail)` |
| ask | empty → the line is the bare verb |
Examples: `Edited foo.ts`; `Edited multiple files`; `Ran cargo check`; `Running 3 checks`; `Searched <pattern clipped 40>`; `Read the web`; a lone unknown tool `Foo` → `Foo`.
`callLine(row, working)` (used by the live list): for a family-less tool → `clip(row.text)` (whole text clipped 40); known → `callsLine([row], working)`.
Tense: `working` true → first verb; false → second.

### A3.6 One call (`Command`, 1324; also `row.kind==="command"` 2303 and `ran` 1634)
`Command` = `li.armada-session-command` (flex, align baseline, gap `space-2`, min-width 0) with `code.armada-session-command__text` (spacious: sans 13/20; base mono 12/16; `fg-muted`; `pre-wrap`; wraps anywhere) holding the raw `text`; if `fromPhone` a `span.armada-session-command__door` (inline-flex, `fg-muted`) with `Smartphone` 12/2 and tooltip `From phone`.
When the host gives `onTeach` (Teach open) the `li` is `tabIndex=0`; right press, ContextMenu key or Shift+F10 opens `useTeachMenu("Command")`: `DropdownMenu` with a label `Command` and one item `Teach` (GraduationCap) → `onTeach({kind:"command", text, at})`; closing focuses the `li` back.
The `row.kind === "command"` row (a command the person typed in a terminal Job, `commandNames`/`onCommand`): `li.armada-session-command > code` with the text only; no Teach.

**`ran` row (RanRow)** — a command the person ran in the Job's terminal, whose lines were sent to the agent:
| State | Draw |
|---|---|
| running (`row.running`) | `li.armada-session-fold.armada-session-fold--ran[data-running]`; head is a non-interactive `div.armada-session-fold__head` (`aria-label="Running: <first line>"`, cursor default) with `TerminalMark running` (`SquareTerminal` 16/2, colour `run-running` #4FB8D9, opacity pulse 1↔0.45 over 1600 ms), `code.armada-session-fold__target` (first line of the command; sans 13/20 in spacious; `fg-muted`; ellipsis), and `span.armada-session-fold__elapsed role=timer` (mono, tabular, `fg-subtle`, margin-inline-start `space-1`; tooltip `Running for`; text `m:ss` or `h:mm:ss` from `row.since`; hidden if `since` doesn't parse; ticks each second). |
| ended | `li.armada-session-fold--ran` with `details > summary.armada-session-fold__head` (`aria-label="Exit <code>: <first line>"` or `"Stopped: …"` when `code === null`), chevron (hidden in spacious), `TerminalMark ended` (`CircleCheck` 16 `run-passed` #4FAF7C for code 0; `CircleX` 16 `run-failed` #E97878 for non-zero; `run-stopped` #9BA3AC for stopped (code null, stopped≠false)), target, and **spacious only, code ≠ 0**: `span.armada-session-fold__exit` (aria-hidden; `flex:none; margin-inline-start:auto`; 13px weight 500 tabular `run-failed`) = `Exit <code>` or `Stopped`. Tooltip of the mark: `Exit N` / `Stopped` / `Ended by a signal` (code null and stopped=false). |
| opened | `pre.armada-session-fold__tail role="log" aria-label="Sent to the Job"` — all lines the agent was sent: margin-top `space-1`, padding `space-1 space-3`, border 1px `border-subtle`, radius `radius-md`, bg `bg-sunken`, mono 12/16 `fg-muted`, pre-wrap |
| peek | while folded (and not running): `pre.armada-session-fold__peek` = last 3 lines of `row.tail` (trailing whitespace trimmed): margin `space-1 0 0 space-4`, padding-left `space-2`, left border 1px `border-subtle`, mono 12/16 `fg-subtle`; hidden when the fold is open |
| landed animation | if the run was seen running in this mount and ended: the mark animates `armada-ran-settle` 220ms (from opacity .4, scale .7) and the peek `armada-ran-land` 300ms (from opacity 0, translateY(−4px), clip-path inset(0 0 100% 0)); none under reduced motion |
| failed card (spacious) | `:has(.armada-terminal-mark[data-result="failed"])`: the whole `li` becomes a card: padding `space-3 space-4`, radius 8, bg `run-failed 7%`, inset 1px ring `run-failed 18%`; head `gap space-2; width:auto; colour fg-default`; target `fg-default`; peek/tail lose border/bg/padding and get margin `space-2 0 0 24px`, 13/18... exactly `font-size text-xs (13px) / 18px`, `fg-muted`. A passing run stays a line. |
Wire: `SessionRow::Ran{command, code, tail, running, since}`.

## A4 Subagents

### A4.1 Item
A `tool` row with `subagent: ToolSubagent` (`@armada/protocol`: `{id, agent_type?, task, latest?, state: "running"|"finished"|"cut_off", since, ended?}`) is its own `subagent` item (ends the calls run before it and after it).
Its own calls are never rows of the thread.
Wire: `ThreadItem::Subagent`; `ToolSubagent` is Fleet's `ipc::ToolSubagent`.

### A4.2 `SubagentRow` (`li.armada-session-subagent[data-state]`)
`li`: flex.
Spacious: `margin-inline: −20px`; two `data-handoff` rows in a row have `margin-block-start: space-1 − turn-gap` (i.e. −24px at default spacing) so a run of handoffs reads as one list of lines.
| State | Draw |
|---|---|
| running | `data-handoff`. Wrapper `.armada-session-subagent__press` is a `button` (aria-label `Subagent <task>, running`) when `onOpenSubagent` is given (it is: `sessions.tsx:3211` opens the subagent's layer `read({kind:"subagent", id})`), else a `div`. Row layout: flex row, align center, gap `space-2`, padding-block `space-1`, no border, transparent bg, `fg-subtle`, `text-sm` (14/20; spacious padding-inline 20px). Content: `Bot` 14/2 (`__bot`) · `span.armada-session-subagent__handoff` text **`Started a subagent: <task>`** (ellipsis, nowrap). Hover: colour `fg-default`. |
| finished / cut_off | Press = card: flex column gap `space-1`, padding `space-2 space-3` (spacious inline 20px), border 1px `border-subtle`, radius `radius-md`, bg `bg-sunken`, `fg-default`; hover (button only) border `border-strong`, bg `bg-hover`; `aria-label="Subagent <task>, finished"` / `"…, cut off"`. Head `span.__head`: flex, align center, gap `space-2`: `StepActivityMark` (finished → `advanced` Check 12 `step-advanced`; cut_off → `killed` Power 12 `fg-default`) with tooltip `Subagent finished` / `Subagent cut off` plus ` · <agent_type>` when given; `span.__task` (flex 1, ellipsis, nowrap, `text-sm` 14/20, weight 500); `span.__ran` (flex none, `text-xs`, tabular, `fg-subtle`) = elapsed `elapsedWords(ended − since)` e.g. `7s`, `1m 05s`, `1h 02m` (omitted if either end won't parse; for cut_off without `ended` nothing); `ChevronRight` 12 `fg-muted` (`__go`) only when pressable. |
Focus ring `1px accent`, offset 2px. Keys: Tab/Enter/Space on the button.

### A4.3 `SubagentsCard` (pinned live view; `li.armada-session-thread__subagents`)
Drawn after the working-line slot (A1.4 #9) while ≥1 subagent item is `running` (`running` = all running subagent items, in order).
The `li` is `position: sticky; bottom: space-2 (8px); z-index: 1`; spacious `margin-inline: −20px`.
The working line is NOT drawn while any subagent runs.
| Part | Draw |
|---|---|
| `section.armada-subagents aria-label="Subagents running"` | flex column, border 1px `border-subtle`, radius `radius-md`, bg `bg-raised`, `box-shadow: --shadow-overlay`, overflow hidden |
| Head | flex, align center, gap `space-1`, padding-inline-end `space-1` |
| Toggle | `button.armada-subagents__toggle` flex 1, min-height 28, padding `0 space-3` (spacious inline 20px), transparent, `fg-default`, `text-sm` weight 500; hover bg `bg-hover`; `aria-label` = `1 subagent running` / `N subagents running`; `aria-expanded`. Content: `span[role=img aria-label=Running]` with `WorkingMark` 14 (colour `status-running`) · text (same as aria-label) · `ChevronUp` 14/2 when folded, `ChevronDown` when open (`fg-muted`). Opens upward from the foot "as Needs you does". |
| Stop | `SessionStop` after the toggle only when `working && onStop` (ends the whole turn) |
| Open list | `ul.armada-subagents__rows` (border-top 1px `border-subtle`; between rows 1px `fg-default 5%`) one `li` per running subagent: grid `14px minmax(0,1fr)`, column-gap `space-2`, row-gap `space-1`, padding `space-2 space-3` (spacious inline 20px). Cell 1 `StepActivityMark running pulsing` (CircleDot 12 `step-running`, inner dot pulses opacity 1↔.45 scale 1↔.7 over 1600ms) with tooltip `Subagent running` (+` · <agent_type>`); cell 2 line: `span.__task` (flex 1 ellipsis `text-sm`) + `span.__ran` (flex none `text-xs` tabular `fg-subtle`, elapsed `elapsedWords(now − since)`, ticks each second only while open); below (col 2) `span.__latest` = `subagent.latest` (its last call; `text-xs`, `fg-subtle`, ellipsis) when present. Whole row is a button (`aria-label="Subagent <task>"`) opening its thread when `onOpen`. |
Gone when the last running subagent returns; its `SubagentRow` then redraws as the finished card (A4.2). While subagents run, a person message is `Queued` (A2.2).

## A5 Checks

### A5.1 Item and states
Check rows (`kind:"check"`, `{id, at, name, run: number, state, target?}`; wire `SessionRow::Check`).
Folding: consecutive Check rows with the same `name` and nothing between are ONE `checks` item (A1.4).
State → mark via `StepActivityMark` (12px, `strokeWidth 2`) and word: `running` → `running` activity (CircleDot, `step-running` #4FB8D9, pulsing dot) word `running`; `passed` → `advanced` (Check, `step-advanced` #4FAF7C) `passed`; `failed` → `failed` (X, `step-failed` #E97878) `failed`; `timed_out` → `failed` mark, word `timed out` (a budget kill is not a failed test).
Tooltip on the mark = the word; the word is also a visually hidden `span.armada-step-mark__name` for screen readers.
Mark cell is `width: space-4` (16) inline-flex centered.

### A5.2 Lone run (`rows.length === 1`)
`li.armada-session-fold` containing the head:
| Host gives `onOpenCheck` (sessions.tsx:3210 → opens run `run` of this Job's Check in a layer) | Head is `button.armada-session-fold__head.armada-session-fold__head--press` `aria-label="Check <name> <target>, <word>"` (no target → `Check <name>, <word>`); press → `onOpenCheck(run)` |
| otherwise | head is a `span.armada-session-fold__head` (tabIndex 0 only if Teach is on) |
Head content: mark · `span.mono` name · `code.armada-session-fold__target` target (mono, `fg-subtle`, ellipsis) if present.
Base size 12/16 (`text-2xs`); spacious: head 13/20 `fg-muted`, the `.mono` name span becomes sans 13/20 (SPA:253).
**Spacious press card**: `--press` heads are drawn as a card: gap `space-2`, min-width `min(100%, 280px)`, padding `space-3 space-4`, radius 8, bg `fg-default 4%` + inset 1px ring `fg-default 7%`, `text-sm` 14/20 `fg-default`, hover bg `fg-default 7%`, svg inside 16×16 `fg-muted`.
(So a lone Check, a shown window and a `judged`/`file` press are all cards.)
### A5.3 Several runs (`rows.length > 1`)
`li.armada-session-fold.armada-session-fold--checks` > `details` > `summary.armada-session-fold__head` (`aria-label="Check <name>, latest <state>"`; chevron hidden in spacious; head font inherits parent so same size as a lone press) with the LATEST run's mark and the name (`span.mono`); NO count.
Opened: `ol.armada-session-fold__calls` with one `li` per run in order: a press button (if `onOpenCheck`, `aria-label` as A5.2) or span: mark · target `code` (if any) · `Stamp still` (`armada-session-fold__at`: flex none, `fg-subtle`).
Right press / ContextMenu / Shift+F10 on the item opens the Teach menu (`useTeachMenu("Check")`, item `Teach`) when Teach is on → `onTeach({kind:"check", text:"<name> <target> <state>" whitespace-collapsed, at: latest.at})`.
Running Checks: the item's mark pulses until a later row replaces it (the wire replaces the row by id). No timer is drawn on a Check row.

## A6 Fleet turns

### A6.1 Folding (A1.4 rules; `itemsOf` 1113-1150, `FleetTurn.tsx`)
A Fleet notice (`message`, `from:"session"`, `sender.id === "fleet"`, not in `keep`) opens a `fleetTurn` item.
It swallows the following consecutive `tool` rows (a tool row that carries a `subagent` is swallowed too, and then is NOT drawn as a subagent item) and `agent` messages; a `you` message or any other row ends it.
The LAST agent message of the swallowed reply is lifted out and drawn after the fold as a normal agent row.
`handled = reply.length > 0` (the agent replied).
Batch: a notice with `batch` (name the person taught) holds several notices, one per line of `text`.

### A6.2 Drawing (`li.armada-fleet-turn`)
| Part | Spacious (shipped) | Base |
|---|---|---|
| `li` | `role="region" aria-label="Fleet turn"`, `data-open`; 13px/20 `fg-subtle` | 12/18 `fg-muted` (root `--text-2xs`/`--leading-xs`) |
| Row press | `button.armada-fleet-turn__row` `aria-expanded aria-controls=<body id>`: flex, align center, gap `space-2`, `margin-inline: −8px`, padding `space-1 space-2`, radius `radius-md`, transparent; hover/open: `color: fg-muted`, no bg; base hover bg `bg-sunken`; max-width 100%; focus ring 1px accent offset 2px | gap `space-2`, same |
| Chevron | `ChevronRight` 12 — **hidden** | rotates 90° when open (120ms) |
| Marks (12px; svgs 14×14 in spacious) | `BellRing` in `span[role=img aria-label="Fleet notice"]` (tooltip `Fleet notice`); if batched `Rows3` (tooltip `Batched`); label; if `handled` `Check` (tooltip `Handled`) | |
| Label | `span.armada-fleet-turn__label`: sans (base mono), ellipsis, nowrap = `notice.batch` (the batch's name) else `summaryOf(notice.text)` = the notice's first sentence up to the first line break or `. ` without a trailing full stop; tooltip = the notice's full text | |
| Body (when open) | `div#<id>.armada-fleet-turn__body`: flex column, gap `space-2`, margin `space-2 0 0 6px`, padding-left `space-4`, left border 1px `border-subtle`. Content in order: the notice (`p.armada-fleet-turn__notice` — for a batch one `p` per line); then each swallowed row: a tool row → `p.armada-fleet-turn__tool` (the raw tool text); a message → `div.armada-fleet-turn__reply` with `Prose`. Notice/tool/reply: sans 13/20 `fg-muted` | margin-inline-start `space-4`, padding `space-1 space-2`, left border 1px `border-default`; notice/tool mono; reply `text-xs` `fg-muted` |
| Open state | local `useState(false)`; closed again on remount (not persisted) | |
Press toggles.
Keys: Enter/Space on the button.
Context menu: right press on the `li`, or ContextMenu / Shift+F10 on it, opens `DropdownMenu triggerLabel="Fleet notice"` with `noticeEntries(kind, muting, teaching)` (A2.5), available when `onTeach` is given or (`notice` kind known and `onMuteNotice` given); Teach → `onTeach({kind:"notice", text, at, ids:[kind]?})`; a mute choice → `onMuteNotice(kind, "job"|"repo"|"hour")`; closing returns focus to the row button.
Wire: Rust `thread_view.rs` folds Fleet turns the same way into `ThreadItem::FleetTurn` (`crates/ipc/src/job_thread.rs:217`); Electron derives it in TS from `message` rows.

### A1.16 Rows drawn just above the thread (inside `.armada-session-frame__centre`, before `SessionThread`; `sessions.tsx:3050-3066`)
In order, inside `ThreadKeys` (B): `Refused` (when Fleet refused the last act on the Job: `Alert tone="escalated"` with Fleet's words, else nothing); `ForkedFrom` (a Job forked from another and not a moved one): `details.armada-forked-from` (`aria-label="Forked from <title>"`, flex none, bg `bg-raised`, radius `radius-md`) closed by default, `summary.armada-forked-from__head role=button` text **`Forked from <title>`** (padding `space-2 space-3`, `fg-muted`, `text-xs`, pointer) — opening calls `watch(origin.id)` and draws a read-only `SessionThread` (BASE look, no context) of the origin's rows up to and including the fork message, in `div.armada-forked-from__rows` (flex, `max-height: 48*6 = 288px`, padding `0 space-3 space-3`); then `startedBy` (the Job another Job's agent started): `button.armada-started-by` text **`Started by <title>`** (press opens that Job); spacious: `align-self: stretch`, margin `space-4 (max(16px,(100%−outer)/2)+20px) 0`, no padding/bg/radius, `fg-subtle` 13/20, hover `fg-default` + underline offset 3px.
Base look (`StartedBy.css`): `flex:none`, padding `space-2 space-3`, no border, radius `radius-md`, bg `bg-raised`, `fg-muted`, `text-xs` 13/18, left aligned, pointer; hover `fg-default`.

## A7 Other rows

Spacious has no override for any row below unless stated; they draw with the base CSS in `SessionThread.css` / their own CSS file at the root scale (`--text-2xs` 12/16, `--text-xs` 13/18, `--text-sm` 14/20).
Row `<li>`s sit in the 28px row gap; none is focusable unless it holds a button.

### A7.1 `woke` (Fleet woke the agent; `Row` 2047; `rowsOfThread` has no case for it — no wire producer)
`li.armada-session-woke`: flex, align center, gap `space-2`, padding-block `space-1`, **dashed** 1px `border-default` top and bottom, `fg-muted`, mono 12/18. `Server` 12/2 in `span.__mark role=img aria-label="Fleet woke it"` (tooltip `Fleet woke it`) · `span.__why` (flex 1, `row.why`) · stamp (`armada-session-woke__at`, flex none, `fg-subtle`).

### A7.2 `not_started` (`StartFailedRow`)
`li.armada-session-start-failed role=region aria-label="Failed to start"`: grid `auto minmax(0,1fr) auto auto auto auto`, align center, column-gap `space-2`, padding-block `space-1`, dashed 1px `border-default` top and bottom, colour **`status-completed-failed`** (#E97878), mono 12/18. Cells: `X` 12/2 in `span[role=img aria-label="Failed to start"]` (tooltip `Failed to start`) · `span.__why` = `Did not start: <cause>` (wire `cause`; TS adds the prefix) · col A: if `log` non-empty `Button ghost sm iconOnly aria-label="Error lines"` (tooltip `Error lines`, `aria-expanded`, `aria-controls`) with `ChevronRight` (closed) / `ChevronDown` (open) 12/2 · col B: `Start again` button (`RotateCcw` 12/2, tooltip + aria `Start again`) if the host serves `startAgain` (always for a live Job) → `onStartAgain()` · col C: `Report` button (`Flag` 12/2, tooltip + aria `Report`) → `onReport(button element)` (host opens the report sheet for the Job) · `Stamp still` (`fg-subtle`).
Columns A–C keep their width when empty (`fg-muted`, inline-flex centered) so the presses line up.
Open: `pre.__log` spanning all columns: margin-top `space-1`, padding `space-2`, radius `radius-md`, bg `bg-sunken`, `fg-default`, inherits mono 12/18, pre-wrap, wraps anywhere; text = `log` lines joined by `\n`.
Open state is local.
The "message from the person is the retry" (the composer).

### A7.3 `act` (`Row` 2076)
Producer: mocks only (no wire).
`li.armada-session-act`: flex, align center, gap `space-2`, padding `space-1 space-2`, border 1px `border-strong`, radius `radius-md`, bg `bg-sunken`, `fg-default`, 12/18. `row.icon` glyph 12/2 in `span[role=img aria-label=<row.said>]` (tooltip `row.said`) · `code.__text` (flex 1, mono 13/18, wraps anywhere) · stamp (`fg-subtle`).

### A7.4 `window` (a walk/window shown; `Row` 2090; wire `SessionRow::Window`)
`li.armada-session-fold` > `button.armada-session-fold__head.armada-session-fold__head--press` `aria-label="Open window <title>"`, `AppWindow` 12/2 + `row.title` → `onOpenWindow(row.url)`.
Drawn as the spacious press card (A5.2 card spec).
NOTE: `show_window` tool calls are removed by `UNDRAWN`, so this row stands for the window.
In Job view walks open as Job tabs; the host maps the url.

### A7.5 `requested` (`RequestedRow`; wire `SessionRow::Requested` / `crates`' request row)
`li.armada-session-fold.armada-session-fold--requested > details`: summary `aria-label="<You|AgentName>: <status|No answer> <METHOD> <url>"`, chevron (hidden in spacious), `RequestMark` (12/2 in `span[role=img]` with tooltip = `<status>` or `No answer` / `No answer: <error>`; glyph `Check` `step-advanced` for status <400, `X` `status-completed-failed` for ≥400, `Unplug` same red for none) · `span.__by` (`fg-subtle`; `You` when `by === "owner"` else the agent name) · `code.__method` (mono, `fg-default`, flex none) · `code.armada-session-fold__target` (url; `fg-muted`; ellipsis) · `span.__ms` (mono `fg-subtle`, margin-left `space-2`): `<n> ms` under 1000 else `<n.n> s`.
Opened `div.__body` (flex column, gap `space-1`): `p.__error` if `error` (12/16 `fg-muted`); `pre.armada-session-fold__tail role=log aria-label="Response"` if response non-empty; `p.__cut` **`Cut at 16 KiB`** if `truncated`; `Button ghost sm` **`Open in Requests`** if the host provides `RequestOpen` (the Job view does) → opens the request in Requests.

### A7.6 `saved` (`SavedRow`)
Wire row `saved` (`sessions-wire.ts:1291`: `name`, `collection`, `collection_id`, `request`); the component's own comment calls it "proposed, not on the wire" but `rowsOfThread` does map it.
`li.armada-session-saved aria-label="Saved <name> to <collection>"`: flex, align center, gap `space-3`, padding `space-2 space-3`, border 1px `border-subtle`, radius `radius-md`, bg `bg-sunken`, 13/18 `fg-default`.
`Save` 12/2 `fg-muted` in `span[role=img aria-label="Saved request"]` (tooltip same) · text **`Saved <name> to <collection>`** (flex 1) · stamp still (`fg-subtle`) · `Button ghost sm` **`Open in Requests`** when `SavedOpen` is provided.

### A7.7 `started` (`StartedRow`; wire: derived from the Job-started record)
`li.armada-session-started`: relative, flex, align center, gap `space-3`, padding-inline-end `space-3`, radius `radius-md`, **bg `fg-default`, text `fg-inverse`**; hover bg `accent`.
The open press `button.__press` (aria-label = the started Job's title) covers the row (flex 1, padding `space-2 0 space-2 space-3`; `::after` overlay `inset:0` takes the focus ring `1px accent` offset 2px): `GitFork` 14/2 (fork) or `CornerUpRight` 14/2 (started) · eyebrow **`Forked`** or **`Started`** (12/16 weight 600 uppercase tracking 0.04em) · title (flex 1, `text-sm` weight 500, wraps anywhere) · `Stamp still` (mono 12/16, above the overlay so it still flips).
If the started Job is held on its start card (`row.start`): `StartAct` before the chevron: `Button secondary sm` **`Start`** (`aria-label="Start <title>"`) → `start.onStart()`; while starting: `LoaderCircle` 12 (tooltip `Starting`) + text **`Starting…`**.
Then `ChevronRight` 14/2 (translates `space-1` right on hover).
Press anywhere else → `onOpenSession(session.id)`.

### A7.8 `moved` (`MovedRow`; TS-derived)
`li.armada-session-moved > button.__press` `aria-label="Moved <to|from> <repository>"`: inline-flex, align center, gap `space-2`, padding `space-2 space-3`, no border, radius `radius-md`, bg `bg-raised`, `fg-muted`, 13/18, hover `fg-default`.
`FolderGit2` 12/2 · `Moved to` / `Moved from` · `span.__repository` (mono, ellipsis) · `ChevronRight` 12. Press → `onOpenSession(row.session)` (the other Job).
A move carries the whole old conversation into the new thread under this `Moved from <repo>` row; the old thread ends with `Moved to <repo>`.

### A7.9 `judged` (`Row` 2189; TS: judge records)
`li.armada-session-fold > button.…--press` `aria-label="Judged <title>"`: `Scale` 12/2 · `Judged ` + `span.mono` title · `Stamp still` (`armada-session-act__at`) → `row.onOpen()` (opens the artifact at its judge notes).
`clean` is on the row but not drawn.
Spacious card look (A5.2).

### A7.10 `divider` (`Row` 2205)
`li.armada-session-divider role="separator"`: `height: 0`, margin-block `space-4`, top border 1px `border-subtle`. Where an earlier Job's carried turns end and this Job's begin (hand off).

### A7.11 `waiter` (`WaiterRow`)
`li.armada-session-waiter > button.__press` `aria-label="<title>, <eyebrow lower-case>"` (`changed` → `<title>, condition changed`): flex column, gap `space-1`, padding `space-2 space-3`, radius `radius-md`, **bg `fg-default`, text `fg-inverse`**; hover bg `accent`; focus ring 1px accent offset 2px.
Line (`flex`, gap `space-3`): `Link` 12/2 in `span[role=img]` (tooltip/aria `Waits on this Job`; `waiting: true` → `Waits on that Job`) · eyebrow (12/16 weight 600 caps) = event `waits` → **`Waits on you`** (`waiting` → **`Waits on`**), `changed` → **`Condition changed`** (or **`You changed it`** when `by === "you"`), `started` → **`Started`** · title (flex 1, `text-sm` weight 500) · `Stamp still` (mono 12/16) · `ChevronRight` 14/2 (nudges right on hover).
Under it, if `says`: `span.__facts` (flex, gap `space-2`, mono 12/18) with the text (first line of the waiting Job's brief, or where it started from); if `from` and `to`: `from` `ArrowRight` 12/2 `to` in the same style.
Press → `onOpenSession(session.id)`.

### A7.12 `linked` (`LinkedRow`)
`li.armada-session-linked[data-event] > button.__press` `aria-label="<title>, <said>"`: flex 1, align center, gap `space-3`, padding `space-2 space-3`, border 1px `border-subtle` (**`status-awaiting-review` for `needs`**), radius `radius-md`, bg `bg-sunken`, `fg-default`, mono 13/18; hover border `border-strong`, bg `bg-hover`.
Content: `Link` 12/2 (tooltip/aria `Linked Job`, `fg-muted`) · title (flex 1, weight 500, ellipsis) · event glyph 12/2 with tooltip = word: needs `Eye` **Needs you** (`status-awaiting-review`), done `Power` **Done** (`step-advanced`), merged `GitMerge` **Merged** (`step-advanced`), started `CircleDot` **Started** (`fg-muted`) · `Stamp still` (`fg-muted`, 12/16) · `ChevronRight` 12/2 (nudges).
Press → `onOpenSession(session.id)`.
Linked needs are not relayed to the parent Job's other surfaces (it pulses on its own row only).

### A7.13 `operator` (`OperatorRow`; no wire producer found; see Open questions)
`li.armada-session-operator`: flex, align baseline, gap `space-2`, padding `space-1 space-3`, `fg-muted`, mono 13/18. `Bot` 12/2 in `span[role=img aria-label=Operator]` (tooltip `Operator`, `align-self:center`) · text (flex 1, the exact words) · `Stamp still`.

### A7.14 `retro` (`SessionRetroRow`; wire `SessionRow::Retro`, replaced by id as Fleet writes)
`li.armada-session-retro role=region aria-label="Retro"` `data-stage`: flex column, gap `space-3`, padding `space-3 20px`, border 1px `edge-card` (= `border-default`) with a **2px `border-strong` left edge**, radius `radius-md`, bg `surface-card` (= `bg-raised`).
Memoised by row value.
| Stage | Draw |
|---|---|
| `asking_agent`, `reading_record`, `writing` | head (flex wrap, gap `space-2`, 13/18 `fg-muted`): `Rewind` 12/2 (opacity 1↔.5 over 1600 ms; static .5 under reduced motion) · `span[role=status]` weight 500 `fg-default`: **`Asking the agent what got in the way`** / **`Reading what Fleet kept of the Job`** / **`Writing the retro`** · `span.mono[role=timer aria-label="<elapsed> in this stage"]` = `elapsedWords(now − stageSince)` (`7s`, `1m 05s`, `1h 02m`; ticks per second) when `stageSince` exists · during `writing` and `characters > 0`: `span.__hint` right-aligned `<n> characters` (tabular). During `writing` with items: `ol[aria-label="Items so far"]` of cards (below). |
| item card | `li` flex column, gap `space-2`, padding `space-2 space-3`, radius `radius-sm`, bg `surface-well` (= `bg-sunken`); route line (`LessonRoute`: who + where it lands); `h3` title (`text-base` 15/22 → root 16/24, weight 600); `p` what (`text-sm`, `fg-muted`); fix block (padding `space-2 space-3`, 2px `accent` left edge, radius `radius-sm`, bg `bg-sunken`): label **`What would change`** (12/16 caps `fg-subtle` weight 500) + text; a field is absent until complete; each field fades in over 180 ms (none under reduced motion); the last item shows placeholders (`bg-hover` bars: route 40% wide, title 70%, what 2 lines, fix block) for missing fields and the line **`Still being written`** (12/16 `text-meta`). |
| `written` | gap `space-2`, padding-block `space-2`: head `Rewind` · **`Retro <n> written · <k item|items>`** (`Retro written` when no number; ` · …` omitted when 0 items) · `Button ghost sm` **`Open retro`** (right-aligned; only with a number and `onOpenRetro`) → opens the Retros page; below `ul[aria-label=Items]` disc list of item titles (padding-left `space-4`, 13/18 `fg-muted`). |
| `failed` | head `Rewind` + **`The retro was not written`**; `Alert tone="escalated"` with `row.why` or **`Fleet gave no reason.`** |

### A7.15 `disputed` (`DisputedRow`; wire `SessionRow::Disputed`)
`li.armada-session-disputed role=region aria-label="Notice disputed"`: flex wrap, align center, gap `space-1 space-2`, padding-block `space-1`, dashed 1px `border-default` top/bottom, `fg-muted`, mono 12/18. `DisputedMark` (`MessageSquareWarning` 12/2, tooltip/aria **`Disputed`**) · `span.__why` (flex-basis ≈100%, `fg-default`, wraps anywhere) · `span.__spotted`: `called` → **`Said by the agent`**, `read` → **`Read by Fleet from the agent's replies`** · if `review`: `button.__review` **`Review job`** + `ChevronRight` 12 (`accent`, hover `accent-hover` + underline, no bg/padding; `aria-label="Open the review job"`) → `onOpenSession(review)` · `Stamp still` (`fg-subtle`).
The disputed notice itself stays a `FromSession` band (A1.4 `keep`) carrying `DisputedMark`.

### A7.16 `change` (`Row` 2210; TS from timeline events)
`li.armada-session-change role=region aria-label="<what> changed"`: flex, align center, gap `space-3`, padding `space-2 space-3`, radius `radius-md`, **bg `you` (#E8C547 Dark / #8F7000 Light), text `fg-inverse`**.
Eyebrow `span.armada-session-lease__eyebrow` (12/16 weight 600 caps) = `Workflow` / `Model` / `Effort` · `span.__facts` (flex 1, mono 12/18, gap `space-2`): `from` `ArrowRight` 12/2 `to` · stamp (`.armada-session-lease__at`, mono 12/16).
The agent was told in the same moment.

### A7.17 `suggest` (`Row` 2223; wire `plan_changes`)
`li.armada-session-suggest role=region aria-label="Changes to the plan"`: flex, align start, gap `space-3`, padding `space-2 space-3`, border 1px `you-edge`, radius `radius-md`, bg `color-mix(you 14%, bg-raised)`, `fg-default`.
Eyebrow **`Plan changes`** · `ul.__lines` (flex column gap `space-1`): per line `li` (flex wrap, align center, gap `space-2`, 13/18): verb glyph in `span[role=img aria-label=<verb>]` (tooltip verb) — `Renamed` Pencil, `Removed` Trash2, `Added` Plus, `Moved` MoveRight, `Subagent` Bot, `Note` MessageSquare (12/2) · `span.__where` (mono 12/16) · `<s>from</s>` (opacity .7) · `ArrowRight` 12 when both `from` and `text` · `span.__text`.
Stamp (lease `__at`).

### A7.18 `lease` / `release` (`Row` 2261; wire `SessionRow::Lease|Release`, `{slot, branch}`)
Spacious: `li.armada-session-lease[data-line] role=region aria-label="Slot leased"|"Slot released"`: flex, align center, gap `space-2`, `width: fit-content`, padding `space-1 space-3 space-1 10px`, margin-inline-start −10px, radius 8, bg `slot-held 10%` (slot-held = `status-running`), `relative`, 13/20 `fg-muted`.
`KeyRound` 14/2 (`slot-held`) · `span.__said` (weight 500, `slot-held`) **`Leased slot <n>`** / **`Released slot <n>`** · `span.__branch` (inline-flex, gap `space-1`, `fg-subtle`, ellipsis nowrap) with `GitBranch` 12/2 + branch name · stamp absolutely positioned at `left: calc(100% + 12px)`, sans 13px tabular `fg-subtle`, `opacity 0`, → 1 while the row is hovered (120 ms).
Base: a filled card (bg `slot-held`, text `fg-inverse`, padding `space-2 space-3`, radius `radius-md`) with the eyebrow **`Slot leased`/`Slot released`** and two chips (`role=img`, aria `Worktree slot <n>` with `KeyRound` + number; `Branch <name>` with `GitBranch` + name; border 1px `fg-inverse`, radius `radius-sm`, mono 12/18) and a stamp.

### A7.19 `command` — see A3.6.

### A7.20 `compaction` (`Row` 2310; wire `SessionRow::Compaction`)
`li.armada-session-fold > details > summary.armada-session-fold__head`: chevron (hidden in spacious), `Layers` 12/2, text **`Conversation compacted`**.
Opened `div.armada-session-fold__body` (margin-top `space-1`, padding-left `space-4`, `text-sm`, `fg-muted`) holding `Prose` of the summary text (15/24 in spacious).

### A7.21 `reasoning` (`Row` 2331; no wire producer found; see Open questions)
`li.armada-session-fold.armada-session-fold--reasoning > details[open when live]`: summary head (spacious gap `space-2`; chevron hidden): `live` → `SessionMark working` + text **`Thinking`**; else **`Thought for <seconds>s`**.
Body: `div.__body` Prose; spacious: margin `space-2 0 0 6px`, padding-left `space-4`, left border 1px `border-subtle`, `fg-muted`, prose at 14/22. A live reasoning row also counts as "ends in calls" for the working-line rule (A1.9).

### A7.22 End card "Nothing left to do" (trailing; spacious only)
`sessions.tsx:3223-3242` renders it as the `trailing` node when `done` is set, the Job is not waiting, and the take is spacious: `section.armada-session-end aria-label="Nothing left to do"`: flex column, gap `space-3`, padding `space-4`, no border; bg `fg-default 4%` + inset 1px ring `fg-default 7%`, radius 8, `margin-inline −20px`.
`h3.armada-session-end__head` flex, gap `space-2`, `spacious-body` 15/24 weight 500: `Check` 14/2 (`step-advanced`) + **`Nothing left to do`**.
`DoneBody`: `p.armada-session-done__summary` (if summary; padding 0, 15/24, `fg-muted`) · `ul.armada-session-done__made` (aria-label `Pull requests` if the first entry is a pull request else `Artifacts`; flex column gap `space-2`, scrolls) of `li.armada-session-ledger__row` (flex, border 1px `border-subtle`, radius `radius-md`, bg `bg-raised`, 13/18; hover border `border-strong` bg `bg-hover`; arrive animation tinting `status-running 14%` decaying over 45 s) each holding `button.armada-session-ledger__open` `aria-label="Open <name>"` (flex, align center, gap `space-2`, padding `space-2 space-3`): a pull request → `PullRequestStateMark merged`, else the artifact glyph by `artifactOf` (tooltip = its word) else `GitPullRequest` 12/2; text (flex 1, ellipsis nowrap); for a PR also `PullRequestChecksMark` · `div.armada-session-done__acts` (flex gap `space-2`; it is one of the waiting cards of the thread keys [B]): `Button secondary sm` **`End job`** + `ThreadKbd` hint, `Button ghost sm` **`Keep going`** + hint (⌥↵ / ⌥X shown per thread-keys [B]).
Base (non-spacious): the same `DoneBody` is the body of the composer's end band instead (`endShown`).

### A7.23 Other pieces that exist beside the rows
| Piece | Where | Note |
|---|---|---|
| `Did` / `actOf` | A2.4 | |
| `WorkingTodo` | exported type (`SessionThread.tsx`), used by `session-activity.ts:13 todosOf` for the Cockpit/Job-list tile | not drawn in the thread (TaskCreate/TaskUpdate calls are `UNDRAWN`) |
| `ConcernClaim` | `SessionThread/ConcernClaim.tsx` | only used by `AdversarialReview` (plan judge card, part B); not a thread row |
| Ledger file entries (attachments, made files) | `SessionLedger` (panel beside the thread, `sessions.tsx:3030`) | not thread content |
| `browser_reminder` rows | filtered out by `session-thread-rows.ts:238`; drawn by the Job browser | |
| Wire `plan` and `delivery` rows | `rowsOfThread` has no case | never drawn |
| `FromSession` for a non-Fleet sender | A2.5 | |
| `SessionNotice` ids | `NOTICE_NAMES` (A2.5) | |

## A8 Fork from selection (`ForkFromSelection.tsx`, `ForkFromSelection.css`)

### A8.1 What it reads
Mounted by the thread when any of `onForkAt`, `onReplyAt`, `onTeachAt` is given (`SessionThread.tsx:3912`), with `scope` = the thread's whole box.
It reads the document selection and never owns it.
A selection counts only if it lies wholly inside ONE `[data-fork-row]` block under `scope` (the text block of a person bubble, an agent message, or a Fleet-band body; folded/Fleet-turn bodies, calls, cards and the composer are not forkable).
A triple-press selection that ends at the start of the next block counts when the block's own text from the start equals the selection.
Empty/whitespace-only, collapsed, or scrolled fully out of the scroller's box → no bar.

### A8.2 The bar
`div.armada-fork-from-selection role="toolbar" aria-label="Selection"` portalled to `document.body`: `position: fixed`, `z-index: var(--z-menu)` (40), flex, align center, gap `space-1`, padding `space-1`, border 1px `border-subtle`, radius `radius-md`, bg `bg-overlay`, `box-shadow: --shadow-overlay`.
Each act: `Button type=button variant=ghost size=sm iconOnly` (24×24) with a 16px/2 glyph, `aria-label` = label, `Tooltip beside` = label.
Bar order and labels: **Reply** (`Reply` glyph), **Fork from here** (`GitFork`), **Teach** (`GraduationCap`); each shown only when its handler exists.
`onMouseDown` preventDefault (a press never takes the selection).
Position: horizontally centred over the first line of the selection (`left = first.left + min(first.width, 96)/2 − width/2`, clamped to `8px` from the window edges), `top = first.top − height − 8`; if there is less than 8px room above, `top = last line bottom + 8`.
Hidden (`visibility:hidden`) until measured.
When it shows/goes: appears on mouse-up (left button) or selection change when no drag is in progress (keyboard selections too); goes when the selection collapses, on Escape (clears the selection; ignored while the menu is open), when the selected words scroll out of the scope, or on resize recompute.
Taking an act clears the selection.
Coexistence: `useCopyOnSelect` (A1.12) also copies the same selection to the clipboard at mouse-up.

### A8.3 The menu form
A right press ON the selected words (inside the selection's bounding box and in the same block), or ContextMenu / Shift+F10 while words are selected (both captured and stopped at `scope`, so a row's own menu, e.g. Fleet's notice menu, is used for every other press) opens `DropdownMenu triggerLabel="Selection"` at the pointer (or under the selection's first line centre for the key) with items in this order: **Teach** (GraduationCap), **Reply** (Reply), **Fork from here** (GitFork) — only the offered ones.
Selecting an item takes the act.

### A8.4 The acts (host: `sessions.tsx:3102-3140`)
| Act | Payload `ForkPick{rowId, quote}` | Effect |
|---|---|---|
| Reply | `quote` = `quoteOf(words)` (every line prefixed `> `, blank line → `>`, trailing blank line) | `quoteIntoDraft(session.id, quote)` puts the quote in this Job's composer draft and focuses it; not offered when the Job is dead |
| Fork from here | same | `forkAt(id, rowId)` creates a new Job forked at that message (carries the conversation up to and including it); then the new Job's draft is seeded with the quote and focused, and the new Job opens (`seedDraft`, `setFocusFor`, `opening(…)`) |
| Teach | same | `teachOpen("words", wordsOfQuote(quote))` — quote with the `> ` marks removed; opens the Teach panel |
Row ids: the id of the message row (person bubble, agent message, Fleet-band body).
An optimistic person message has a client-made `id` until Fleet's row replaces it (it keeps `stableKey`); what `forkAt` does with such an id is not in the files read.
`MessageBar` Reply (A2.3a) uses the same `onReplyAt` with the WHOLE message as the quote.

## Open questions

QUESTION: Threads drawn outside the Job view (the origin thread inside `ForkedFrom`, call cards, subagent pane, mock sheets) get the BASE look, not spacious; should the native build draw the base look for the opened `Forked from <title>` thread, or the spacious one?
QUESTION: `MessageBar` fades with `transition: opacity 120ms` and has no `prefers-reduced-motion` override in `spacious-thread.css` or `SessionThread.css`; is there a global reduced-motion rule that zeroes it, or does it fade under reduced motion today?
QUESTION: `spacious-thread.css` still carries `p[data-pick]` hover/focus fill rules and `.armada-prose__pick-acts` rules, but `ProsePick` is `null` in the spacious thread (`SessionThread.tsx` agent row), so nothing in the Job view sets `data-pick`; is that CSS dead for the Job view, and should the native build skip a per-paragraph pick bar?
QUESTION: Row kinds `woke`, `act`, `operator`, `reasoning` (and `moved`/`judged`/`divider`/`change`/`suggest`/`not_started` via TS) exist in `SessionThreadRow`, but `rowsOfThread` (`packages/screens/src/sessions-wire.ts:1094`) has no case for `woke`, `act`, `operator`, `reasoning`; does any other producer (Rust `SessionRow`, plugin, mod) emit them, or are they mock-only and safe to omit natively?
QUESTION: Rust `thread_view.rs` folds Fleet turns, calls, Checks and subagents without Electron's `keep` set (disputed notices, and the last Fleet turn while the Job is asking), without the `UNDRAWN` filter, and leaves a settled `ask` as `Row{Ask}`; should the native thread use Fleet's `ThreadItem`s and re-add those three TS rules on top, or fold the rows itself exactly as `itemsOf` does?
QUESTION: In `itemsOf`, a `tool` row that carries a `subagent` and follows a Fleet notice is swallowed into the Fleet turn's `reply` (drawn as a plain tool line inside the opened turn), so it never becomes a `SubagentRow`; is that intended, and does Rust `thread_view.rs` do the same?
QUESTION: `forkAt(id, rowId)` is given the message row id; for an optimistic person message (client-made `id`, `stableKey` kept) before Fleet's row arrives, does the fork still work, or is the Fork act hidden/failing?
QUESTION: Syntax colouring of fenced blocks (`colourCode`, `.tok-*` classes at `spacious-thread.css` ~340-600) was not recorded token by token; does the native build need the exact per-language palette (keyword/string/comment/number colours) and which languages are recognised?
QUESTION: The inline Run of a shell fence (`RunnableBlock`, `RunOutput`, `InlineTerminal`, `HeldTerminal`, `ProseTerminal`) and the slash-command chip (`SlashCommand`) are summarised only by their acts here (A2.9); are they part of "message markdown" for the native spec, or covered by another part?
QUESTION: `ReferenceLink` (rendered `#123`, Job ids, paths, documents, taught links) and `Picture`/`ProseAttachments` (inline images, video posters) were recorded by behaviour, not by style; do they need pixel values recorded?
QUESTION: `LessonRoute` (the who/lands-in route line in a retro item), `PullRequestStateMark`, `PullRequestChecksMark` and `artifactOf` (glyph and word per artifact kind in the end card) were named but not expanded; do they need their own entries?
QUESTION: `Stamp` ages and clock use `Intl` with the user's locale and the owner's time-format setting (`hour12`, `seconds`, `date`); in native, is the same locale-aware format required, and where does the setting live (`packages/components/src/time-format.ts`)?
QUESTION: The scroller restores position on Back via `session-view/place.ts` (two animation frames after rows draw, skipped within 40 px of the end); is that restore part of the Thread spec or of the Job view's navigation spec?

### Part B: Job thread, part B: the cards (as shipped in Electron today)

Record of what IS. Paths are repo-relative. `ST` = `packages/components/src/compositions/SessionThread`. `DESK` = `apps/desktop/src/renderer/src`.

## B0 Ground rules that apply to every card below

### B0.1 Where the cards are mounted

| Fact | Where |
|---|---|
| The Job's thread is `<SessionThread>` inside `<ThreadKeys key={session.id}>` inside `.armada-session-frame__centre` | `DESK/sessions.tsx:3050-3058` |
| The frame is a `Card` (`armada-card armada-session-frame`, `role="region"`, `aria-label="Job <name>"`) | `packages/components/src/compositions/SessionFrame/SessionFrame.tsx:738` |
| `.armada-card.armada-session-frame` sets `--pad-card: calc(var(--space-6)/2)` = 12px (default spacing), `gap: calc(var(--space-3)/2)`; `--pad-card` inherits, so every `Card` in the thread (plan, draft, ask, sketch pin) pads 12px, not the Card default 20px | `DESK/session-view/session-view.css:1304-1325` |
| Inside the frame `.armada-session-thread`, `.armada-session-thread__stage`, `.armada-session-frame__centre` gap is `calc(var(--space-3)/2)` = 6px (base rule is `--space-4`) | `DESK/session-view/session-view.css:1311-1315`, base `ST/SessionThread.css:3-10` |
| Thread rows are one `<ol class="armada-thread-rows">`: `display:flex; column; gap: var(--thread-gap, var(--space-4)); list-style none` | `ST/ThreadRows.css:8-16` |
| Thread scroller `.armada-session-thread__rows`: `flex:1; min-height:0; overflow:auto; isolation:isolate` | `ST/SessionThread.css:20-26` |
| CSS order in `packages/components/src/index.css`: Card.css (line 20) loads before SessionThread.css (329), SketchRow.css (338), DraftRow.css (413); a one-class rule in the later file wins over `.armada-card` | `packages/components/src/index.css` |

### B0.2 Which cards Electron really draws in the thread (important)

| Card | Drawn in the thread's `<ol>`? | Notes |
|---|---|---|
| Sketch card (`SketchRow`) | yes, as a row | the newest asked sketch is drawn as the pinned `SketchPin` instead |
| File row | yes, as a row | `Opened <name>` quiet row |
| Document card (`DocumentRow`) | yes, as a row | |
| Draft card (`DraftRow`) | yes, as a row | |
| Plan card (`PlanCard`) | pending/set aside: the thread's tail `li`; approved: a record `li` after row `approval.after` | |
| Permission card / question deck / lapsed deck (`AskCards`) | **NO, not in the thread.** `sessions.tsx` passes neither `asked`, `lapsed` nor `askRef` to `<SessionThread>`. They are drawn inside the Needs you panel on the composer (`question.children`, `<AskCards flat ...>`) | `DESK/sessions.tsx:2245-2350` comment: "is the first thing in the Needs you panel on the composer, not the thread's last card" |
| `AskCard` (composition) / `CallCard` | not in the Job thread. `AskCard` is used only by the Cockpit `DESK/cockpit/CallCard.tsx:475` | |
| Open call (`open_call`) | not a card; it is the agent row's text while a turn runs (`StoryColumn.tsx:207`) | see B5 |

`SessionThread` props actually passed (all at `DESK/sessions.tsx:3071-3270`): `sessionId`, `rows={withJudged}`, `lead`, `transcript`, `inline={inlineCards}` (mod cards + browser reminders, B5.3), `onOpenRetro`, `onRetry`, `onStartAgain`, `onReport`, `onForkAt`, `onTeachAt`/`onTeachRow`, `onMuteNotice`, `onReplyAt`, `approval`, `sketching`, `waitingOnYou: true` iff `session.asked !== undefined`, `onOpenWindow`, `onOpenSketch` (opens `read({kind:"sketch", id})`), `onOpenFile`, `onReadMedia`, `onDraft`, `onWithdrawDraft`, `onAnswerSketch`, `onWithdrawSketch`, `onOpenDocument`, `onAnswerDocument`, `noted`, `onOpenCheck`, `onOpenSubagent`, `working`, `onStop`, `trailing`, `runner`, `onCommand`/`commandNames`, `onAnswer={() => undefined}`, `onOpenSession`, `lookHere` props, `onPickLine`.

### B0.3 Spacious take (which rules apply, how chosen)

| Fact | Where |
|---|---|
| `document.body.dataset.threadTake = "spacious"` is set unconditionally at start-up: **the shipped app always draws every Job thread in the spacious take** | `DESK/main.tsx:13-14` |
| `import "./spacious-thread.css"` after the app's own CSS | `DESK/main.tsx:6` |
| `document.documentElement.dataset.look = "new"` | `packages/components/src/look.ts:4`, imported `DESK/main.tsx:7` |
| `SpaciousTake` React context value = `useContext(SpaciousTake) \|\| body.dataset.threadTake === "spacious"` = true in the app; provided by `<SpaciousTake.Provider value={spacious}>` | `DESK/sessions.tsx:1722, 2874`; context `ST/spacious-take.ts:7` |
| Every spacious selector is `:root[data-look] [data-thread-take="spacious"] .armada-session-frame ...` (so only inside a Job's frame) | `DESK/spacious-thread.css` |
| JS difference with `spacious` true: rows the take leaves out (`UNDRAWN`: `TaskCreate TaskUpdate TaskList TaskGet ToolSearch show_window` tool rows) are filtered before folding | `ST/SessionThread.tsx:1215-1226, 3728-3731` |

Spacious variables on `.armada-session-frame`: `--spacious-column: clamp(680px, 70cqi, 960px)`, `--spacious-bleed: calc(space-4 + space-1)` = 20px, `--spacious-outer: column + 2*bleed`, `--spacious-gutter: space-4`, `--spacious-body: 15px`, `--spacious-leading: 24px`, `--spacious-meta: 13px`, `--spacious-radius: var(--radius-lg)` (8px), `--spacious-card: color-mix(in srgb, var(--fg-default) 4%, transparent)`, `--spacious-card-edge: inset 0 0 0 1px color-mix(in srgb, var(--fg-default) 7%, transparent)` (`spacious-thread.css:5-22, 703-708`).
Aa-panel overrides: `data-text` xs/sm/lg change `--spacious-body` to `--text-xs`/`--text-sm`/`--text-lg` and leading 20/22/28; `data-spacing` compact/roomy set `--spacious-turns` (`:145-181`).
The rows column: `.armada-session-thread__rows` gets `overflow-x:hidden; padding: var(--space-4) calc(max(var(--spacious-gutter), (100% - outer)/2) + bleed) 0` (`:36-41`).
`.armada-thread-rows` gap = `var(--spacious-turns, calc(var(--space-6) + var(--space-1)))` = 28px, `padding-bottom: var(--space-6)` (`:139-142`).

Spacious rules that touch cards of part B (all in `DESK/spacious-thread.css`):

| Selector | Effect | Lines |
|---|---|---|
| `.armada-thread-rows :is(.armada-session-sketch, .armada-session-end, .armada-session-thread__ask, .armada-session-thread__subagents, .armada-session-subagent)` | `margin-inline: calc(var(--spacious-bleed) * -1)` = -20px: the card reaches 20px past the words column. Hits the sketch and document cards and the plan card (`__ask` class), pending and as a record, and the pinned sketch. Does NOT hit the draft card | 46-59 |
| `.armada-session-sketch__asked` | `border-color: var(--border-subtle); border-radius: var(--spacious-radius); background: transparent`; then overridden at 711-716 to `border: none; background: var(--spacious-card); box-shadow: var(--spacious-card-edge)` | 383-387, 711-716 |
| `.armada-session-sketch__band` (also the plain sketch card's band) | `min-height:0; padding: var(--space-3) var(--space-4) var(--space-1); border-bottom:none; background:transparent; font-family: var(--font-sans); font-size: var(--spacious-meta); text-transform:none; letter-spacing:0` | 389-398 |
| `.armada-session-sketch__title` | `font-size: var(--spacious-body); line-height: var(--spacious-leading); font-weight: var(--weight-medium)` | 400-404 |
| `.armada-session-sketch__at, __state` | `font-family: var(--font-sans); font-weight: var(--weight-body); font-variant-numeric: tabular-nums; color: var(--fg-subtle)` | 406-411 |
| `.armada-session-sketch__answers` | `padding: var(--space-2) var(--space-4) var(--space-4); border-top:none; background:transparent` | 413-421 |
| `.armada-session-sketch__answers .armada-button` and `.armada-session-done__acts .armada-button` | `height: var(--h-control-lg)` (32); `padding: 0 var(--space-4)`; `border-radius: var(--spacious-radius)`; `font-size: var(--text-sm)`; `font-weight: var(--weight-medium)`; secondary: `border-color: transparent; background: color-mix(in srgb, var(--fg-default) 8%, transparent)` | 423-452 |
| `.armada-session-sketch__answers .armada-card-footer` | `gap: var(--space-2)` | 454-461 |
| `.armada-session-fold__head--press` (window, file, judged rows) | a card: `gap: space-2; min-width: min(100%, 280px); padding: space-3 space-4; border-radius: var(--spacious-radius); background: var(--spacious-card); box-shadow: var(--spacious-card-edge); font-size: var(--text-sm); line-height: 20px; color: var(--fg-default); transition: background var(--duration-fast) var(--ease)`; hover `background: color-mix(fg-default 7%, transparent)`; its `> svg` is 16px in `--fg-muted` | 703-750 |
| `.armada-question-deck__*` | see B4.4 | 1557-1640 |
| `.armada-needs-panel__band/__body/__label` | see B7 | 526-538 |
| tooltip bubble over the thread | `padding: var(--space-1) calc(var(--space-2) + var(--space-1)); border-color: transparent; border-radius: var(--spacious-radius); box-shadow: var(--spacious-card-edge), var(--shadow-overlay)` | 1631-1640 |
| Plan card, draft card, permission card, SketchPin card, thread-target ring | **no spacious rule**: they draw as the base CSS in B2/B3/B4 | grep: none |

### B0.4 Token table (value is Dark, `data-look="new"`, default spacing)

| Token | Value | Token | Value |
|---|---|---|---|
| `--space-1/2/3/4/6/8/12` | 4 / 8 / 12 / 16 / 24 / 32 / 48 px (compact: 3/6/9/12/18/24/36; roomy: 5/10/15/20/30/40/60) | `--border-width` | 1px |
| `--radius-sm/md/lg` | 3 / 6 / 8 px | `--h-row` | 28px |
| `--h-row-unit` | 36px | `--h-control / -sm / -lg` | 28 / 24 / 32 px |
| `--h-badge` | 20px | `--h-kbd` | 20px |
| `--pad-card` | 20px (12px in the Job frame) | `--text-2xs` | 12px / leading 16px |
| `--text-xs` | 13px / 18px | `--text-sm` | 14px / 20px (body) |
| `--text-base` | 15px / 22px | `--text-lg` | 18px / 28px |
| `--weight-body/medium/heading` | 400 / 500 / 600 | `--tracking-caps` | 0.04em |
| `--font-sans` | "IBM Plex Sans", ui-sans-serif, system-ui | `--font-mono` | "IBM Plex Mono", ui-monospace, "SF Mono", Menlo |
| `--bg-base/sunken/raised/overlay/hover` | #0F1419 / #0B0F13 / #161C23 / #1D242D / #212A34 | `--border-subtle/default/strong` | #232B35 / #2E3946 / #3D4A5A |
| `--fg-default` | #E4E9EF | `--fg-muted` / `--fg-subtle` | **#B3BFCE / #94A3B6** (takes.css overrides colors.css #93A1B1/#7E8CA0 under `data-look="new"` dark) |
| `--fg-inverse` | #0F1419 | `--accent / -hover / -muted` | #4A9EDB / #5FB0E8 / #1C3A52 |
| `--accent-faint` | accent 10% in transparent | `--you` / `--you-edge` | #E8C547 / you 55% in transparent |
| `--status-running / awaiting-review / escalated` | #4FB8D9 / #E5A93D / #EE8450 | `--status-completed-success / -failed` | #4FAF7C / #E97878 |
| `--step-advanced / running / waiting` | success / running / awaiting-review | `--step-failed` | completed-failed |
| `--status-not-started` | #8C97A6 | `--status-handed-in` | #4FB4AA |
| `--duration-press/fast/base/sheet/travel/pulse` | 90 / 120 / 180 / 220 / 300 / 1600 ms (all 0 under reduced motion) | `--ease` | cubic-bezier(0.2,0,0,1) |
| `--focus-ring` | `1px solid var(--accent)` | `--focus-ring-offset` | 2px |
| `--shadow-overlay` | `0 8px 24px -6px rgb(0 0 0/.55), 0 2px 6px -2px rgb(0 0 0/.4)` | `--shadow-primary` | `inset 0 1px 0 rgb(255 255 255/.25), 0 2px 10px -3px color-mix(accent 45%, transparent)` |
| `--z-menu` | 40 | | |

Sources: `packages/tokens/src/{spacing,typography,colors,status,motion,fonts,semantic}.css`, `packages/tokens/takes.css:106-110`, `spacing-sets.css`.
Derived values used below: `--h-row*1.25` = 35px; `--h-row-unit*8` = 288px; `--h-row-unit*3` = 108px; `--space-12*5` = 240px; `--space-12*6` = 288px; `--space-12*2 + --space-4` = 112px; `--space-8*2 + --space-2` = 72px.

### B0.5 Shared primitives these cards use

| Primitive | Facts (file) |
|---|---|
| `Card` (`flat`) | `div.armada-card[data-flat]`: flex column, `gap: var(--space-3)`, `padding: var(--pad-card)`, `border: 1px solid var(--border-default)`, radius `--radius-md` (6), bg `--bg-raised`, color `--fg-default`; `flat` removes glass (`packages/components/src/primitives/Card/Card.css:20-30`, `Card.tsx`). `CardFooter`: flex, `align-items:center`, `gap: var(--space-2)` |
| `Button` | height `--h-control` (28), `sm`: `--h-control-sm` (24), padding `0 var(--space-4)` (sm: `0 var(--space-2)`), `gap: var(--space-2)`, radius `--radius-md`, 14px/20px weight 500 (sm: 13px/18px), border 1px transparent. `primary`: bg/border `--accent`, color `--fg-inverse`, `--shadow-primary`; hover `--accent-hover`; active bg `--accent-muted`. `secondary`: bg `--bg-sunken`, border `--border-strong`; hover `--bg-hover`. `ghost`: transparent, color `--fg-muted`; hover `--bg-hover` + `--fg-default`. `iconOnly` width = height (`packages/components/src/primitives/Button/Button.css`) |
| `Kbd` | `inline-flex`, `height: var(--h-kbd)`, `padding: 0 var(--space-1)`, `border: 1px solid --border-default; border-bottom-width: 2px`, radius `--radius-sm`, bg `--bg-raised`, color `--fg-muted`, mono `--text-2xs` (`packages/components/src/primitives/Kbd/Kbd.css`) |
| `Stamp` | time text; `still` = plain `<span>`; otherwise a `<button>` with tooltip showing the other reading; shows clock first by default; text that is not a date is drawn as given; one press on any stamp inside `<StampSurface>` flips all (`packages/components/src/primitives/Stamp/Stamp.tsx`) |
| `Tooltip` | wraps every bare glyph; the label text is what is listed in each table |
| `Textarea`/`Input` | `keep` prop persists typed text under the key until cleared |

---

## B1 Sketch, file, window, judged and document cards

### B1.1 Where it lives

| What | Component | CSS |
|---|---|---|
| Sketch card (in a row) | `SketchRow` `ST/SketchRow.tsx:242-381` | `ST/SketchRow.css:1-163` |
| Approve / Send back / Withdraw footer shared by sketch, document, pin | `ApprovalAnswers` `ST/SketchRow.tsx:43-135` | `SketchRow.css:124-163` |
| Sketch pinned as the thread's last card | `SketchPin` `ST/SketchRow.tsx:515-627` (mounted `ST/SessionThread.tsx:3869-3878`) | `SketchRow.css:220-276` |
| The sketch read large (panel and pin) | `SketchReading` `ST/SketchRow.tsx:386-470` | `SketchRow.css:165-218` |
| Revision picking | `sketchAtRevision` `SketchRow.tsx:137-184`, `useSketchRevision` `:187-205`, `RevisionMenu` `packages/components/src/compositions/RevisionMenu/RevisionMenu.tsx` + `.css` | `RevisionMenu.css:1-10` |
| Document card | `DocumentRow` `ST/DocumentRow.tsx:1-99` | `SketchRow.css:278-298` (+ shares `__asked`, `__band`, `__answers`) |
| File row | inline in `Row`, `ST/SessionThread.tsx:2106-2123` | `ST/SessionThread.css:507-546` (`armada-session-fold*`) |
| Picture/video under a file row | `MediaFrame` `packages/components/src/compositions/MediaRow/MediaRow.tsx` | not read; QUESTION below |
| Window row | inline `ST/SessionThread.tsx:2089-2103` | same fold CSS |
| Judged row | inline `ST/SessionThread.tsx:2183-2196` | same fold CSS |
| Sketch panel body the card opens | `DESK/session-view/SketchBody.tsx:1-134` (uses `SketchReading`, `RevisionMenu`) | out of scope except how it opens |
| Preview drawing | `SketchScene` (`preview`, `playing`) `packages/components/src/compositions/SketchScene/` | not read; QUESTION |

### B1.2 Sketch card (plain, not asked)

`<li.armada-session-sketch [data-replaced] [data-revisions]>` containing `<button.armada-session-sketch__press>` (and a sibling `<span.armada-session-sketch__revision>` holding the revision menu when the sketch has revisions).

| Region (top to bottom) | Layout / size / tokens (SketchRow.css) |
|---|---|
| `li.armada-session-sketch` | `display:flex; min-width:0` (:4-7). `[data-revisions]`: `position:relative` (:68-70) |
| `button.__press` | flex column, `flex:1; min-width:0; padding:0; border: 1px solid var(--border-default); border-radius: var(--radius-lg)` (8); bg `--bg-sunken`; color `--fg-default`; `font:inherit; text-align:start; cursor:pointer; overflow:hidden; transition: border-color 120ms ease` (:9-24). Hover: border `--accent` (:26-28). Focus-visible: outline `--focus-ring`, offset 2px (:30-33). `[data-replaced]`: `border-style: dashed` (:92-94) |
| `span.__band` | flex, `align-items:center; gap: var(--space-3)`; `min-height: calc(var(--h-row) * 1.25)` (35px); padding `4px 12px 4px 16px`; `border-bottom: 1px solid --border-default`; bg `--bg-raised`; color `--fg-muted`; mono `--text-xs` (13/18); weight 600; `letter-spacing: var(--tracking-caps)`; `text-transform: uppercase` (:35-50). `[data-revisions]` adds `padding-inline-end: calc(var(--space-12)*2 + var(--space-4))` = 112px (:72-74) |
| band children, in order | glyph `PencilRuler` 12px stroke 2; `span.__title` (`flex:1; min-width:0; color:--fg-default; ellipsis, nowrap` :52-59); `span.__state` "Replaced" (if `row.replaced`); `span.__state` `Against <row.against>` (if `against` is set and the shown revision is not an earlier one); `Stamp still at=row.at` class `__at`. `__state`/`__at`: `flex:none` (:61-64) |
| `span.__glass` | `display:block; box-sizing:border-box; block-size: calc(var(--h-row-unit) * 8)` = 288px; `padding: var(--space-2)` (:85-90). `[data-replaced]` glass and title: `opacity:0.5` (:96-99). Holds `<SketchScene scene=... against=... preview playing={pointed\|\|focused} key={revision}>` |
| `span.__revision` (only with revisions) | `position:absolute; inset-block-start: 1px; inset-inline-end: var(--space-3); display:flex; align-items:center; block-size: calc(var(--h-row)*1.25)` (:76-83); contains `RevisionMenu align="end"` |

Revision menu (`RevisionMenu.tsx`): `span.armada-revision-menu` (`inline-flex; color --fg-muted`); the trigger is a plain `DropdownMenu` button labelled `Revision <n>` with tooltip `Revision`, text `--text-xs`/`--leading-xs`/weight 400 (`RevisionMenu.css`).
Entries `Revision N` for N = count down to 1 (newest first), the shown one `selected`.
Picking the newest sets `picked` undefined.
The menu exists wherever `revisions.length > 0` (even one).

Behaviour: press = `onOpen(row.sketch)` (aria-label `Open sketch <title>`); hover or focus sets `playing` on the scene (steps loop only while pointed/focused, else a still drawing).
`sketchAtRevision`: with revisions, each revision is read against the one before it (title `Revision <n-1>`), the first plain; the newest keeps the card's own `against` when the agent named one (`named`); an earlier revision is read only (`earlier`).

### B1.3 Sketch card put to the person (asked)

Chosen when `(row.asked === true && row.withdrawn !== true) || settled !== undefined` (`SketchRow.tsx:329`). Otherwise B1.2.

`li.armada-session-sketch[data-asked]` > `div.__asked[role="article"][aria-label="Sketch: <title>"][data-settled=<how>]` (flex column, `flex:1; min-width:0; border:1px solid --border-default; radius --radius-lg; bg --bg-sunken; overflow:hidden; transition border-color` `SketchRow.css:103-113`; border `--accent` when the press is hovered :115-117).
Inside it the press loses border/radius (:119-122).
Under the press, when `open || settled`: `div.__answers` = `ApprovalAnswers`.

`div.__answers`: flex column, `gap: var(--space-2)`, `padding: var(--space-2) var(--space-3)`, `border-top: 1px solid --border-default`, bg `--bg-raised` (:124-131).

`open` (`SketchRow.tsx:253-258`) = `row.asked && !row.withdrawn && settled===undefined && !row.replaced && onAnswer!==undefined`.

`ApprovalAnswers` (`SketchRow.tsx:43-135`), top to bottom:
1. Note `Textarea` `aria-label="Note"` `rows=3` `autoFocus` `keep="<keep>:note"` (only when `open && noting`). `noting` initial = a kept note exists.
2. Quoted note `blockquote.__note` when `settled.how==="sent_back" && settled.words`: `margin 0; padding 8px 12px; border-left: 2px solid var(--you-edge); pre-wrap; anywhere; 14/20; color --fg-default` (:134-143).
3. Settled line `p.__settled[data-how]` (`flex; gap space-2; text-xs 13/18; color --fg-muted` :145-153) with a `Tooltip` + `span.__mark[role=img][aria-label=<said>]` holding a 16px glyph (`BadgeCheck` for Approved, `Reply` for Sent back; mark colour `--step-advanced`, `--fg-muted` for sent_back :155-163) and `<span aria-hidden>` with the same word.
   Strings: `Approved`, `Sent back`.
4. `CardFooter[role=group][aria-label="Answers"]` when `open && onAnswer`: not noting: `Approve` (primary sm; sends `{choice:0}`), `Send back` (secondary sm; sets noting), `Withdraw` (ghost sm; only when `onWithdraw` given; calls `onWithdraw(id)`).
   Noting: `Send back` (primary sm, disabled while note blank; sends `{text: note.trim()}` then cancels), `Cancel` (ghost sm; forgets the kept note).
5. A settle (`settled` changes) cancels the note box (effect :60-65).

Strings: `Open sketch <title>`, `Sketch: <title>`, `Replaced`, `Against <title>`, `Note`, `Answers`, `Approve`, `Send back`, `Withdraw`, `Cancel`, `Approved`, `Sent back`, `Revision <n>`, `Revision`.

### B1.4 Pinned sketch (`SketchPin`)

When Electron has a sketch waiting, `sessions.tsx:2420-2445` builds `sketching` from the **newest asked sketch only** (`newestAskedSketch(attachmentsOf(session,"sketch"))`) when `draft.answerSketch` exists and the sketch is not withdrawn or settled.
The thread then (a) drops that sketch's own non-replaced card from the rows (`ST/SessionThread.tsx:3724-3731`), (b) does not pass `onAnswerSketch/onWithdrawSketch` to rows (`:4111-4120`), (c) draws `SketchPin` in the tail `li` after the plan card.

`Card flat.armada-session-thread__ask.armada-session-sketch-pin[role=article][aria-label="Sketch to approve: <title>"][data-folded]` (`SketchRow.tsx:540-548`).
Base `.armada-session-thread__ask`: flex column, `gap: var(--space-2)`, `flex-shrink:0`, `max-height:60vh`, `overflow:auto` (`ST/SessionThread.css:164-172`).

| Region | Content / size |
|---|---|
| `div.armada-session-thread__plan-head` | flex, `justify-content:space-between`, `gap: var(--space-3)` (`SessionThread.css:205-210`); holds `span.armada-session-sketch-pin__band` (flex, `inline-size:100%`, gap `--space-2`, color `--fg-muted` :221-229): `PencilRuler` 12px; `span.armada-session-thread__eyebrow` = title (2xs/16, weight 600, caps tracking, uppercase :441-446); bell `span.armada-session-thread__settled[role=img][aria-label="Sketch waiting for approval"][data-state=waiting]` with `BellRing` 16px, colour `--step-waiting` (tooltip `Sketch waiting for approval`); `span.__tools` (flex, gap space-1, `margin-inline-start:auto` :242-247) = revision menu (if any), `Button ghost sm iconOnly` `Maximize2` 12px labelled `Open in the panel` (only if `onOpen`), `Button ghost sm iconOnly` aria-label/tooltip `Fold` or `Unfold` with `ChevronDown` (open) / `ChevronRight` (folded), `aria-expanded={!folded}` |
| `div.__read` (unless folded) | flex column, `block-size: calc(var(--space-12)*6)` = 288px, `min-height:0; border: 1px solid --border-subtle; radius --radius-md; overflow:hidden` (:231-239); holds `SketchReading` (scene `autoplay`, title, `against` toggle band, `notes` = ViewerNotes scope when the shown revision is not earlier) |
| `div.armada-session-sketch__answers` | `ApprovalAnswers` with `open` always true |
| Folded | `[data-folded]` becomes a grid `minmax(0,1fr) auto`, `align-items:center; column-gap: space-3`; head spans row 1; the answers' `[role=group]` footer sits on row 1 col 2 with `margin-inline-end: calc(var(--space-8)*2 + var(--space-2))` = 72px (left of the fold button); a note box goes on row 2 (:249-276) |

`SketchReading` (`:386-470`) when `against` or a menu exists: `div.__read__band` flex gap space-4, min-height 35px, padding `space-1 space-3 space-1 space-4`, bottom rule, bg raised, mono 13/18 weight 600 caps; left `span.__against` "Against <title>" (flex 1, ellipsis); `span[role=group][aria-label="Read as"]` with `Button sm` `Changes` / `Whole` (`aria-pressed`; active = `secondary`, other `ghost`; default Changes); then the menu.
The scene fills the rest (`.armada-sketch` flex 1, no border).

### B1.5 Document card

`li.armada-session-sketch.armada-session-document[data-asked]` > `div.__asked[role=article][aria-label="Document: <title>, Revision <n>"][data-settled]` (`DocumentRow.tsx:41-48`).
Always the "asked" frame (B1.3 CSS), no drawing.

| Region | Detail |
|---|---|
| `div.__band` (`[data-press]` when `onOpen`; whole band is a click target, `cursor:pointer`, hover bg `--bg-hover` `SketchRow.css:283-289`) | same band as B1.2 but `border-bottom:0` (:279-281). Children: `FileText` 12px; `span.__title`; `Tooltip(Revision <n>)` over `span.__state` text `r<n>` (`text-transform:none` :296-298); `Stamp still` `__at`; `Button ghost sm iconOnly.armada-session-document__open` `Maximize2` 12px, tooltip `Open`, aria-label `Open <title>, Revision <n>`, `stopPropagation`, `flex:none` |
| `div.__answers` | when `open \|\| settled`; `ApprovalAnswers` with `keep = keep ?? document:<row.id>`; **no `onWithdraw`**, so no Withdraw |

`open` = `row.asked && settled===undefined && onAnswer!==undefined`.
Every revision is put to the person; a new revision asks again.
Press on band or Open: `onOpen(row.page)` -> `openOwedDocument(session, page)` (opens the page layer; not covered).
Approve/Send back -> `documentAnswerOf` -> `draft.answerDocument` (`DESK/sessions.tsx:3173-3181`, `packages/screens/src/thread-cards.ts:49-71`).

### B1.6 File, window and judged rows (quiet rows)

All three are `li.armada-session-fold` > `button.armada-session-fold__head.armada-session-fold__head--press` (type button).

Base CSS (`SessionThread.css:507-546`): `.armada-session-fold {min-width:0}`; `__head`: flex, `align-items:center; gap: var(--space-1); min-width:0; width:fit-content; max-width:100%; cursor:pointer; list-style:none; font-size: var(--text-2xs); line-height: var(--leading-2xs); color: var(--fg-muted)`; `--press`: `padding:0; border:0; background:none; font:inherit`; hover `color: --fg-default`; focus-visible ring.
In spacious (shipped) these become cards, B0.3.

| Row | Content | aria-label | Press |
|---|---|---|---|
| File (`kind:"file"`) | `File` 12px + `Opened <span.mono>{name}</span>`; below it a `MediaFrame path name onRead` when `onReadMedia` is given and `mediaKindOf(path)` is not undefined | `Opened <name>` | `onOpenFile(path)`: video -> `draft.openFile(sessionId,path)` (Mac player); else `openInPanel({kind:"file", path, from:"session", sessionId})` (`DESK/sessions.tsx:3158-3168`) |
| Window (`kind:"window"`) | `AppWindow` 12px + title | `Open window <title>` | `onOpenWindow(url)` -> `draft.openWindow(id,url)` |
| Judged (`kind:"judged"`) | `Scale` 12px + `Judged <span.mono>{title}</span>` + `Stamp still` class `armada-session-act__at` | `Judged <title>` | `row.onOpen` (opens the artifact at its notes; rows produced in `DESK/judge/thread.tsx`, client-only) |

### B1.7 Order and folding (B1)

- Rows keep wire/record order. Sketch/file rows replace the `show_sketch`/`show_file` tool row in place (`DESK/session-thread-rows.ts:82-153, 255-258`).
- Window rows: the `show_window` tool rows are filtered out in spacious (`UNDRAWN`); the window row stands for them.
- The document card is inserted before the first row whose time is later than the document's `kept` time (`packages/screens/src/thread-cards.ts:105-132`); a row whose time does not parse is never "later", so the card follows it.
- Sketch `replaced`: set on every card of a sketch id except the last one with that id (`DESK/session-thread-rows.ts:155-222`); the pinned sketch removes its own non-replaced card.
- These cards are never grouped into a tool-call fold.

### B1.8 States

| State | Drawn as | Condition |
|---|---|---|
| plain | B1.2 | `!(asked && !withdrawn) && settled === undefined` |
| replaced | dashed press border, glass+title opacity 0.5, band tag `Replaced`, no answers | `row.replaced` |
| against | band tag `Against <title>` | `row.against !== undefined && !shown.earlier` |
| waiting for answer | asked frame + footer | `open` (B1.3) |
| approved | line `Approved` with BadgeCheck, footer gone | `settled.how === "approved"` |
| sent back | quote of note + `Sent back` with Reply | `settled.how === "sent_back"` |
| withdrawn | falls back to the plain card (no frame) | `withdrawn && settled===undefined` |
| earlier revision shown | read-only, no notes | `shown.earlier` |
| playing | scene steps loop | pointer over or focus in the press |

### B1.9 Keys

No card-level keys of its own.
The pinned sketch and an asked sketch/document card are **not** registered with `useThreadCard` (only plan, draft, permission, question, done, handoff register).
`⌥↵/⌥E/⌥R/⌥X` do nothing on them.
Press on the card is a button (Enter/Space natively).
Esc in the note `Textarea`: none bound.
Tab order: press, revision menu button, then footer buttons.

### B1.10 Accessibility

`role="article"` + the names above; `role="img"` + `aria-label` on the settled mark; `aria-expanded` on Fold; `aria-pressed` on Changes/Whole; `role=group aria-label="Answers"`, `"Read as"`; bare glyphs are `aria-hidden`.

### B1.11 Fleet data

| Card | Wire (`crates/ipc/src/job_thread.rs`) | Electron today |
|---|---|---|
| Sketch | `ThreadItem::Sketch(ThreadSketch{id, at, sketch ("sketch:<title>"), title, drawing: RawScene, against, against_drawing, replaced, asked, withdrawn, settled: CardSettled{how, at, words}})` (:224, :255-279) | **Not read from the wire.** TS parses the `show_sketch` tool row text (`sketchCallOf`, `packages/screens/src/sessions-wire.ts:435-470`), looks up ledger sketch `sketch:<title>`, merges `kept` hashes (`keptFrom` :235), derives `replaced`, `againstScene`, `asked/withdrawn/settled` on the latest card (`DESK/session-thread-rows.ts`). **`revisions` (each drawing for the menu) is not in `ThreadSketch`**; Electron reads each revision file through the draft (`useSketchRevisions`, `DESK/session-sketch.ts:132-165`) |
| File | `ThreadItem::File{id, at, path, name}` | TS: `fileCallOf` regex on the tool row `show_file <path>` + ledger artifact `form:"file"` whose id equals the path or ends with `/<path without ./>`; path = ledger id, name = ledger title (`session-thread-rows.ts:116-153`); `mediaKindOf(path)` extension decides MediaFrame (not in wire) |
| Document | `ThreadItem::Document(ThreadDocument{id "document:<page>:r<rev>", at, page, title, revision, asked, settled})` "one per revision kept" | TS: **one card per page**, its newest revision only, `id = document:<page>:r<n>`, `at = artifact.kept`, `asked/settled` from the artifact (`thread-cards.ts:79-103`); the wire says one per revision kept: QUESTION below |
| Window | `SessionRow::Window{title,url,variations,steps,origins,brief,ended_at}` via `ThreadItem::Row` | passed as is |
| Judged | none | client-only (`DESK/judge`) |
| Answer | `answer_sketch {session_id, sketch, drawn, how, words?}` / `answer_document {session_id, document:"document:<page>", revision, how, words?}` | `drawn`/`revision` are read from the ledger at press time (`thread-cards.ts:17-71`) |

QUESTION: `MediaFrame` (`packages/components/src/compositions/MediaRow/MediaRow.tsx`) and the `SketchScene` preview (`packages/components/src/compositions/SketchScene/`) were not read: their pixels (picture frame size, video chrome, scene preview scaling) are not recorded here.
QUESTION: wire `ThreadItem::Document` says "one per revision kept" while Electron draws only the newest revision per page; which does the native card follow?
QUESTION: `ThreadSketch` has no `revisions`; where does the native app get the revision scenes the menu needs?

---

## B2 The plan card

### B2.1 Where it lives

| What | Where |
|---|---|
| Card | `PlanCard` `ST/SessionThread.tsx:2795-3195` (not exported) |
| Type | `PlanApproval` `ST/SessionThread.tsx:653-714` |
| CSS | `ST/SessionThread.css:181-460` (plan), `1329-1370` (changes list), `1284-1327` (Plan changes row), `164-179` (`__ask`), `1583-1594` (`__tail`) |
| Revision (diff) view | `PlanRevisionView`, `useUnchanged` `ST/PlanRevision.tsx`, CSS `ST/PlanRevision.css` |
| Landing runs | `landingsOf` `ST/lands-in.ts:19+`, `LandsInChip` `ST/LandsInChip.tsx`, `JobCard` `packages/components/src/compositions/JobCard/` |
| Task mark | `TaskMark` `packages/components/src/compositions/TaskMark/TaskMark.tsx` + `.css` |
| Judge notes pinned on the plan | `GroupFindings`, `GivesRow`, `FindingNotes`, `anchorsOf`, `useFindingKeys` (`packages/components/src/compositions/AdversarialReview/AnchoredFindings.tsx`); press `usePlanJudge` `DESK/judge/thread.tsx:89-139` |
| Built in Electron | `approval` in `DESK/sessions.tsx:2467-2562` |
| Page the card opens (`onReview`) | `PlanReview` / `PlanBody` / `PlanPane` `DESK/session-view/PlanReview.tsx`, edit pieces `PlanEdit.tsx`, marks `plan-marks.ts`, diff data `plan-revision.ts` |
| Not the card (the Plan panel / Story column) | `PlanGroup.tsx`, `PlanSteps.tsx` (`stepsOf`), `StoryColumn.tsx`, `StoryParts.tsx`: used by `BlueprintPanel`, `FlightPlan`, `StoryColumn`; they are the plan panel after approval, not the thread card |

### B2.2 Placement in the thread

| Rule | Where |
|---|---|
| `approval` is built only when `planDrawn` = plan exists and (`approved` \|\| `setAside` \|\| `planAsked(session)` = a waiting item with `source:"plan"`) | `DESK/sessions.tsx:2395-2402`, `packages/screens/src/draft/sessions.ts:2100` |
| **Not approved (waiting or set aside)**: one `li.armada-session-thread__tail[aria-label="Waiting on you"]` after all rows, before the working mark, subagents and `trailing`: `display:flex; column; gap: var(--space-3); list-style:none` (`SessionThread.css:1583-1587`); inside it the order is plan card, pinned sketch (`SketchPin`), `AskCards` (empty in Electron) | `ST/SessionThread.tsx:3866-3900, 4172-4176` |
| `asking` = `(approval && !approval.approved) \|\| sketching \|\| asked \|\| lapsed`; a set-aside plan has `approved:false`, so it still counts as asking (keeps the last Fleet turn open, hides the live mark on the last calls) | `:3745-3749` |
| `.armada-session-thread__tail > .armada-session-thread__ask`: `max-height:none; flex-shrink:0` | `SessionThread.css:1589-1592` |
| **Approved**: a record `li.armada-session-thread__record` (`list-style:none`) after the item whose row id equals `approval.after`, only for items of `kind:"row"`. If `after` is undefined or matches no row item, **the approved plan is not drawn at all** | `ST/SessionThread.tsx:4160-4170` |
| `after` = id of the last drawn row before the `plan` row for this version (`afterOf`) | `packages/screens/src/session-work.ts:131-145` |
| Scroll pin: a layout effect re-pins when the `approval` state flips (0 none, 1 pending, 2 approved) | `:3672-3680` |
| A chip over the composer (`CallChip`, word `Plan`) appears when the plan is waiting and the tail is scrolled out of view | B5.1 |

### B2.3 Layout (pending / open)

`Card flat.armada-session-thread__ask.armada-session-thread__plan` with `role="article"`, `aria-label` = `Plan approved` \| `Plan set aside` \| `Plan to approve`; data attrs `data-approved`, `data-aside`, `data-folded`, `data-findings` (judge notes present), `data-jobs` (some group goes to its own Job).
`ref`/`tabIndex`/`data-thread-target` come from `useThreadCard` (`bind`); then `review` spreads `tabIndex:0 data-reviewable onClick onKeyDown` (so a reviewable card is a tab stop).

Card box (`SessionThread.css`): base `__ask` flex column gap `--space-2`, `flex-shrink:0; max-height:60vh; overflow:auto` (:164-172); `__plan`: `max-height: calc(var(--space-12)*5)` = 240px, `overflow:hidden` (:193-196); `[data-jobs]`: `max-height:none` (:198-200); `[data-findings]`: `max-height:70vh` (:202-204); `[data-reviewable]`: `cursor:pointer`, `transition: border-color 120ms ease`, hover border `--accent`, focus-visible ring (:182-192).
Padding `--pad-card` = 12px (frame override), radius 6, border default, bg raised.
`[data-aside][data-folded]`: `padding-block: var(--space-1)`, color `--fg-muted`; band colour `--fg-subtle` (:256-263).

Regions, top to bottom:

| # | Region | Detail |
|---|---|---|
| 1 | `div.__plan-head` | flex, `align-items:center; justify-content:space-between; gap: space-3` (:205-210) |
| 1a | `button.__plan-band` | `type=button`; `flex:1 1 auto; display:flex; align-items:center; gap: space-2; min-width:0; padding:0; border:0; bg transparent; color --fg-muted; font:inherit; text-align:start; cursor:pointer` (:211-224); hover color `--fg-default`; focus-visible ring. `aria-expanded={!folded}`, `aria-label` = `Plan set aside` or `Plan`. Press toggles `folded`. Children: `ListChecks` 12px; `span.__eyebrow` text `Plan` (or `Plan set aside`), 12/16, weight 600, caps, uppercase, in the plan `display:flex; align-items:center; gap: space-2` (:314-317, :441-446); `span.__plan-version` `v<version>` (mono 2xs, `--fg-subtle` :232-236) if `version` is given; state mark (omitted when set aside) `span.__settled[role=img][aria-label=<stateSaid>][data-state=going\|approved\|waiting]` with a 16px glyph: `LoaderCircle` (going; rotates `armada-session-you-turn var(--duration-pulse) linear infinite`, colour `--fg-muted`; none under reduced motion), `BadgeCheck` (approved; colour `--step-advanced` :319-321), `BellRing` (waiting; colour `--step-waiting` :240-242); tooltip text = `stateSaid`; trailing `ChevronDown` (open) / `ChevronRight` (folded) 12px, class `__plan-fold` (`margin-inline-start:auto`) |
| 1b | head `Button sm primary` `Approve` + `ThreadKbd yes` | only when `!approved && folded && !aside`. Calls `approval.onApprove` |
| 2 | `ul.__changes[aria-label="Changed since the last plan"]` | only when open, not `revising`, and `approval.changes.length>0`. flex column gap `--space-1`, 13/18, no list style (:1329-1338). `li[data-kind]` grid `auto 1fr`, `align-items:start`, `column-gap: space-2` (:1340-1346). Glyph span `role=img` `aria-label` and tooltip `Added` (`Plus`) / `Renamed` (`Pencil`) / `Removed` (`Trash2`), 12px; colours: added `--step-advanced`, renamed `--step-running`, removed `--step-failed` (:1355-1366). Text `span.__change-text` (flex column, `overflow-wrap:anywhere`): renamed shows `<s>{from}</s>` above `<span>{text}</span>`; removed shows `<s>{text}</s>`; `s` colour `--fg-muted`; removed row colour `--fg-muted` |
| 3 | `div.__plan-body` | `display:grid; grid-template-columns: minmax(0,1fr); gap: space-4; align-items:center` (:395-400); with a sketch: `minmax(0,3fr) minmax(0,2fr)` (:401-403); `min-height:0; overflow:auto; scroll-padding-block: space-3` (:309-313). When pinned (judge notes shown) it also carries `keys.bind` |
| 3a | plain mode (`approval.summary` defined) | `div.__plain` (flex column, `align-items:flex-start; gap: space-2` :264-270): `p.__plan-summary` (`margin:0; white-space:pre-line; 14/20; color --fg-default` :271-277) `{summary}`; `FindingNotes` label `Findings on the plan` (if judge notes); `ul.__gives[aria-label="What each group gives"]` per landing run (flex column gap space-1, `padding: 0 0 0 space-4`, color `--fg-muted` :278-285) with `li` = `group.gives ?? group.title` (or `GivesRow` when judge pins exist); `Button sm ghost` labelled `Details` / `Show less` (`aria-expanded`) toggling `technical` below |
| 3b | technical mode | `technical` = `PlanRevisionView` when `revising`, else `groups` (below). If judge notes exist and not plain: `div.__pinned` (flex column gap space-2) with `FindingNotes` + technical |
| 3c | `ul.__groups` | flex column gap `--space-2`, no list style (:323-331). Each `li.__group`: grid `minmax(0,1fr) auto`, `align-items:center; column-gap: space-3; row-gap: space-1` (:353-360). `span.__group-title` (weight 600, `overflow-wrap:anywhere`); `span.__marks` (flex wrap, `justify-content:flex-end; max-width:50%; gap: space-1`) with one `TaskMark state="open"` per task, **not drawn when `approved`**; agent chip `Tooltip("Subagent type")` > `span.__agent[role=img][aria-label="Subagent type <agent>"]` (inline-flex, gap space-1, color `--fg-muted`) with `Bot` 12px + `<code>{agent}</code>`; in plain mode `span.__paths[aria-label="Paths"]` (flex wrap gap space-2, 12px, `--fg-subtle`) of `<code>{path}</code>`; judge `GroupFindings` when pinned and not plain |
| 3d | landing runs (`landed`) | one run with no `landsIn` and no `own` draws the list bare. Otherwise `div.__landings` (flex column gap space-3 :332-338) of `section.__landing[aria-label=<own title> or "Lands in #<n>" or "Lands in <key>"]` (flex column gap space-2; `> ul` gets `margin-inline-start: space-3; padding-inline-start: space-3; border-inline-start: 1px solid --border-subtle`; `> .__gives` padding-inline-start space-4). Own-Job run: `JobCard` (placeholder `placeholder=<title>` until handed; then `title state said handedFrom at onOpen`, optional `issue` chip) and no group list. Other run: `LandsInChip` (`PullRequestChip framed`; placeholder with the run label until opened, else number/title/state with `onOpen(number)`) then the list. Runs come from `landingsOf(groups, approval.opensPullRequests===true)`: a group with `ownJob` is its own run `job:<id>`; others gather by `landsIn.label` (default label `PR 1` when the Job opens PRs); with `opens` false no chip |
| 3e | sketch (`approval.sketch`) | `ul.__sketch` (no margin/padding/list) with a `SketchRow` built from `{id: sketch.id, at:"", kind:"sketch", sketch: id, title, drawing: scene}`, `onOpen={() => onReview?.()}`. In the plan its `__band` is `display:none` and `__glass` `block-size: calc(var(--h-row-unit)*3)` = 108px (:404-409) |
| 4 | `div.__answers[role=group][aria-label="Answers"]` | only when open and not approved. flex, `align-items:center; gap: space-2` (:456-460). `Button sm primary` `Approve` + `ThreadKbd yes`; `Button sm secondary` `Suggest` + `ThreadKbd change` (not when `aside` or no `onReview`; opens the full plan to suggest in); `Button sm secondary` `Not yet` + `ThreadKbd sendBack` (not when `aside` or no `onSetAside`); then `judged.press` (the `Judge plan` act, B2.6) when a judge exists |

`ThreadKbd` shows a `Kbd` with the key (`⌥↵`, `⌥E`, `⌥R`) **only while this card is the target** (B6).

### B2.4 States and the condition that picks each

| State | Drawn | Condition |
|---|---|---|
| waiting | band `Plan`, BellRing amber, answers row | `!approved && !aside && going!==true` (`waiting`, :2873), `approval` from `planAsked` |
| going | LoaderCircle spinning, tooltip `Sending` | `approval.going` = `session.pressed?.includes("plan")` (client-only `pressed`) |
| approved | BadgeCheck green, `Plan approved`; no Approve/answers, no task marks; sits as a record in the line | `approval.approved` = `plan.approved`; placement B2.2 |
| set aside | folded chip: band `Plan set aside`, no state mark, quiet colours; press unfolds (`folded` re-set true whenever `aside` flips true) | `aside = setAside===true && !approved` (`plan.set_aside_at` defined) |
| folded | head only; Approve in the head (not approved/aside) | `folded` state (default `aside`) |
| reviewable | pointer, hover accent border, tab stop; click anywhere except a `button` opens the full plan; Enter/Space on the card itself too | `onReview` defined and not folded (`review` object, :2820-2837) |
| revising | body is `PlanRevisionView` (diff), the `__changes` list is hidden, `U` works | `approval.revision !== undefined && !approved` (`revision` is set when version >= 2 and the record has changes, `DESK/session-view/plan-revision.ts:11-17`) |
| findings | max-height 70vh, notes pinned under the plan/group/task | `judged` defined, not approved, `findings.length>0` |
| jobs | no max-height | any run with `own` |
| target | focus ring from `[data-thread-target]` | B6 |

### B2.5 PlanRevisionView (`ST/PlanRevision.tsx`, `PlanRevision.css`)

`div.armada-plan-revision` (flex column gap space-2) > `ul.__groups[aria-label="Plan"]` (flex column gap space-2).
A group is **changed** if it has `since`, or `gone.length>0`, or any task with `since`.
- Changed group: `li.__group` (flex column gap space-1) > `div.__head` (flex wrap, `align-items:center; gap: space-1 space-3`) with `span.__title[data-kind]` (inline-flex, `align-items:flex-start; gap: space-2; flex: 1 1 calc(var(--space-12)*2)` = 96px basis; weight 600; a `ChangeMark` first when added/renamed; the text column shows `<s>{from}</s>` above the title) and the agent chip (right: `margin-inline-start:auto`); then `ul.__tasks` (grid `auto auto minmax(0,1fr)`, column-gap space-2, row-gap space-1, `padding-inline-start: space-3`, 13/18) of `li.__task` (subgrid; colour `--fg-muted`, `--fg-default` when it carries a change): [change mark or empty `span aria-hidden`] [`TaskMark state`] [text with `<s>` for renamed]; then `GoneLine`s (removed mark, empty, `<s>{text}</s>`, colour `--fg-muted`).
- Unchanged group: `li[data-quiet]` > `button.__fold.__head` (`aria-label=<title>`, `aria-expanded`, `aria-controls` when open; tooltip `Show tasks` / `Hide tasks`): `ChevronRight`/`ChevronDown` 12px, title in `--fg-muted`, task marks (`span.armada-session-thread__marks`), agent chip.
  Button CSS: inline-flex, `gap: space-2`, `padding: space-1 space-2`, `margin-inline-start: -space-2`, radius md, transparent, 13/18, hover bg `--bg-hover` + `--fg-default`, width `calc(100% + space-4)`, `margin-inline-end: -space-2`.
  Open shows `ul.__tasks` with `padding-inline-start: space-4`.
- ChangeMark: `span.__change[role=img][aria-label=Added\|Renamed\|Removed]` 12px glyph `Plus` / `Pencil` / `Trash2`, colours as B2.3 row 2.
- `useUnchanged`: set of open unchanged group ids; `toggleAll` opens all, or closes all once every one is open.
- `TaskMark` (12px glyph, `role=img`, tooltip = verb capitalised): `open` CircleDashed `--fg-subtle` (tooltip `Open`), `working` CircleDot `--step-running`, `handed_in` Check (+ 8px PencilOff corner), `done` Check `--step-advanced`, `failed` X, `dropped` Minus; Fleet `marked`/`stale` add tooltip lines `Marked by Fleet: <text>` and `Stale: <why>, <age>`.

### B2.6 Judge pieces on the card

- `judged` = `usePlanJudge(session)` (`DESK/judge/thread.tsx:89-139`): defined when a judge API exists, a plan exists and is not approved.
  Props: `findings` (notes mapped by `findingOf`), `onRun` (`openJudge`), `onSend`, `onDismiss`, `press` = `<JudgeAct label="Judge plan" shortcut={keyFor("plan_judge")}>` (a Scale-glyph act; opens a small popover asking what to verify, placeholder `existing sign-ins survive the switch`; `DESK/judge/JudgeAct.tsx`).
- Keys (focus anywhere in the card, `onKeys`, `:2862-2871`): `U` (plain key, no meta/ctrl/alt) toggles unchanged groups when `!folded && revising`; `plan_judge` (`R`, scope `plan`, Contextual) calls `judged.onRun()` when `!folded && judged`; otherwise the review key handler (Enter/Space on the card itself opens the full plan).
  Findings keys (`useFindingKeys` `AnchoredFindings.tsx:42-92`, bound on `div.__plan-body` when pinned): `finding_move` (`j / k / ↓ / ↑`) moves between notes, `judge_note_send` (`Enter`) sends the focused note, `judge_note_dismiss` (`d / Backspace`) dismisses it, focus passes to the next.
  Scope `plan` in `crates/core-model/domain/actions.toml:1121-1165`.
- The card key handler is attached only if `revising || judged !== undefined` (`:3013-3017`); otherwise `review.onKeyDown` from the `review` spread.

### B2.7 Acts (what each does)

| Control | Label (exact) | Does | Shown when |
|---|---|---|---|
| Approve (head) | `Approve` | `approval.onApprove` -> `approvePlan` (`DESK/sessions.tsx:2124-2129`): no marks -> `draft.work.approve(session.id)` (`approve_plan {session_id, version}`); with marks -> `draft.work.suggest(id, marks, true)` (`send_plan_changes {…, approve:true}`) then clears marks | folded, not approved, not aside |
| Approve (answers) | `Approve` | same | open, not approved |
| Suggest | `Suggest` | `onReview` -> opens the Plan review sheet (`setReviewing(true)`) where marks are made | open, not approved, not aside, `onReview` given |
| Not yet | `Not yet` | `approval.onSetAside` -> `draft.work.setAside(session.id)` (plan set aside, agent told to keep talking); the card becomes the chip | open, not approved, not aside, `onSetAside` given |
| Band | `Plan` / `Plan set aside` | toggles fold | always |
| Details / Show less | `Details`, `Show less` | toggles technical groups under the summary | plain mode only |
| Judge plan | `Judge plan` | judge popover | judge exists |
| Card press | (no label) | `onReview()` | reviewable |
| Sketch thumbnail | `Open sketch <title>` | `onReview()` (not the sketch panel) | `approval.sketch` |
| Pull request chip | chip | `onOpenPullRequest(number)` -> `read({kind:"pull_request", number})` -> side panel | landing run with a PR |
| Job card | | `onOpen` -> open that Job | own-Job group handed |

### B2.8 The page the card opens (Plan review sheet): marks, edit, reach

Mounted from `sessions.tsx` (`reviewing` state; `<PlanReview>`/`PlanPane`). Recorded at the level the brief asks.

- `PlanReview` (`DESK/session-view/PlanReview.tsx:215-336`): `Sheet kind="plan-review" floating size="widest" title="Plan" closeLabel="Close" closeBinding="Esc"`, wrapped in `OverTheApp`; `controls` = `BadgeCheck` 16px with tooltip `Plan approved` when approved; `footer` (`div.sv-review__foot`) shown if `marked || !approved`: `Edit plan`/`Show changes` ghost sm (only if a `revision` exists, plan not approved, no marks), `Send changes` (secondary sm, `Send` 16px glyph, only if marks), `Approve` primary sm with tooltip `Approve with these changes` (marks) or `Approve`.
- `PlanBody` (`:55-190`): `div.sv-review` > `div.sv-review__groups` (findings keys bound if judge notes) > either `PlanRevisionView` (when `revision` and not `editing` and no marks) or: `FindingNotes` `Findings on the plan`, landing runs (`landingsOf`) each `div.sv-review__landing[role=group][aria-label=...]` with `LandsInChip` + one `section.sv-group.sv-review__group[data-open][aria-label=<title>]` per group holding a `GroupEditor`; own-Job runs are `JobCard`s; then `AddGroup`.
  A plan sketch is drawn large below: `div.sv-review__sketch` > `SketchReading`.
- `PlanPane` (review sheet pane): summary `p.sv-blueprint__summary`, `Edit plan`/`Show changes` foot, `PlanBody`.
- Marks (`plan-marks.ts`): `PlanMark` = `rename {group, task?, from, to}`, `remove {group, task?}`, `add {group, task?|id, text}`, `move {task, from, to}`, `agent {group, from, to}` (UI name; wire `Subagent`), `note {id, group, task?, text}`.
  Wire `PlanMark` (`crates/ipc/src/session_plan.rs`): `Rename`, `Remove`, `AddTask`, `AddGroup`, `MoveTask`, `Subagent`, `Note`.
  `viewOf(plan, marks)` lays marks over the plan (renamed shows the old text struck, removed struck, added marked new, moved shows source, notes pinned, `since` from agent changes, `gone` = tasks the agent removed).
  `linesOf` words marks as lines (`Renamed`, `Removed`, `Added`, `Moved`, `Subagent`, `Note`; `where` = `<group title> › <task>`).
  Helpers: `rename`, `remove`, `addTask`, `addGroup`, `move`, `setAgent`, `addNote`, `dropNote`, `newId(prefix)` -> `<prefix>-new-<n>`.
- Edit pieces (`PlanEdit.tsx`): `EditText` (a button showing the text, `aria-label="<label>: <value>"`; press -> `Input` `aria-label=<label>`; Enter saves, Esc/blur keeps old); `Tools` hover glyphs: `Suggest` (MessageSquarePlus 16, aria `Suggest on <what>`), `Teach` (GraduationCap, aria `Teach on <what>`, only when teaching is available), `Remove`/`Keep` (Trash2 / Undo2, aria `Remove <what>`/`Keep <what>`); `NoteBox` (Enter pins, Shift+Enter newline, Esc closes; close button `Close suggestion`); notes list `ul.sv-pins[aria-label="Suggestions"]` with mark tooltip `Suggestion, not sent yet` and take-back button aria `Take back suggestion` (tooltip `Take back`); agent marks `Added by the agent` / `Renamed by the agent, was <from>`; `Added, not sent yet`; `Removed by the agent`; task rows `draggable` (drag a task onto another group to move; dataTransfer `text/plain` = task id); group subagent `Select` aria `Subagent type of <title>` with options `rust-engineer, bridge-engineer, protocol-engineer, fixture-author, judge, general-purpose` (tooltip `Subagent type`); `Edit group` tooltip; `AddTask` input aria `New task in <group title>` (Enter adds if non-blank, Esc closes); `AddGroup`: ghost sm `Plus 16` + `Add group` -> `Input` aria `New group` (Enter adds, Esc/blur closes); `SendChanges`: `Send changes` secondary sm with `Send` 16 (only with marks) -> `ed.send`.
- `sendBack(note)` in the review sheet: the note rides on the plan's first group as a `note` mark then `draft.work.suggest(session.id, marks)`; marks cleared (`DESK/sessions.tsx:2594-2612`).
- The thread afterwards shows the `Plan changes` row (B2.9).

QUESTION: the pixel CSS of the review sheet (`.sv-review*`, `.sv-group`, `.sv-edit*`, `.sv-pins`, `.sv-since`, `.sv-task`) in `DESK/session-view/session-view.css` was not recorded; the sheet is likely another part's, only its parts and behaviour are listed above.
QUESTION: `Sheet`, `OverTheApp`, `Dialog`, `JobCard`, `PullRequestChip`, `JudgeAct` popover internals were not read.

### B2.9 Rows that belong to the plan in the thread

`suggest` row = Plan changes (`ST/SessionThread.tsx:2223-2257`, wire `SessionRow::PlanChanges{version, approved, lines[]}`): `li.armada-session-suggest[role=region][aria-label="Changes to the plan"]`: flex, `align-items:flex-start; gap: space-3; padding: space-2 space-3; border: 1px solid var(--you-edge); radius md; bg color-mix(you 14%, --bg-raised); color --fg-default` (`SessionThread.css:1284-1293`).
Children: eyebrow `Plan changes`; `ul.__lines` (flex column gap space-1); each `li.__line` (flex wrap, `align-items:center; gap: space-2`, 13/18): verb glyph 12px with tooltip+aria = verb (`Renamed` Pencil, `Removed` Trash2, `Added` Plus, `Moved` MoveRight, `Subagent` Bot, `Note` MessageSquare), `span.__where` (mono 2xs), optional `<s>{from}</s>` (opacity .7), `ArrowRight` 12 when both from and text, `span.__text`; then `RowStamp`.
The plain `Plan` wire row (`SessionRow::Plan{version}`) draws nothing; it only anchors `after`.

### B2.10 Fleet data

| Need | Wire | Electron TS |
|---|---|---|
| plan content | `SessionRecord.plan: SessionPlan{version, summary, approved, set_aside_at, groups[{id, agent, title, gives, paths, tasks[{id,text,state,seen,marked,stale,changed_at}], issue, lands_in{label,pull_request}, own_job, gate…}], changes{added,renamed,moved,removed}, sketch{id,title}, held_by, held}` (`crates/ipc/src/session_plan.rs`, `sessions.rs:1370`). **`ThreadHead` has no plan field**; `JobThread` carries only the `plan` / `plan_changes` rows | mapped by `planOf` (`packages/screens/src/session-work.ts:154-260`): empty summary omitted, `setAside` = `set_aside_at` present, sketch scene taken from the ledger attachment (left off if the ledger lacks it) |
| waiting | `waiting_for` item `source:"plan"` | `planAsked` |
| `after` | rows | TS `afterOf` (id of last drawn row before the plan row) |
| `changes` lines | `plan.changes` | TS `changeLines` (`plan-marks.ts:488-523`): added then renamed then removed; text found by id in groups/tasks, else the id |
| `revision` | `plan.changes` | TS `planRevision` -> `viewOf(plan, [])` per group (`plan-revision.ts`) |
| `going` | none | client `session.pressed` |
| `opensPullRequests` | | TS `opensPullRequests(session)` |
| own-Job group card | child sessions | TS `jobOf(group)` finds the child session with that `group` id; `stateOf(child)`; issue chip `cardIssueOf(issueOfGroup(...))` |
| `judged` | none | client `usePlanJudge` |

---

## B3 The draft card

### B3.1 Where it lives
`DraftRow` `ST/DraftRow.tsx:1-530` (row `kind:"draft"`, `ST/SessionThread.tsx:444-466`); CSS `ST/DraftRow.css:1-101`; `DraftAnswer` type `DraftRow.tsx:29-39`; mounted `ST/SessionThread.tsx:2126-2147`.

### B3.2 Layout
`li.armada-session-draft-row` (`list-style:none`) > `Card flat.armada-session-draft[role=article][aria-label="<Draft\|Rule\|Workflow>: <title>"][data-settled]` (+ `bind` from `useThreadCard`, + `teaching.on` context-menu handlers).
Card = base `Card` (12px pad in the frame, gap `--space-3`, radius 6, border default, bg raised).
No spacious rule; no bleed.

| # | Region | Detail |
|---|---|---|
| 1 | `div.__head` | flex, `align-items:center; gap: space-2; color --fg-muted` (:6-11): `span.armada-session-thread__eyebrow.__eyebrow` text `Draft` / `Rule` (when `row.rule`) / `Workflow` (when `row.workflow`); `Stamp at=row.at` (live, a button) class `__at` (`margin-inline-start:auto; color --fg-subtle`) |
| 2 | `p.__title` | `margin:0; 14/20; weight 600; overflow-wrap:anywhere` (:18-24) |
| 3 | facts `dl.__facts` | workflow: `Name` / `{name}`, `Scope` / `Every repository on this Mac` (kit) or `This repository`; rule: `Where` / `{where}`, `To` / `{to}`. grid `max-content minmax(0,1fr)`, gap `space-1 space-4`, dt mono 2xs caps `--fg-muted`, dd margin 0 (:86-101) |
| 4 | body | edit mode: `Textarea aria-label="Draft text" rows=8 grow autoFocus keep="<keep>:fixed"`; else `div.__text` (`min-width:0; overflow-wrap:anywhere`) rendering `Prose` of the text (a settled `fixed` shows `settled.words`), with `ProseRunner`/`ProseCommand` = null (code blocks have no Run). When `open`, a right press on selected words inside offers a menu (B3.5) |
| 5 | before | `div.__before` (flex column, `align-items:flex-start; gap: space-2`): ghost sm `Text it replaces` (`aria-expanded`) toggling `Text` on `--bg-sunken`, padding `space-2 space-3`, radius sm, color `--fg-muted` (:32-45). Only if `row.before` is defined |
| 6 | note box | `Textarea aria-label="Note" rows=3 autoFocus keep="<keep>:note"` when noting |
| 7 | settled note | `blockquote.__note` (same as the sketch note: left 2px `--you-edge`, 14/20, pre-wrap) when `how` is `sent_back` or `withdrawn` and `words` is present |
| 8 | settled line | `p.__settled[data-how]` (flex gap space-2, 13/18, `--fg-muted`) with a 16px glyph in `span.__mark[role=img]` (colour `--step-advanced`; `--fg-muted` for sent_back/withdrawn) + `<span aria-hidden>` word (:59-78) |
| 9 | footer | `CardFooter[role=group][aria-label="Answers"]` when `open` |

### B3.3 Settled words and glyphs (`DraftRow.tsx:48-65, 210-226`)
`approved` -> `Approved` BadgeCheck; `fixed` -> `Approved with your fixes` BadgeCheck; `sent_back` -> `Sent back` Reply; `withdrawn` -> `Withdrawn` Ban.
Overrides: proposed workflow + withdrawn -> `Dismissed` Ban; rule + approved -> `Sent` BadgeCheck.

### B3.4 Modes, footer and keys

State: `mode` read | edit | note (initially edit if a kept `:fixed` exists, else note if kept `:note`, else read).
`open = settled===undefined && onAnswer!==undefined`; `editing = open && mode==="edit"`; `noting = open && mode==="note"`.
Settle clears kept text.

| Mode | Footer buttons (all `size sm`) |
|---|---|
| read | `Approve` (primary; `Send` for a rule; hidden when `headed`) + yes kbd; if not workflow: `Edit` (secondary) + change kbd, and unless rule/headed: `Note` (secondary) + sendBack kbd; then the end-aligned Withdraw (if `onWithdraw`): `span.__withdraw` (`display:flex; margin-inline-start:auto`) > secondary `Withdraw` (`Dismiss` for a workflow) + no kbd |
| edit | `Approve with fixes` (primary; `Send with fixes` for rule; disabled if text blank; sends `{fixed}` or `{choice:0}` when unchanged) + yes kbd; `Cancel` (ghost; restores text, forgets keeps) |
| note | `Send back` (primary; disabled if note blank; `{text: note.trim()}`); `Cancel` (ghost); Withdraw (with the typed note as reason) |

`Approve` sends `{choice:0}`.
`⌘Enter` (`isPressed("send_message")`) inside the Draft text box sends `approveFixed` when the text is non-blank, inside the Note box sends `sendBack` when the note is non-blank (`:287-293`).
Edit opened by key focuses the textarea next frame (`openBox`).

Card registration `useThreadCard` id `draft:<row.id>`, kind `Rule`/`Workflow`/`Draft`, title `row.title`, acts:
- `yes`: editing -> `Approve with fixes`/`Send with fixes` (only if text non-blank); noting or headed -> none; else `Approve`/`Send`
- `change`: `Edit` unless noting or workflow
- `sendBack`: `Note` unless editing, headed, workflow or rule
- `no`: `Withdraw`/`Dismiss` unless `onWithdraw` is missing or editing.
Kbd chips (`ThreadKbd`, at rest only on the target card) sit in the buttons above (yes on Approve, change on Edit, sendBack on Note, no on Withdraw).

### B3.5 Selection menu and Teach
`onContextMenu` on the text when `open` and the press is inside a non-collapsed selection within the text: `DropdownMenu triggerLabel="Selection"` at the pointer with items `Edit` (Pencil; opens edit with those words selected in the box) and `Add note` (MessageSquarePlus; opens the note quoted with `quoteOf(words)`; omitted when `headed`).
`useTeachMenu("Draft", ...)` adds a Teach context menu on the card (`onTeach({kind:"words", text: row.text, at: row.at})`).

### B3.6 Wire and Electron
Wire: `SessionRow::Draft{id, at, title, text, before, settled: DraftSettled{how: approved\|fixed\|sent_back\|withdrawn, words, at}, change}` (`hosted_sessions.rs:700-720, 501-520`) via `ThreadItem::Row`.
The row is replaced by id on settle.
Electron: pass-through plus `withWorkflowCards` (`DESK/sessions.tsx:3810-3828`): a draft with `change` and no `workflow` gets `workflow{name: proposedName(...), scope}` looked up asynchronously by change number (`useProposedScopes`); `rule` is a mock-only field.
Answer routing: `onDraft(rowId, answer)` -> `draft.answerWaiting(session.id, "draft:<rowId>", answer)`; `onWithdrawDraft(rowId, reason?)` -> `dismissWaiting(session.id, "draft:<rowId>", reason)`; `keep = draft:<sessionId>:<rowId>`.
The review sheet draws the same card with `headed` (its Approve/Send back in the sheet head).

---

## B4 The asked question card, permission card and picks

### B4.1 Where it lives and who hosts it
| What | Where |
|---|---|
| `AskCards` | `ST/SessionThread.tsx:3293-3511` (export) |
| `askLabel` / `askText` | `:3278-3290` |
| Types `AskOffer`, `AskedQuestion`, `QuestionAnswer` | `:625-651` |
| `QuestionDeck` | `ST/QuestionDeck.tsx:1-540` |
| `ReachAnswers`, `KindPicker`, `REFUSE`, `LIKE_THIS`, `FOLDER_ANSWERS`, `ALWAYS_HEAD` | `:2595-2788` |
| CSS | `SessionThread.css:164-179` (`__ask`), `745-1030` (deck), `1465-1575` (reach), `PathPicker.css` |
| **Host in Electron**: Needs you panel `question.children` | `DESK/sessions.tsx:2245-2350`, `NeedsYouPanel.tsx` (B7) |
| Other deck host: ledger rows with `options` (Fleet waiting items with choices, e.g. the delivery question) | `packages/components/src/compositions/SessionLedger/SessionLedger.tsx:975-1011` (`QuestionDeck flat canSkip={false} noOther={row.onOther===undefined}`) |
| Start picks (held Job) | `DESK/session-view/StartPicks.tsx` (composer lead, not a thread card) |
| Open group | `DESK/session-view/OpenGroup.tsx` (Story column panel, not a thread card) |

### B4.2 What Electron passes (and does not)
`const asked = askedOf(session)` (`DESK/asked-of.ts`) builds `{command, questions?, offers?[{id,label,means}], reaches?, reachesTo?, kind?}`; offer words come from `CALL_ANSWER` (`packages/screens/src/copy.ts:61-110`) via `offeredOf`, or `reachOfferedOf` when `reaches` is set (which renames `refuse`: label `Reject once`, means `Not run. The job is told no, and the next call asks again.`, and `allow_once` means `Runs it now. The next call outside its slots asks again.`).
`lapsed` = `session.lapsed` unless `session.lapsed.id === answeredLapse`.
The panel passes `AskCards flat asked keep onAnswer lapsed [detail pick]`.
`SessionThread` itself receives neither `asked` nor `lapsed`, only `waitingOnYou`.

| Offer id | Label | Means (tooltip) |
|---|---|---|
| `allow_once` | `Allow once` | Runs it now. Nothing is written down, and the same call asks again next time. |
| `allow_and_remember` | `Allow and remember` | Runs it now, and the rule goes into this repository's own personal agent settings — so your terminal stops asking too. |
| `refuse` | `Refuse` (`Reject once` on a reach card) | The job is told no, and says what it could not do. |
| `always_allow` | `Always allow` | Runs it now. Every job reads and writes in the folder chosen, and under it, with no card. Settings, Features lists it. |
| `always_block` | `Always block` | Not run. Every job is refused the folder chosen, and under it, with no card. Settings, Features lists it. |
| `allow_for_session` | `Always allow for this job` | Runs it now. This job reads and writes in the folder chosen, and under it, with no card, until it is closed. Settings does not list it. |
| `allow_like_this_for_session` | `Allow calls like this for this job` | Runs it, and every call of this kind until the job is closed. Settings does not list it. |

### B4.3 AskCards structure
`asked` present: `Card flat.armada-session-thread__ask[role=article][aria-label="Waiting on you"]` (+ `bind` from `useThreadCard`, `ref` forwards to `askRef`).
Base `__ask` CSS B2.3. When the card holds a deck, `.__ask:has(.armada-question-deck)` is `padding:0; border:0; background:transparent; box-shadow:none` (:174-179).
Inside the Needs you detail the panel also sets `.armada-needs-panel__content .armada-session-thread__ask {max-height:none; padding:0; border:0; background:transparent; box-shadow:none}` (`NeedsYouPanel.css:362-370`), so a panel permission card is its content bare.

A. **Permission** (no `questions`):
1. `span.__eyebrow` `Permission` or `Permission to read|write|run` (`reachesTo`).
2. command: if `allowsOnce` (offers include `allow_once`): `div.armada-prose__run[role=group][aria-label=<first command line>]` > `div.__run-block` > `pre.armada-prose__block > code {command}` + `span.__run-acts` with `Tooltip("Allow once")` > `Button ghost sm iconOnly aria-label="Allow once"` glyph `Play` 16 (always visible: `.armada-session-thread__ask .armada-prose__run-acts {opacity:1}` :1279-1281) and the yes `ThreadKbd`.
   Else `p.__command` (`margin:0; mono 2xs/16`, `:447-452`) with the command text.
3. answers:
   - reach card (`reaches` set and offers include a folder answer) -> `ReachAnswers`: `div.__reach[role=group][aria-label="Answers"]` (flex column gap space-2): a row `div.__answers` of the once offers (everything except `allow_once`, `allow_like_this_for_session`, and folder answers), each `Tooltip(means) > Button sm secondary {label}` + `ThreadKbd no` on the `refuse` button when the card is the target; then `div.__always` (flex column gap space-2; `padding: space-2 space-3; border: 1px solid --accent-muted; radius md; bg --accent-faint`): head `Always applies to` (+ `<strong>reads|writes|commands</strong> of|to|on` when `reachesTo`), `PathPicker path=reaches chosen=folder label="Folder for always"` (folders as mono 2xs buttons, `height --h-kbd`, border subtle; chosen = accent on accent-muted; covered = accent-faint; `aria-pressed`; `title`=full folder; separators `›`), `span.__always-covers` (mono 2xs `--fg-muted`; shows `reaches` or `<folder>/*`), a row of folder offers `Button sm secondary {label} <span.__always-folder>{last folder name}</span>` (accent mono) each calling `onAnswer(id, undefined, folder)`; and when the offers include `allow_like_this_for_session` and `kind` is set: `div.__like` (border-top accent-muted, padding-top space-2) with head `Calls like`, `KindPicker` (`role=group aria-label="Calls like"`: words `program` and `script` as buttons, `aria-pressed`, `data-covered` for script when the program is chosen; pressing program = wide), a `Switch size sm` `Only in <folder name>` (off by default) and `Button sm secondary {like.label}` calling `onAnswer(like.id, undefined, onlyIn ? folder : undefined, chosenKind)`.
   - no offers list: `div.__answers[role=group][aria-label="Answers"]`: `Button sm secondary` `Allow once` + yes kbd (`onAnswer()`), `Button sm secondary` `Deny` + no kbd (`onAnswer()`).
   - other offers: one `Tooltip(means) > Button sm secondary {label}` per offer except `allow_once`; the `refuse` button wears the no kbd.
   `.__answers`: flex, `align-items:center; gap: space-2`.

B. **Question** (has `questions`): `QuestionDeck` (key = command), props `flat`, `keep`, `questions`, `onAnswer`, `detail` (Operator read), `pick`, `canSkip = offers undefined \|\| offers has refuse`.

C. **Lapsed** (only when `lapsed` and no `asked`): `Card flat.armada-session-thread__ask[role=article][aria-label="Unanswered"]` > `QuestionDeck key=lapsed.id questions flat canSkip` with `onAnswer` -> `lapsed.onAnswer(answers)` when answers are given, `lapsed.onSkip()` when the answer is `"refuse"`.

Registration (`useThreadCard`, only for permission cards, `permission = asked && no questions`): id `permission:<command>`, kind `Permission` or `Permission to <reachesTo>`, title first command line; `yes` `Allow once` (when no offers, or offers include allow_once); `no` `Deny` (no offers) or the `refuse` offer's label.
A question registers inside `QuestionDeck`.

### B4.4 QuestionDeck layout and behaviour (`QuestionDeck.tsx`, `SessionThread.css:745-1030`)

`div.armada-question-deck[data-flat]` (`--hue: var(--status-awaiting-review)`; flex column gap `--space-2`; `[data-thread-target]` has `outline:none`, no ring). In the panel it is always `flat`.

Not flat: above the card a `div.__done[role=group][aria-label="Answered"]` (flex wrap gap space-1) of ghost sm buttons `<header or question>: <chosen>` (title attr = question) for earlier questions when `front>0`; strips behind (`__behind`, up to 3, `aria-hidden`, `title=question`) peeking above the card: `__stack` `position:relative; padding-top: calc(var(--space-2)*behind)`; each behind strip absolute, `left/right: calc(var(--space-3)*(i+1))`, height `space-2 + radius-lg`, border hue, radius `8 8 0 0`, bg `color-mix(hue 45%, --bg-overlay)`, `opacity: 1 - i*0.25`.

`div.__card[role=group][aria-label=<question>]` (`onKeyDown=key`): not flat: `position:relative; z-index:20; border:0; radius lg; bg --bg-overlay; box-shadow --shadow-overlay; overflow:hidden; animation armada-question-up var(--duration-travel) var(--ease)` (opacity .6, translateY(-24px) scale .97; none under reduced motion).
Flat: border 0, radius 0, transparent, no shadow, no animation, `__band` hidden, `__body` padding 0 (`:905-917`).

Band (`header.__band`, not flat only): bg hue, color `--fg-inverse`, 13px weight 600; `button.__toggle` (flex, gap space-2, width 100%, padding `space-2 space-4`, transparent, `aria-expanded={open}`, `title=question` when closed): `MessageCircleQuestion` 16; `span.__title` (flex 1) = `header` or `Question`; pips `span.__pips[title="<n> of <N>"]` (only when N>1) of `span.__pip` (8px circles, border 1px `--fg-inverse`; here/done filled; done opacity .6); `ChevronDown` (open) / `ChevronRight` 16. Press folds the body (`hidden`).

Body (`div.__body`): flex column, `gap: var(--space-3)`, `padding: var(--space-4)` (halved to 8px inside the Job frame, `session-view.css:1324-1326`; flat: padding 0).
1. `p.__ask`: `margin:0; font-size --text-lg (18/28); weight 600; color --fg-default; text-wrap: balance`; content `RefText(question)` (issue/PR refs become chips).
2. `detail` node (Operator read / steps) if any.
3. `div.__answers[role=radiogroup\|group][aria-label="Options"]`: flex column gap `--space-1`.
   Each option `button.__answer[role=radio\|checkbox][aria-checked][aria-label=<label>][aria-description=<description>][data-on]`: flex, `align-items:center; gap: space-3; padding: space-2 space-3` (4px/6px in the Job frame, `session-view.css:1318-1320`); `border: 1px solid --border-default; radius md; bg --bg-sunken; color --fg-default; weight 500; text-align:left`; hover border hue; `[data-on]` border hue + bg `color-mix(hue 18%, --bg-sunken)`; focus ring; transition border/background 120ms.
   Inside: `<kbd>{index+1}</kbd>` (mono 2xs `--fg-subtle`, `flex:none`; a bare `<kbd>`, not the `Kbd` primitive), `span.__option` (flex wrap, `align-items:baseline; column-gap: space-3`) with `<span>{label}</span>` and `span.__description` (13/18, weight 400, `--fg-muted`; `RefText`) when the description is non-empty.
   Hover/focus on an option with `preview` shows `PreviewCard`.
   Last row `Other` (unless `noOther`): same button `aria-label="Other"` with `<kbd>{options.length+1}</kbd>`, text `Other`, and `ThreadKbd change`.
4. `PreviewCard` (portal on body, `role=region aria-label="Preview"`): `pre.__preview` fixed, `z-index: var(--z-menu)`, `max-width: calc(100vw - space-8)`, `pointer-events:none`, `--shadow-overlay`, padding `space-3`, border 1px default, radius md, bg `--bg-sunken`, mono 13/18, `white-space:pre`, `overflow:auto`, `max-height: calc(var(--space-6)*10)` = 240px.
   Placed beside the panel (right if room, else left, else narrowed with wrap if >= 160px free, else over the row's end), level with its row (top, or bottom-anchored when the row is below mid-window); gap 8px (`GAP`), drawn once hidden to measure.
5. If `Other` is on: `Input aria-label="Other" autoFocus` (value in `words[front]`).
6. `beneath` node (held slots) if given.
7. `div.armada-session-thread__answers[role=group][aria-label="Answers"]`: last question -> `Button sm primary` `Answer` + yes kbd (`disabled` unless every question has a choice and not sending; `autoFocus` when a `pick` was preset); other questions -> `Button sm secondary` `Next` (disabled until the question has a choice); `Button sm ghost` `Skip` + no kbd (only if `canSkip`); and flat with N>1: step nodes `div.__steps[role=group][aria-label="Questions"]` (flex, gap space-1, `margin-inline-start:auto`): `button.__step` 24x24 (`--space-6`), border 1px default, radius md, mono 2xs; shows the number, or `Check` 12px stroke 3 when answered (`data-done`: border+bg hue, text `--bg-sunken`); `aria-current="step"` on the front (border hue, extra 1px hue ring); `aria-label`/tooltip `<name>` or `<name>: <chosen>`; `span.__link` (8x1px, default border colour; hue when done) between nodes.

State: `held = {front, picked[][], using[], words[]}` kept under `question:<keep>` (`useKept`) until answered or skipped; a deck with no `keep` keeps nothing.
Single choice: picking sets the choice, clears Other, advances unless last.
Multi-select: toggles, never advances.
`Other` single: selects Other, clears picks; multi: toggles.
`chosenFor` = picks + typed Other text if on and non-blank.
`answer()` -> `onAnswer("allow_once", [{question, chosen}...])` (a returned promise disables Answer until settled, then keeps released); `skip()` -> forget, `onAnswer("refuse")`.
Electron's `onAnswer` -> `draft.answer(session.id, answer, answers, folder, kind)` after `operatorRead?.answered()`; a lapsed answer -> `draft.send(...)` of text `Answering your earlier question "<q>": <a, b>` per line (`lapsedAnswerText`, `packages/screens/src/sessions-wire.ts:1017`) and hides the deck when it went; lapsed skip -> hides the deck for that `lapsed.id` (`answeredLapse`).

Keys on the deck (the four ⌥ keys: B6): `1`–`9` on the card group (`onKeyDown` at `QuestionDeck.tsx:305-324`): ignored when the card is folded or the event target is an `<input>`; with a preset `pick`, Enter takes the selection (answers if last and complete) unless the target is a radio; digit `n` picks option `n` if `n <= options.length`, `Other` if `n === options.length+1` and not `noOther`, else ignored; calls `preventDefault`.
There is **no modifier check**, so `⌘1` also picks.
Enter/Space press the focused option (native button).
No arrow-key roving: options are separate tab stops.
The panel moves focus to the first `[role=radio]`/`checkbox`/input/button of the content when a question opens, so digits work at once (B7).

Spacious (`spacious-thread.css:1557-1640`): `__body gap: space-4`; `__ask font-size: calc(var(--spacious-body)+1px)` = 16px, leading `--spacious-leading` 24px, weight 600, `text-wrap: pretty`; `__answers gap: space-2`; each `__answer`: `align-items:flex-start; gap: space-3; padding: space-3 space-4; border:0; radius lg; bg var(--spacious-card); box-shadow var(--spacious-card-edge); font-size --spacious-body; line-height 22px`; hover shadow `inset 0 0 0 1px color-mix(hue 60%, transparent)`; `[data-on]` bg `color-mix(hue 14%, transparent)`, shadow `inset 0 0 0 1px hue`; `> kbd` `min-width:1ch; font-size --spacious-meta; line-height 22px`; `__option` column, `align-items:flex-start; row-gap:2px`; `__description` 13px/20px.
Panel overrides for a flat deck (`NeedsYouPanel.css:377-409`): the detail becomes a flex column that fills; `__answers` is the scroll box: `flex: 1 1 auto; min-height: calc(var(--space-6)*5)` = 120px; `overflow-y:auto; padding: space-1; margin: -space-1` (room for the focus ring); everything else in the body is `flex:none`.

### B4.5 Texts (verbatim)
`Waiting on you`, `Unanswered`, `Permission`, `Permission to read|write|run`, `Allow once`, `Deny`, `Reject once`, `Always applies to`, `reads of`, `writes to`, `commands on`, `Folder for always`, `Calls like`, `Only in <folder>`, `Question` (default header), `Options`, `Answered`, `Questions`, `Other`, `Next`, `Answer`, `Skip`, `<n> of <N>` (pips title), `Preview`.
`askLabel`: permission with no questions -> `Permission` or `Permission to <reachesTo>`; else the first question's `header`, or `Question` if blank.
`askText`: the first question's text, or the first line of the command, or the lapsed first question's text.

### B4.6 START picks (held Job) and Open group
- `useStartPicks(session, repositories, compact)` (`StartPicks.tsx`) returns `{lead, begin}` for a held Job (`session.start` defined and a draft source), else undefined; `sessions.tsx:1891` mounts `lead` in the composer (`sv-start-strip`: flex wrap, `gap: space-2`, `padding: space-1 space-2`, border 1px default no bottom, radius `md md 0 0`, bg `color-mix(accent 12%, --bg-sunken)`, `session-view.css:1382+`).
  It is a strip of `Pick` controls, **not a card in the thread** and not a question deck; digits do not drive it.
  - Optional status glyphs first: `LoaderCircle` 12 tooltip `Starting` (when `start.queued`), `TextCursor` 12 tooltip `Reading the request` (while the proposer reads).
  - `Pick Glyph=Workflow label="Workflow"` with `Select aria-label="Workflow"`: options `Workflow` placeholder (when none is chosen), `None`, then the repository's workflows (those for this manifest and not switched off by `workflows.off`); disabled while reading; value = proposed workflow else `feature` else first.
    A `proposed` mark when Fleet proposed it.
  - `Pick Glyph=GitMerge` (only when `start.asksDelivery` and Workflow is not `None`) with `Select aria-label="Delivery"`: options `Select delivery` placeholder, `Auto land` (title `Open a pull request, watch ci, merge it.`), `Open PR only` (`Stop at the open pull request.`), `Keep local` (`Stop at the commit on the job's branch. Nothing is pushed.`).
    Label shown = the chosen option's detail, else `Delivery`.
    A mark glyph `Quote` (tooltip `Named in your request`) or `Repeat` (`Your usual for <plural of workflow name> in <repository>` or `Your usual`) when Fleet took it from his words or usual.
  - `begin(text)` sends `draft.begin(id, landing | undefined, text?)`; choosing a workflow calls `draft.work.choose(id, value|null)`.
- `OpenGroup` (`OpenGroup.tsx`) is the Story column's Open work group; a right press offers one `DropdownMenu` item `Hide Open group` (EyeOff) which sets setting `bridge.openGroup=false`.
  Not a thread card.

### B4.7 Fleet data
Wire: `ThreadHead.asked: ThreadAsk{command ("argument whole where detail was cut, else detail, else tool name"), call: CallInFlight{call, manifest_id, asked_at, tool, detail, truncated, length, whole, rule, offers[CallAnswer], holding_for_seconds, questions[AskedQuestion{question, header, multi_select, options[{label, description, preview}]}], reaches, reaches_to, kind}}`; `ThreadHead.lapsed: Lapsed{id, questions}` ("questions nobody answered in time, that were skipped or kept, oldest first; absent for an ended Job").
Electron derives from `SessionRecord.hosted.asked`/`terminal.asked` (`askOf`, `sessions-wire.ts:978-1011`: `command = whole ?? (detail==="" ? tool : detail)`) and computes `lapsed` itself from rows (`lapsedOf` `:1032-1062`: walks `ask` rows newest to oldest; a row is kept if `state==="kept"` or (nothing he said after it and state is `unanswered` or `refused`); a message of his that starts `Answering your earlier question` ends the walk; any other message of his sets "said"; a question text asked again later is left to the later ask; id = newest ask row id).
Offer labels/means (B4.2) are TS only; the wire has ids.
`holding_for_seconds`, `rule`, `truncated`, `length` are not drawn.

---

## B5 Open call, lapsed, ThreadCard, notices

### B5.1 Call chip (the only "call" the Job view draws)
`CallChip word={callWord(session)} scope={centre}` mounted after the thread (`sessions.tsx:3285`).
`callWord` = `"Plan"` iff the plan exists, is not approved and `planAsked(session)`; else undefined (a question, a permission and Needs-you items are in the panel).
`DESK/session-view/CallChip.tsx`.

- Visible when `word` is set and the thread tail (`.armada-session-thread__tail`) has its top below `scroller.bottom - 24px` (scroll and resize listeners). Hidden while the tail is in view or absent.
- Control: `div.sv-callchip` (relative; `z-index:2; flex:none; height:0`, `session-view.css:1331-1336`) > `button.sv-callchip__press` (`absolute; inset-block-end: calc(var(--space-8) + var(--space-2))` = 40px; `inset-inline-start:50%; translate: -50% 0`; inline-flex, `gap: space-1`; `height: --h-control-sm` 24; `padding: 0 space-3`; border 1px default; radius md; bg raised; color `--status-awaiting-review`; 13px; hover bg `color-mix(fg-default 6%, raised)`; focus-visible `outline:none; box-shadow: inset 2px 0 0 --accent`).
  Content: `<span>Plan</span>` + `ArrowDown` 12px (aria-hidden).
- Press: scroll the tail into view (`block:"end"`, smooth unless reduced motion) and focus its first `button, textarea, input` (`preventScroll`).

### B5.2 Open call
`ThreadHead.open_call{text, since}` "the tool call the running turn is in: set while a turn runs and the thread's last row is a tool row".
Electron: `openCallOf(rows, record)` (`sessions-wire.ts:1525-1537`): `record.turn_since` defined and the last row is a tool row -> `{text: last.text, since: last.at}`.
Read in `StoryColumn.tsx:207` (`session.openCall`) as the agent's line in the story column; **not drawn in the thread's card area** (part A's calls fold carries the working mark).
`CallCard.tsx` is Cockpit-only.

### B5.3 ThreadCard (`inline`)
`ThreadCard{key, at?, node}` (`ST/SessionThread.tsx:716-720`): drawn as `li.armada-session-thread__card` (`list-style:none`) after the item chosen by `afterWhich(items, card)` (`:736-744`): the newest item whose last-row time is <= `card.at`; the first item (index 0) if it came before all; the last item if `at` is missing or not a date.
Cards are placed after each item in the `flatMap`, after any approved-plan record `li` for that item (`:4160-4172`).
Electron passes `inlineCards = [...useModCards(session.id), ...useBrowserReminders(session.id)]` (`DESK/sessions.tsx:2748`): mod cards (`DESK/session-view/mod-cards.tsx`) and the Job browser's reminder card (`DESK/job-browser/JobBrowserPane.tsx`, per the `session-thread-rows.ts` comment).
A wire `browser_reminder` row is excluded from the rows (`session-thread-rows.ts:227-236`).
Their contents are not covered here.

### B5.4 Notice / Teach (menus, not cards)
`SessionNotice` (list `ST/SessionThread.tsx:155-197`, 43 names) and `NoticeChoice = "job"|"repo"|"hour"` (:198); `TeachPick{kind: "words"|"command"|"notice"|"check", text, at?, ids?}` (:201-206).
A Fleet notice row/turn (`FleetTurn.tsx`) opens a context menu on right press, ContextMenu key or Shift+F10 (`noticeEntries`, `ST/FleetTurn.tsx:106-145`): `Teach` (GraduationCap; first on every notice; a separator when both exist) then, only when the notice can be muted, a label `Disable <notice name>` (`NOTICE_NAMES`, e.g. `slot behind notice`, `checks failed`, `conflicts`, `merged`, `delivery question`) with items `For job`, `For repo`, `For 1 hour`.
`Teach` -> `onTeach({kind:"notice", text, at, ids:[notice]})`; a choice -> `onMuteNotice(notice, choice)` (`muteNoticeFor`, `DESK/sessions.tsx`).
The menu is a `DropdownMenu triggerLabel="Fleet notice"` at the pointer (or at the row's bottom centre for the key); closing returns focus to the row button.
The folded row itself (part A's Fleet-turn row): `li.armada-fleet-turn[role=region][aria-label="Fleet turn"][data-open]` > `button.__row[aria-expanded][aria-controls]` with `ChevronRight` 12, `BellRing` 12 (tooltip `Fleet notice`), `Rows3` 12 (tooltip `Batched`, when `notice.batch`), label (`batch` or `summaryOf(text)`: first sentence up to ". " or line break, closing full stop dropped; tooltip = full notice text), `Check` 12 (tooltip `Handled`).

### B5.5 Lapsed
See B4.3C and B4.7: drawn as a deck in the Needs you panel (label from `askLabel`, row text `askText`), not in the thread. The lapsed deck keeps no state (no `keep`).

---

## B6 thread-keys: the four keys

### B6.1 Registry (`crates/core-model/domain/actions.toml:1170-1217`, generated `packages/components/src/generated/actions.ts`)

| Act id | Chord | Verb | Tier / scope | Destructive / confirms | Meaning |
|---|---|---|---|---|---|
| `thread_yes` | `⌥↵` | Yes | Global / `thread` | no / **yes** | Approve a plan or a draft, Allow once, Answer a question, Send to job, End job, Retry a hand off |
| `thread_change` | `⌥E` | Change | Global / `thread` | no / no | Opens the card's own box: draft Edit, question Other, plan in full |
| `thread_send_back` | `⌥R` | Send back | Global / `thread` | no / no | Draft Note box; plan Not yet |
| `thread_no` | `⌥X` | No | Global / `thread` | no / **yes** | Deny, Skip, Keep going, Withdraw/Dismiss |

Contract: `docs/contracts/design-system/keyboard-and-command-palette.md:352-378` (*Keys on a Job's thread cards*): `⌥` chords because the caret usually sits in the message box; `1`–`9` still pick a question's option; `⌘↵` commits an open edit/note box; in the review sheet (`⇧⌘Y`) `⌘↵` approves the selected item, `⌥R` sends back with a note, `↑↓` move, Esc returns focus.
`⌥↵`/`⌥X` always confirm in a dialog (owner, 28 Oct 2026); `⌥E`/`⌥R` open the card's own box, no dialog.
Palette hints (`DESK/palette.ts:115-118`): `⌥↵ on a card in a Job's thread` etc.

Keys read by place: `keyOf` uses `event.code` (`KeyE`, `KeyR`, `KeyX`, `Digit*`) when Alt is held, because Option changes the typed character (`packages/components/src/keymap.ts:429-444`); a `Dead` key with Alt and a letter/digit code is still read (`:450`).
`⌘` also answers to Ctrl unless the chord asks for ⌃ (`:471-476`).
Chords can be rebound by the user (`isPressed`, `keyFor` read the live binding).

### B6.2 Mechanism (`ST/thread-keys.tsx`)
- `ThreadKeys` (provider) wraps both the thread and the composer (the Job's `centre`, `sessions.tsx:3051`; keyed by session id).
  It owns `cards` (Map id -> {kind,title,acts}), `places` (id -> element), `focused`, `confirming`.
- Order = cards drawn top to bottom by `compareDocumentPosition`, only elements still connected.
  **Target** = the card containing focus (a `focusin` listener on document; ignores focus inside `[role=dialog],[role=alertdialog]`; focus anywhere else clears it), else the newest (last) card.
- One `keydown` listener on `window` (not capture) while no confirm is open: ignored if `defaultPrevented` or any `[role=dialog],[role=alertdialog]` exists; finds the act by `isPressed(THREAD_ACT_ID[act])`.
  - `yes`/`no`: collect all cards having that act, in drawn order; `preventDefault`; open the confirm dialog with `picked` = the target's index in that list (else the last); remember `document.activeElement` to return focus.
    No card with the act -> key ignored (not prevented).
  - `change`/`sendBack`: run the target's act directly (`preventDefault`); if the target lacks it, nothing happens.
- `ThreadConfirm` (portal on `document.body`, `Dialog open tone="neutral" confirmLabel=<verb> initialFocus="confirm"`; title `` `${picked.run.verb} which?` ``, e.g. `Approve which?`, `Answer which?`, `Allow once which?`, `Withdraw which?`): body `RadioGroup label="Waiting cards"` with one `Radio name="thread-confirm"` per card: `span.armada-thread-confirm__row` (inline-flex, `align-items:center; gap: space-2; min-width:0`) = `Kbd aria-hidden` digit (1–9; a blank `span` for the 10th+) + `span.armada-thread-confirm__title` (`overflow-wrap:anywhere`) `{kind} · {title}`.
  A window capture-phase listener while open: `⌥↵` (`thread_yes`) or plain Enter on a radio confirms; plain `1`–`9` picks that row (ignored beyond the count); both `preventDefault` + `stopPropagation`.
  Confirm runs the picked card's act; Esc / Cancel closes; focus returns to the element that had it.
  If every listed card disappears, the dialog closes.
- `useThreadCard(card|undefined)` registers while `card` is given and drops on unmount/undefined; returns `{target, bind}`.
  `bind` = `{ref, tabIndex:-1, "data-thread-target": ""}` (the data attribute only on the target); with no provider or no card it returns just `ref`.
  The plan card spreads `review` after `bind`, so a reviewable plan has `tabIndex:0`.
- `ThreadKbd act on`: renders `Kbd aria-hidden` containing `keyFor(thread_*)` (`⌥↵`, `⌥E`, `⌥R`, `⌥X`) only when `on` (the card is the target); nothing otherwise or if the key is unbound.
  Chips sit inside the buttons, after the label (Button gap `--space-2`).
- `useThreadYesWaits()` -> true when some registered card has a `yes`; `SessionComposer.tsx:976` then ignores `⌥↵` in the message box instead of sending (`if (yesWaits && isPressed("thread_yes", event)) return;`), outside the `open` autocomplete state.
- Ring: `[data-thread-target] {outline: var(--focus-ring); outline-offset: var(--focus-ring-offset)}` (`thread-keys.css:1-5`): 1px accent, 2px offset.
  No ring (outline none) on `.armada-question-deck[data-thread-target]` (`SessionThread.css:759-761`) and `.armada-session-done__acts[data-thread-target]`.

### B6.3 Which cards register (id, kind, acts)
| Card | id | kind | yes | change | sendBack | no | Registered when |
|---|---|---|---|---|---|---|---|
| Plan | `plan` | `Plan` | `Approve` | `Suggest` (if `onReview`) | `Not yet` (if `onSetAside`) | none | `!approved && !aside && !going` |
| Draft | `draft:<rowId>` | `Draft` / `Rule` / `Workflow` | `Approve` / `Send` / `Approve with fixes` / `Send with fixes` | `Edit` | `Note` | `Withdraw` / `Dismiss` | `open` (unsettled, can answer); per-mode rules B3.4 |
| Question | `question:<keep or first question>` | `Question` | `Answer` (last question, all answered, not sending) | `Other` (unless `noOther`) | none | `Skip` (if `canSkip`) | always while the deck exists; title `<header>: <question>` or the question |
| Permission | `permission:<command>` | `Permission` / `Permission to <x>` | `Allow once` | none | none | `Deny` / refuse label | `asked` with no questions |
| Done (not mine) | `done` | `Done` | `End job` | none | none | `Keep going` | `DoneBody` (SessionEndBand) |
| Handoff (not mine) | `NeedsYouPanel/HandoffBand.tsx:30` | | `Retry` | | | | |

Sketch/document/pin cards, the Needs-you ledger rows and Operator rows do not register.

### B6.4 Hint chips drawn (labels and positions)
`Approve ⌥↵`, `Suggest ⌥E` and `Not yet ⌥R` on the plan card (answers row; the folded head's Approve); draft `Approve ⌥↵`, `Edit ⌥E`, `Note ⌥R`, `Withdraw ⌥X` (read), `Approve with fixes ⌥↵` (edit); question `Answer ⌥↵`, `Skip ⌥X`, and an `Other` row ending in `⌥E` after its label; permission `▶ Allow once` icon button followed by `⌥↵`, `Deny ⌥X` or the refuse button `⌥X`.
A chip is a `Kbd` (20px tall, mono 12px, `--fg-muted` on `--bg-raised`, 1px border + 2px bottom lip, radius 3px, `padding 0 4px`), `aria-hidden`.

---

## B7 Needs you panel (dock): where the question sits

### B7.1 Where it lives
`NeedsYouPanel` `packages/components/src/compositions/NeedsYouPanel/NeedsYouPanel.tsx:152-715`, CSS `NeedsYouPanel.css:1-426`; mounted `DESK/sessions.tsx:3306-3334` inside `div.sv-bands` inside `div.sv-dock` (`:3286-3300`), keyed by `session.id`.
Sibling bands (ToolsBand, HandoffBand, SessionEndBand, WaitingBand) are not covered here.
`sv-dock` props: `data-needs` (when `waits || endShown || failedShown || toolsBand || waitingBand`), `data-open={panelOpen || endOpen}`, `data-end`, `data-wait-open`, `data-starting`.

### B7.2 Props passed (`sessions.tsx:3306-3334`)
`rows={needs}` (`cardRowsOf(session, waitingRows, reviewWaiting)`: waiting ledger rows minus those the review sheet built, and never `plan`/`sketch`/`document` sources, `JobReviewSheet.tsx:292-308`), `open={panelOpen}`, `onOpen(open) -> setPanel({id: session.id, open})`, `question` (when defined: `{id, text: askText(...), label: askLabel(...), children: <AskCards flat ...>}`; `id = asked ? (session.asked.call ?? asked.command) : lapsed.id`), `cards={notOurs}` (Fleet's own "watch ended, not ours" cards: `CornerUpRight` glyph, `NotOursCard` with `Fix it` / `Ignore`; not detailed), `review={reviewWaiting.map({key, Glyph, said})}`, `onReview` -> `setReviewSheetFor(session.id)` or, for a lone page, `openOwedDocument(session, lonePage)`.

### B7.3 Open/closed rules
| Name | Rule | Where |
|---|---|---|
| `waits` | `needs.length>0 \|\| question!==undefined \|\| notOurs.length>0 \|\| reviewWaiting.length>0` | `sessions.tsx:2642` |
| `listed` | `needs.some(row => row.restart===undefined \|\| row.onDismiss!==undefined) \|\| question!==undefined \|\| notOurs.length>0` (a restart is the band's own key unless dismissible; a band with nothing in it is never open) | `:2654-2658` |
| `panelOpen` | `(panel?.id === session.id ? panel.open : holdsDelivery(session)) && waits && listed` | `:2659-2660` |
| `holdsDelivery` | `owesOf(session).some(one => one.source === "delivery")` (the delivery question opens the card before he touches it; after he opens/closes, his choice holds) | `packages/screens/src/session-state.ts:18-19` |
| inside the panel `open` | `wanted && things.length>0` | `NeedsYouPanel.tsx:~262` |
| arrival | a question's `id` not yet seen opens the panel (`host(true)`) once per mount (re-opens each time he comes into the Session since the panel is keyed by session); focus moves into it only if `focusIsFree()` (nothing focused, or focus is not in the composer/field, or the focused field is empty), else `quiet` (no focus change) | `:262-312` |
| empty | when `things` becomes empty after having rows, calls `host(false)`; renders `null` when empty and no restarts and no review | `:313-330, 383` |
| Esc | `useEscapeFold`: while open, Esc anywhere (no dialog up, not already handled) folds it; focus goes back to the trigger only if focus was inside | `useEscapeFold.ts` |
| Scrim | `useScrimFold`: a press on the dimmed thread folds it | `NeedsYouPanel.tsx:347` |
| panel keys | `onKeyDown` on the section: Esc (when open and no menu): inside a detail goes Back, in the list folds; Backspace goes Back when not typing (`input, textarea, select, [contenteditable]`); ArrowDown/Up move between list buttons (`LIST_KEYS`) | `:388-420` |
| global | `needs_you` (`⇧⌘Y`): if there are review items -> focus the trigger and `onReview()`; else if not empty -> toggle open | `:362-380` |
| one thing | when there is exactly one thing, it is a question that opens here, and there is no review sheet row: it opens straight to its detail (`only`), and Back folds the panel | `:279-283, 396-398` |
| dimming | the thread's stage is dimmed over its own box while open: `.armada-session-thread__stage::after` bg `color-mix(--bg-base 50%, transparent)`, `opacity` 0 -> 1 over `--duration-sheet`, `pointer-events:auto` while open | `NeedsYouPanel.css:~235-255` |

### B7.4 Layout
`section.armada-needs-panel[aria-label="Needs you"][data-open][data-keeps-escape]` (`--hue: var(--step-waiting)`; `position:absolute; inset-block-end:100%; inset-inline:0; z-index:2; flex column; pointer-events:none`, children `auto`; data-kind variants `end` -> `--step-advanced`, `failed` -> `--status-completed-failed`, `waiting` -> `--status-handed-in` belong to the sibling bands).
`.sv-bands > .armada-needs-panel {position: static}`; `.sv-bands` is `position:absolute; inset-block-end:100%; inset-inline:0; z-index:2; flex column`; `.sv-dock[data-needs]` has `margin-block-start: calc(var(--h-badge) + var(--space-3))` and a composer with squared top corners.

| Region | Detail |
|---|---|
| `div.__band` | flex, `height: var(--h-badge)` (20px; spacious: `calc(var(--space-6)+var(--space-1))` = 28px, radius `--spacious-radius`), `border-radius: md md 0 0` (open: `lg lg 0 0`; spacious both: 8px), bg `var(--hue)`, color `--fg-inverse`, `overflow:hidden` (`NeedsYouPanel.css:~95-110`, spacious :526-529) |
| `button.__trigger` | `type=button`, `aria-label="Needs you"`, `aria-expanded={open}` (undefined when empty), `aria-haspopup="dialog"` when empty with a review sheet, `aria-disabled` when empty and no sheet; flex 1, gap space-2, padding `0 space-3`, transparent, inherit font. Children: `BellRing` 12px; `span.__label` text `Needs you` (12/16, weight 600, caps tracking, uppercase; spacious: sans, 13px, weight 500, no transform, no tracking); `span.__glyphs` (inline-flex gap space-1) one `span.armada-session-mark[role=img][aria-label=<tip>]` 12px glyph with tooltip per thing (questions first, then actions) then one per review item (`said`); `ChevronUp` 12px class `__fold` (`margin-inline-start:auto`; rotates 180deg when open over `--duration-sheet`), absent when empty. Press: `onReview` if empty with a sheet, else toggle |
| restart key | `RestartKey` `button.__restart` (flex none, bg `--accent`, 13px weight 500, hover `--accent-hover`) for each banded restart row, beside the trigger (not detailed) |
| `div.__reveal[data-open]` | grid `grid-template-rows: 0fr -> 1fr` over `--duration-sheet`, `visibility:hidden -> visible` (delayed on close) |
| `div.__shell` > `div.__body[data-open]` | flex column, `gap: space-3`, `box-sizing:border-box`, `max-height: calc(var(--thread-h, 55vh) * 0.95 - var(--h-badge))`, `padding: space-2 space-3` (spacious: `space-3 space-4 space-4`), `border: 1px solid var(--hue)` left/right only, bg `--bg-raised`, `overflow:auto`, `opacity 0 -> 1` (120ms; open 180ms after a 90ms delay) |
| list (`div.__list`) | flex column, `gap: space-4`, `padding-block-end: space-3`; animation `armada-needs-panel-in` 180ms (opacity from 0, translate 8px) |
| detail (`div.__detail`) | flex column, `align-items:flex-start`, `gap: space-2`; head `div.__head` (flex, `justify-content:space-between`, `align-self:stretch`): `Button ghost sm iconOnly aria-label="Back"` with `ChevronLeft` 16 and tooltip `Back` (shortcut `Esc`); thumbs on the right if the item is a misread; content `div.__content` (`align-self:stretch; min-width:0`) |

### B7.5 Row kinds in the list (order top to bottom)
Section groups (`section.armada-session-ledger__group[aria-label=<label>]` > `h3.armada-session-ledger__eyebrow` + rows):
1. **Questions** (`asks(row)`): the question (`MessageCircleQuestion` mark, tooltip = `label`, text = `askText`; key `question:<id>`), then `cards` (Fleet's own: their `Glyph`/`said`, key `card:<id>`), then ledger rows that are not walk/run/steps/approve_pr and not a banded restart (their mark from `waitingMark`: Bot with `row.operator` text; `ScanText` `Read from the reply`; `MessageCircleQuestion` `Question` (has options); `Link` `Opens the linked Job` (held child start); `Reply` `Reply`; else by act `AppWindow` `Opens the walk`, `MessageSquare` `Goes to the question`, `GitPullRequestArrow` `Opens the pull request`, `Terminal` `Copies the command`, `SquarePen` `Goes to the message`, `ListChecks` `Steps`, none `BellRing` `Waiting on you`).
   Each `li.armada-session-ledger__row[aria-label=<name>]` has `button.__open[aria-label="Open <name>"][data-thing]` = mark + `span.__text`; pressing opens the detail (`opensHere`: no custom `press`, no `operator`, and (no ledger row, or a row with `options` or `onReply`)), else it runs the row's own `press`/`onOpen`.
   Extras at the row end: thumbs (`Thumbs`, only when `misread.yes` exists: ghost sm iconOnly 12px glyph in 24px squares `ThumbsUp` aria/tooltip `A question`, `ThumbsDown` `Not a question`); `StartAct` (Start for a held child); Operator ways (`Button sm` per option, first `secondary`, rest `ghost`, aria `<label>: <name>`, fixed columns 96px and 56px).
   Right press / ContextMenu / Shift+F10 on a misread row: `DropdownMenu triggerLabel="Question"` items `A question` (ThumbsUp, only if yes), `Not a question` (ThumbsDown).
2. **To review** (when `review` has items and `onReview`): `section[aria-label="To review"]` with one row `aria-label="Review"`, `button.__open[aria-label="Review"][aria-haspopup="dialog"][data-thing="review"]`: glyphs per item + text joined from the `said`s with `, `.
3. **Walks** (`act==="walk"`), 4. **Commands** (`act` `run` or `steps`), 5. **PRs** (`act==="approve_pr"`): `LedgerRows kind="waiting"`: a `run` row with a command draws a `RunnableBlock` (`pre.armada-prose__block > code`, Run via runner); a row with `options` draws a flat `QuestionDeck` (`canSkip={false}`, `noOther={no onOther}`, `beneath` = `HeldSlots`, `detail` = `StepLines`; its answer calls the option's `onPick()` or `onOther(text)`); other rows are the opener button with mark and text (PR rows add `PullRequestStateMark` and a checks mark), a reply row opens a reply box (`aria-expanded`).

`ACTIONS` order and labels: `Walks`, `Commands`, `PRs` (`NeedsYouPanel.tsx:49-63`).

### B7.6 How the question (QuestionDeck) sits in it
- Folded: the band shows `MessageCircleQuestion` (tooltip = header or `Question`/`Permission…`) among the glyphs; nothing of the deck is in the tree (`visibility:hidden`).
- Open with one thing: the detail shows directly: `Back` head, then `div.__content` holding `AskCards flat` (for a question: `Card.__ask` collapsed to bare content, `QuestionDeck[data-flat]`, band hidden).
  Open with several things: the list; pressing the question row opens its detail.
- On opening a detail, focus goes (next frame) to the first of `input, textarea, [role='radio'], [role='checkbox'], button` inside `.armada-needs-panel__content`, else to `Back` (`NeedsYouPanel.tsx:~330-343`), so `1`–`9` and the four `⌥` keys work at once.
- The panel's body replaces the composer's top edge; the thread behind is dimmed; a press on the dimmed thread folds it.

### B7.7 Strings (verbatim)
`Needs you`, `Questions`, `To review`, `Review`, `Walks`, `Commands`, `PRs`, `Back`, `Esc`, `A question`, `Not a question`, `Question`, `Open <name>`, `<label>: <name>` (ways), `Waiting on you` and the act tooltips in B7.5.

QUESTION: `SessionLedger.css` row classes (`armada-session-ledger__row/__open/__text/__eyebrow/__group/__held`), `RestartKey`/`RestartRow`, `StartAct`, `HeldSlots`, `StepLines`, `RunnableBlock`, `NotOursCard` and `LedgerPreview` (hover card over a row) were not read; the panel list rows' pixel sizes are not recorded.
QUESTION: exact `useScrimFold` behaviour (what counts as a press on the thread, and the CSS `z-index`/pointer rules of the dimming `::after` beyond `inset: -space-2 -space-3 0`) was not read in full.
QUESTION: `Dialog` primitive (used by `ThreadConfirm`: size, buttons `Cancel`/confirm label, backdrop) and `Radio`/`RadioGroup` pixels were not read.
QUESTION: Electron derives almost all of B1–B5 in TypeScript from `SessionRecord` + rows and does not call `get_job_thread` (`JobThread` appears only in `packages/protocol/src/job-thread.ts` and a story).
The wire has no `plan`, no `revisions`, no offer wording, no `going`, no judged rows, and `ThreadItem::Document` is per revision; the native app must either take these from `SessionRecord` or add them to the wire.

## 4. The composer (message box)

| | |
|---|---|
| Component | `packages/components/src/compositions/SessionComposer/SessionComposer.tsx` (`SessionComposer` `:529-1489`, `Row` `:1568`, `Pick` `:1625`, `TalkGuide` `:1503`); `QuickBar.tsx`, `TunePicker.tsx`, `AttachmentTray.tsx`, `InlineTag.tsx`, `message-dom.ts`, `large-paste.ts`, `talk-guide.ts` |
| CSS | `SessionComposer/SessionComposer.css`, `QuickBar.css`; spacious overrides in `apps/desktop/src/renderer/src/spacious-thread.css` (`.armada-session-composer` `:543-570`, `.armada-quick` `:968-1020`, `.armada-tune*` `:1100-1240`, `.armada-tray*` `:1452-1545`, `__gap` `:1544`); dock `session-view/session-view.css:1365-1425` |
| Mounted | `sessions.tsx:3385-3529` inside `.sv-dock`, only when `session.dead === undefined`; a second, plain box `SessionBox` (`sessions.tsx:3650`) serves call cards |
| Draft | kept per Job under `keep` key `session:<session.id>` (`draftKey`): `{text, tags}` persisted to `localStorage["armada:kept:session:<id>"]` (files are never stored; their bytes do not survive a reload), plus an in-memory map for panel switches (`packages/components/src/keep.ts:1-60`). Cleared by a send (`forget()`); a refused send (`onSend` resolves `false`) puts the draft back unless something was typed since (`SessionComposer.tsx:1036-1053`). Seeding helpers: `seedDraft`, `quoteIntoDraft` (appends a quote after a blank line, quote ends in a blank line), `tagIntoDraft` (chip at the end; no duplicates) (`:326-384`). |

### 4.1 Layout

Spacious take is the shipped one; base values in brackets.

```
.sv-dock  (width calc(100% - 2*gutter16), max-width --spacious-outer, centred)
 +- .armada-session-composer  (form; flex column; border 1px; radius --spacious-radius 8px [--radius-md 6px];
 |    background --bg-sunken; border-color --border-subtle (focus: unchanged in spacious; base --edge-focus = accent))
 |   [ .armada-quick  QuickBar   ]  padding --space-2 --space-2 0 --space-4  (when quick lines are on)
 |   [ .armada-session-composer__field ]
 |        [ .armada-tray  attachments ]   padding --space-3 --space-4 0; gap --space-2; scrolls x   (spacious + something attached)
 |        .armada-textarea.armada-session-composer__editor  role=textbox, contentEditable
 |             padding --space-3 --space-4; font 15px/24px (--spacious-body/leading); min-height calc(--leading-sm*2 + --space-2*2)=44px;
 |             max-height = half of .armada-session-frame__body's height, then scrolls y; white-space pre-wrap; overflow-wrap anywhere
 |   [ held chips row ] (compact && held && !spacious only)
 |   .armada-session-composer__bar   flex, nowrap, align center, gap --space-2, padding --space-1 --space-2 --space-2 (spacious),
 |                                    border-top 1px --border-subtle
 |       [Permission mode select] [TunePicker: model + effort] [Attach] [Sketch] [gap flex:1] [context ring] [Talk] [Send]
 +- (typeahead list hangs above, `.armada-mention`, or below when there is no room)
```

Region rules:

| Region | Rule (file:line) |
|---|---|
| Box | `.armada-session-composer`: `position:relative; display:flex; flex-direction:column; border:var(--border-width) solid var(--border-default); border-radius:var(--radius-md); background:var(--bg-sunken)`; `container-type:inline-size` (`SessionComposer.css:1-13, ~205`). Spacious: `border-color:--border-subtle; border-radius:--spacious-radius` and focus-within keeps `--border-subtle` (`spacious-thread.css:543-557`); `z-index:2` (`:923`). A Needs-you / tools / end band on its edge squares its top corners (`session-view.css:1385-1388`). |
| Editor | `.armada-session-composer__editor`: `min-height: calc(var(--leading-sm)*2 + var(--space-2)*2)`, `overflow-y:auto`, `overflow-wrap:anywhere`, `white-space:pre-wrap` (`:~233-239`); the field has no own border (`.armada-session-composer__field .armada-textarea {border:none;background:transparent;box-shadow:none;outline:none}`); spacious `padding: --space-3 --space-4; font-size:--spacious-body (15px); line-height:--spacious-leading (24px)` (`spacious-thread.css:560-565`). Max height set in JS = `bound.clientHeight / 2` where bound is `.armada-session-frame__body` (`SessionComposer.tsx:768-792`). |
| Quote in the box | base: `blockquote`/`[data-block=quote]`: `padding --space-2 --space-3; border-left 2px solid --accent; radius 0 --radius-sm --radius-sm 0; background --accent-faint; colour --fg-default`; spacious: `margin --space-1 0 --space-3; padding 0 0 0 --space-3; border-left 2px solid --border-subtle; no background/radius; colour --fg-muted; font --text-sm; line-height 1.55` (`spacious-thread.css:1435-1448`). List: `padding-left --space-4; list-style:disc; li::marker colour --accent`. |
| Bar | `.armada-session-composer__bar`: `display:flex; align-items:center; gap:--space-2; padding:--space-1 --space-2; border-top:1px solid --border-subtle; flex-wrap:nowrap`; every direct button `flex:none`; its selects and ghost buttons are borderless/transparent, `--fg-muted`, hover `--bg-hover`/`--fg-default` (`:~86-130, ~183-189`). Container query `max-width:400px`: gap `--space-1`, the context ring's percent text hidden, selects' padding `0 --space-6 0 --space-1` |
| Select (Permission mode) | `Select` height `--h-control-sm` (24px), `--text-xs`/`--leading-xs`, `flex:1 1 auto; min-width:0; text-overflow:ellipsis`; its `__pick` wrapper `flex:0 1 auto; min-width: calc(--space-12 * 1.5)` = 72px (0 under the container query) |
| TunePicker (spacious) | trigger `.armada-tune__trigger`: inline-flex, gap `--space-2`, `height:--h-control`, `padding:0 --space-2 0 --space-3`, no border/ground, `--text-sm`, `--fg-muted`, hover `--bg-hover`/`--fg-default`, radius `--radius-md`; model name nowrap; effort meter = 10px-high flex of N bars each 3px wide, 2px gap, radius 1px, bar heights `(i+1)/N*100%`, off colour `color-mix(--fg-default 16%, transparent)`, on `--accent`; `ChevronDown` 14px. Panel `.armada-popover__panel` width 280px, `padding --space-1`, opens upward at the block-start (margin-bottom `--space-2`), radius `--spacious-radius`, border `--border-subtle`; model rows `height:--h-control-lg (32px)`, `padding 0 --space-3`, `--text-sm`, `--fg-default`, hover `--bg-hover`, selected shows `Check` 14px `--accent`; below a hairline the Effort block `padding --space-3 --space-3 --space-2; gap --space-1`, label `Effort` `--text-xs --fg-subtle`, then a `Slider` |
| Attach / Sketch buttons | ghost sm icon buttons (spacious 28px via `.armada-button` rules), glyph 16px stroke 2 |
| Gap | `.armada-session-composer__gap` `flex:1` (`spacious-thread.css:1544`) takes the room attached chips take in the base look |
| Context ring | `.armada-session-composer__figure`: `display:flex; flex:none; margin-inline-end:--space-3` (`:~474`); content = `ContextFigure` (see 4.9) |
| Talk | `.armada-session-composer__talkwrap` relative inline-flex; button ghost sm |
| Send | `Button type=submit variant="primary" size="sm"`; `Send` glyph 12px stroke 2 + word |
| Typeahead list | `.armada-mention.armada-session-composer__offer`: `top:auto; bottom:100%; margin-bottom:--space-1` (below: `top:100%; margin-top:--space-1`, `data-below`). Row: `.armada-mention__row` (+ `--active`); name `font-family:--font-mono; --text-xs; flex:none`; says: `margin-inline-start:--space-3; --fg-muted; --text-xs; flex:1; ellipsised` (sans for `#` rows); standing: `--fg-subtle`, `--text-xs`, after the title; group heading `p.armada-session-composer__group`: `padding --space-1 --space-3; --fg-subtle; --text-2xs; weight --weight-heading; tracking --tracking-caps; UPPERCASE`; glyph 12px `--fg-muted`, `margin-inline-end --space-2`. List flips below when `at.top - bound.top < min(items,6)*28+16` and there is more room below (`SessionComposer.tsx:717-733`). |
| Inline tag (chip in the text) | `.armada-inline-tag`: inline-flex, gap `--space-1`, `padding-inline --space-1`, `1px solid --border-default`, radius `--radius-md`, background `--bg-raised`, `--text-xs/--leading-xs`, nowrap, `user-select:all`; glyph 12px `--fg-muted`; atomic (one backspace deletes it whole; `contentEditable=false` with `data-tag-kind/id/title/url`) |
| Attachment tile (tray) | `.armada-tray__tile` 160 x 64 px, flex column, `justify-content:space-between`, `padding --space-2 --space-3`, radius `--radius-md`, background `--spacious-card`, edge shadow `--spacious-card-edge`; name: 2-line clamp, `--text-xs`, line-height 1.35, break-all; kind label (`PNG`... extension uppercased, or `Sketch`): `--text-2xs`, weight medium, tracking 0.04em, `--fg-subtle`; a picture tile is `width:auto; padding:0; overflow:hidden` with the `<img>` `height:100%; min-width:48px; max-width:128px; object-fit:cover` (tooltip = file name, `alt` = name); remove button 22px round at `top/right --space-1`, `Trash2` 12px, `--bg-overlay`, opacity 0 until the tile is hovered or has focus within |
| Attachment chip (base look) | `AttachmentChip`: inline-flex, gap `--space-2`, `min-height --h-control-sm`, `padding --space-1 --space-2`, `1px --border-default`, radius `--radius-sm`, `--bg-sunken`, `--text-xs`; preview image `--space-4` square radius `--radius-sm`; sketch chip shows `Sketch` as `from`; remove `Trash2` 12px, `--space-4` square |

### 4.2 Controls

| Control | Label / tooltip / aria | Glyph | When shown | Does |
|---|---|---|---|---|
| Editor | `role="textbox"`, `aria-label="Message"` (prop `label`, default `Message`), `aria-multiline`, `aria-disabled`, `aria-controls=<list id>` while the list is open | | always (contentEditable unless `disabled`) | types; see 4.4 |
| Permission mode | Tooltip = the mode's sentence; `Select aria-label="Permission mode"`; options `Ask`, `Auto`, `Accept edits`, `Plan` | `Shield` (12px, only in compact) | `onMode` given and not `modeHidden`/`bare`; disabled when `modeLocked` (terminal Job) with tooltip `<sentence> The terminal holds it.` | `draft.tune(id, {model, effort, mode})` |
| Mode sentences | `Ask`: `Asks before it edits or runs anything`; `Auto`: `Runs on its own, and asks before a push or anything destructive`; `Accept edits`: `Edits files without asking, and asks before running a command`; `Plan`: `Reads and plans, and changes nothing` | | | |
| Model + effort | trigger aria-label `Model <name>, effort <level or auto>`, tooltip `The model and effort the next turn runs on`, `aria-haspopup=dialog`; popover label `Model and effort`; models `radiogroup aria-label="Model"` rows (first row `Default` + the followed model's name, `aria-label="Default, <name>"`), slider `aria-label="Effort"` steps `Auto` + the efforts (lowest first); meter `role=img aria-label="Effort: auto"` or `Effort: <level>` | `ChevronDown`, `Check` | spacious && !`textOnly` && !`attachOnly` (base look: two selects `Model` (tooltip `The model the next turn runs on`; empty option text `Model`/`Auto`/`<default> · default`) and `Effort` (tooltip `The effort the next turn takes`; empty option `Effort`/`Auto`), glyphs `Cpu`, `Zap` in compact) | `onTune({model, effort})` -> `draft.tune`; picking a value the proposer chose clears its mark (`draft.work.unpick`) |
| Proposer mark | tooltip + aria-label `Model picked by the proposer` / `Effort picked by the proposer` / `Model and effort picked by the proposer` (base look: `Picked by the proposer`) | `Lightbulb` 12px `--accent` | `session.work.proposal.picked` includes model/effort and not yet changed | none |
| Attach | tooltip `Attach a file or a picture`, aria-label `Attach file`; hidden `<input type=file multiple aria-label="Files to attach">` | `Paperclip` 16px | !`textOnly` | opens the picker |
| Sketch | tooltip `Draw a sketch to send with the message`, aria-label `Draw sketch` | `PencilRuler` 16px | !`textOnly` && !`attachOnly` | `onDraw` -> `setPadOpen(true)` opens `SketchSheet`; the sketch returns as a chip (`drawn`) |
| Attached strip | `role="group" aria-label="Attached"` (chips) / same on the tray | | something attached | each has Remove: aria-label `Remove <name>` |
| Context ring (`beforeSend`) | button `aria-label="Context <n>%"`; see 4.9 | | `session.context !== undefined` | |
| Talk | tooltip `Talk` + `Kbd ⌥Space`; aria-label `Talk`; `aria-pressed=listening`; `aria-keyshortcuts="Alt+Space"` | `Mic` 16px | `talk` prop present (the app has a voice engine) | press starts, press again stops; disabled when `disabled` |
| Talk guide | card `role="status" aria-label="Talk could not hear"`, head `MicOff` 16px on a `--h-control` square (`--bg-sunken`, radius `--radius-md`), title, numbered steps, foot: primary sm button + `Kbd Esc` | `MicOff` | a hearing ended early with a reason, until Esc / press outside / window refocus / Talk pressed again | button opens the Settings pane |
| Send | label `Send` (a held Job: `Start`); `slim` draws the glyph only with aria-label and tooltip | `Send` 12px | always; disabled when `disabled` or (`text.trim()==="" && nothing attached && !sendEmpty`) | `send()` |
| Quick bar | see 4.7 | | | |

Talk guide texts (`talk-guide.ts`): `microphone_denied`: title `The microphone is off for Armada Voice`, steps `Open **Privacy & Security**, then **Microphone**.` / `Turn on **Armada Voice**.` / `Press Talk again.`, button `Open Microphone settings`; `speech_denied`: `Speech recognition is off for Armada Voice`, `Open **Privacy & Security**, then **Speech Recognition**.` / `Turn on **Armada Voice**.` / `Press Talk again.`, button `Open Speech Recognition settings`; `microphone_unavailable`: `No microphone is available`; any other: `Talk stopped`.
`**x**` is bold.
Card: absolute, `right:0; bottom: calc(100% + --space-2)`, `z-index:--z-menu (40)`, `min-width:--sidebar-default (200px)`, `max-width:--palette-width (560px)`, `padding --space-3`, `gap --space-3`, radius `--radius-lg`, `--bg-overlay`, `1px --border-default`, `--shadow-overlay`, `--text-sm/--leading-sm`; step numbers on `--bg-sunken` discs `--space-4` wide, mono `--text-xs`.

### 4.3 States

| State | Drawn | Condition |
|---|---|---|
| Idle / empty | editor 44px min, Send disabled | |
| Focus | base: border `--edge-focus` (accent); spacious: border unchanged (the caret says where) | `:focus-within` |
| Typeahead open | list above/below the box; Up/Down move, Enter/Tab choose, Esc closes | `items.length > 0` from `tokenAt` |
| Listening (Talk) | button background `--bg-hover`, colour `--fg-default`, glyph opacity 1 <-> 0.45 over `--duration-pulse` (still under reduced motion) | `listening` |
| Disabled | `aria-disabled`, not editable, Send/Talk off | `disabled` |
| Text-only / attach-only / bare / compact / slim | `textOnly`: no model, effort, attach, sketch; pasted/dropped files ignored; `attachOnly`: files and tags but no model/effort/sketch; `bare`: field + Send on one row, no border-top on the bar; `compact` (narrow window, `useNarrow()`): selects become glyph + value; `slim`: Send is its glyph | props; the Job's box passes `modeLocked/modeHidden` for terminal Jobs, `compact={narrow}` |
| Held Job | Send reads `Start`, works on an empty box when Fleet gave no first message (`sendEmpty`); strip `.sv-start-strip` stands on the top edge | `starting !== undefined` (`useStartPicks`, `sessions.tsx:1891`) |
| Paused Job | send asks `Job is paused` first | 1.4 |
| Waiting on you | quick bar not drawn (only `talk` stays) | `waits` (`sessions.tsx:2645-2652`) |
| Ended / quiet Job | no composer is drawn at all | `session.dead !== undefined` (`:3385`) |
| `⌥↵` with a waiting card | Enter handling skips; the thread card takes `⌥↵` | `useThreadYesWaits()` (`thread-keys.tsx`) |

### 4.4 Typing rules (`SessionComposer.tsx`)

- Enter sends; Shift+Enter inserts a line break (`execCommand("insertLineBreak")`); with `enterBreaks` (phone) Enter breaks the line and only Send sends; Cmd/Ctrl+Enter also sends (the Enter test ignores meta/ctrl) (`:964-987`).
  IME composition never sends.
  In a quote or list line, Enter/Backspace belong to the block (`enterInBlock`, `backspaceInBlock`, `message-dom.ts:473, 498`).
- Markdown drawn as typed: a run of `> ` lines is one quote block, a run of `- ` or `* ` lines one list, markers left out of the element and given back in the text (`readDom`); bare web addresses, paths and `[text](https://…)` links are decorated; Cmd-click on a link opens it via `ProseLinks` (`onClick` `:910`).
- Sent text (`sentText`, `message-dom.ts:167`): each tag chip is written `@<title>` at its place; a blank line is added where a quote/list ends in a line of words. Trimmed.
- Triggers (`tokenAt`, `:450-484`): `/` at the very start (`^\/(\S*)$`) lists skills and commands; `@` at the start of a word lists Jobs (`taggable`: other live Jobs with titles) then up to 5 files; `#` at the start of a word with digits or key letters lists issues and pull requests.
  - `/`: commands whose names start with the query, then those that contain it; row = `/name` + the command's `says`.
    A command typed whole (`items[current].id === query`) sends on Enter; Enter/Tab otherwise completes `/name ` and puts the caret after it.
    List aria-label `Skills and commands`.
    Commands for a hosted Job = `hostedCommands(draft.commands)`; for a terminal Job `session.commands`.
    `/mcp` (typed alone, no files) opens the Job's Loadout on MCP instead of sending (`opensMcp`, `sessions.tsx:3501`).
  - `@`: groups `Jobs` (a Job row = `SquareTerminal` + the Job's title; chosen -> an atomic chip) then `Files` (`File` glyph, name then folder; chosen -> plain `@path ` text).
    List aria-label `Tag a Job or a file`.
    File search runs `searchFiles(query, manifestId)` 150 ms (`LOOKUP_MS`) after the last keystroke; the previous answer stays until the next arrives.
  - `#`: up to 6 rows (`NUMBERED_ROWS`): open issues / PRs, numbers newest first, keys in source order; a number not among the open ones is looked up after 150 ms and put first, with a `standing` word (`Merged`, `Closed`); row = kind glyph (`GitPullRequest` / `Ticket`, tooltip `Pull request` / `Issue`) + `#<n>` or key + title; chosen -> atomic chip; tags already in the message are not offered again.
    List aria-label `Tag an issue or pull request`.
    Tag kinds and glyphs (`InlineTag.tsx:32`): session `SquareTerminal`, pull_request `GitPullRequest`, branch `GitBranch`, issue `Ticket`, report `Bug`, terminal `Terminal`; kind words `Job`, `Pull request`, `Branch`, `Issue`, `Reported bug`, `Terminal`.
    In the thread a tag with an open target is a link `a.armada-inline-tag--link` (accent colour, tooltip `<Kind> · <title>`).
  - Tags can also arrive from outside (`tags` prop: Terminal tab "Send to agent", session-pick tags, `pendingTags` from Fleet/draft) and are appended as chips at the end of the draft, then `onTags([])` (`:806-827`).
- Paste: files/images on the clipboard are attached (`Pasted file` if unnamed); a text paste over 8000 estimated tokens (4 chars each, `large-paste.ts`) becomes a file `pasted-snippet-<n>.txt` attached; otherwise plain text is inserted (rich text is stripped).
  Drop of files anywhere on the box attaches them (not when `disabled`/`textOnly`).
  Pictures get an object-URL preview.
- A quote from the thread (Reply / fork-from-selection) is put in the draft with `quoteIntoDraft` and the caret goes to the blank line under it; `focusFor` focuses the box with the caret at the end (`sessions.tsx:3132-3137`, `focusFor` `:3465`).
  A Job opened empty takes the caret as its page shows (`sessions.tsx:~1760`).
- `! command` (text starting with `!`, no files): runs in the Job's docked terminal, not sent to the agent (`bangOf`, `sessions.tsx:3506-3510`).

### 4.5 Sending and queued messages

`onSend` (`sessions.tsx:3498-3528`): held Job -> `starting.begin(text)`; `/mcp` -> Loadout; `!cmd` -> terminal; else (paused check) -> `draft.send(session.id, {text, files, sketches: drawn, tags})`, then `drawn` is cleared, and if the send fails the sketches return.
**The composer has no queue control or queue key.** A message sent while a turn runs is the agent's next input ("Send stays on" when `working`, prop doc `:172`) and the thread draws it as a message row with a `Queued` mark until delivered (thread section A: `Queued` `SessionThread.tsx:1603`, `Delivery` `:1574`).
`⇧⌘↵` (`queue_session` "Queue for later") belongs to the Dispatch panel (`dispatch-panel/DispatchPanel.tsx:148`); `⌘↵` (`send_message`) is registered as "type in a message box first" (`palette.ts:106`) and the Job's box sends on plain Enter or ⌘/⌃ Enter.
A failed message gets a `Retry` on its own row (`draft.retry`, thread section A), so the box has nothing to take back.

### 4.6 Voice (Talk)

Press Talk (or hold `⌥Space` in the box): the box takes focus with the caret at the end, `talk.listen(onWord, onEnd)` is called; each word heard is appended to the draft with one space between (`joined`); nothing is sent.
Held key: keydown `Alt`+`Space` (not repeat) starts; keyup of Space or Alt stops.
Pressed: a second press stops.
A hearing that ends by itself leaves the button; if it carries a reason the guide card shows.
Engines: `Apple on-device` or `Whisper` (select `Voice` in Aa; Whisper needs its model installed: `Install` -> progress -> `Installed`).
Words come from `voice.listen(engine, ...)` supplied by the host (`thread-look-here.tsx:185`).
The key `⌥Space` is read by `event.code === "Space"` with `altKey` and no meta/ctrl (`:935-945`).

### 4.7 Quick bar (`QuickBar.tsx`)

`<div class="armada-quick" role="group" aria-label="Quick replies">` on the box's top edge inside its border (`padding --space-2 --space-2 0 --space-4` spacious; gap `--space-1`).
Hidden entirely when `quickShownIn(look)` is false (`look.quickOff` or this Job in `quickOffFor`) or `waits`.
Content: `.armada-quick__chips` (flex, gap `--space-1`, scrolls x, no scrollbar) then `Save` then (if `onHide`) `Hide`.

| Piece | Detail |
|---|---|
| Chip | `<button class="armada-quick__chip">`: height `--h-control`, `padding 0 --space-2`, `1px solid --border-subtle`, radius `--radius-md`, background `--bg-raised`, `--text-xs/--leading-xs`, nowrap; hover border `--border-default` + `--bg-hover`; active translateY(1px); focus ring. Parts: kind glyph 12px (`MessageCircleQuestion` tooltip `Asked`; `Repeat` `Learnt`; `MessageSquareQuote` `Saved`; `ArrowUpToLine` tooltip `Pinned` after the text), text, `Kbd ⌘1`..`⌘9` for the first nine (`aria-keyshortcuts="Meta+n"`) |
| Order | the agent's question's answers (`asked`, from Fleet's `SessionQuick` read, only while the last message asks and still the message they were written for), a vertical rule (`--border-width` wide, `--border-default`) if any `asked`, then his lines, pinned first (`pinnedFirst`), then learnt lines (when `look.learn`) minus those he deleted/reworded |
| Press | chip click sends the line as-is (`sendLine`: `{text, files:[], tags:[]}`) and the box keeps its draft; `⇧` + click or `⇧⌘n` puts it after the draft (`joined`) with the caret at the end; `⌘1`..`⌘9` while the box has focus send line n (`preventDefault` + `stopPropagation`, so tab keys do not move) (`SessionComposer.tsx:917-933`) |
| Chip menu | right-click / `Shift+F10` / ContextMenu key: for a saved/learnt line (has `id`) `Edit`, `Pin`/`Unpin`, rule, `Delete` (danger); for an `asked` answer `Save` (disabled with hint `Already a quick reply` when kept). `Edit` turns the chip into a field (aria-label `Edit <text>`), Enter saves, Esc cancels, leaving saves, empty/unchanged = cancel |
| Save | ghost sm icon button, tooltip `Save what is typed as a quick reply`, aria-label `Save as quick reply`, glyph `MessageSquareQuote` (spacious) / `ListPlus` (base) 16px, disabled when the box is empty; adds `{id:"saved:<ts>", text, kind:"saved"}` to `thread.look.quick` (skips duplicates) |
| Hide | menu button tooltip/trigger `Hide quick replies`, `EyeOff`; entries `Hide for this Job` / `Hide for all Jobs`; also right-click on empty bar space. Spacious: Save/Hide buttons `--h-control` square, transparent, `--fg-subtle`, 15px glyph, disabled opacity 0.4 |

### 4.8 Keys that act on the composer

| Key | Scope | Does | Bound |
|---|---|---|---|
| `⌥M` | anywhere in the Job view | focus the box, caret at end | `SessionWorkspace.tsx:207` (`message_job`, `actions.toml`) |
| `Enter` / `⌘Enter` / `Ctrl+Enter` | in the box, no list open | send | `SessionComposer.tsx:978-986` |
| `Shift+Enter` | box | line break | `:964-973` |
| `⌥↵` | box, when a thread card waits | answered by the card (`thread_yes`) instead | `:976`, `thread-keys.tsx` |
| `⌥Space` (hold) | box | Talk while held | `:935-945`, release `:1131-1141` |
| `⌘1`..`⌘9` / `⇧⌘1`..`⇧⌘9` | box focused, quick bar on | send / drop quick line n | `:917-933` |
| `ArrowUp/Down`, `Enter`/`Tab`, `Esc` | list open | move, choose, close | `:989-1007` |
| Backspace on a chip | box | deletes the chip whole (atomic) | `message-dom.ts:498` |
| `Cmd+click` link | box | open link | `:910` |
| `Esc` / `Enter` / `Shift+F10` | Talk guide / chip field / chip | close / save / cancel / menu | `:1515-1532`, `QuickBar.tsx` |
| `↑↓` `Tab` `←→` `Enter` `Esc` | model/effort panel | move models, reach slider, slide, put away | `TunePicker.tsx` |

### 4.9 Context ring (`ContextFigure`, in the bar via `beforeSend`)

`SessionFrame/ContextFigure.tsx`.
Drawn only if `fill.window > 0` (`session.context = {tokens, window}`; `contextBreakdown` from the record).
Ring 16 x 16 viewBox: track `r=5.25 --border-default`, arc `stroke var(--tone, --fg-muted)` from 12 o'clock to `tokens/window`, tick at the autocompact threshold (`stroke color-mix(--fg-subtle 60%)`, 1px) when the breakdown holds `autocompactThreshold`; ring `--space-4` square; then the percent text in mono `--text-xs --fg-muted` (hidden under the 400px container).
Tone by `usageLevel(percent)`: normal below 80, warning (amber) from 80, danger (red) from 95. Variants: no breakdown + no compact -> `role=img aria-label="Context <n>%"` with tooltip `Context · <tokens> of <window> tokens`; no breakdown + compact (Cockpit) -> button with popover `Context`: `<tokens> of <window> tokens` + `Compact`; with breakdown -> `Inspect` card (hover/focus opens, Enter keeps open) titled `Context window`, kind `Context` (`Layers`), where `<tokens> / <window>`, state `<n>% used`, a proportional bar of categories, fact rows per category, `Memory files` (top 3 by tokens), `MCP tools` (top 3 servers), and `Compact` (secondary sm button; tooltip `Compact the Job's context`, or the off reason `The agent has not listed compact` / `The terminal has not listed compact` / mute reason).
`tokensSaid`: `53840 -> 53.8k`, `5000 -> 5.0k`, `142000 -> 142k`, `1000000 -> 1M`.

### 4.10 Props the Job passes (Fleet data)

| Prop | From | Note |
|---|---|---|
| `working` | `session.turn.state==="working"` | |
| `mode`, `modeLocked`, `modeHidden` | `session.mode ?? (terminal ? "ask" : "auto")`; terminal Job locks, hides until its mod reported a mode | `SessionRecord.hosted.mode` |
| `model`, `effort`, `models`, `efforts`, `defaultModel`, `resolvedModel` | `session.model/effort`, `draft.models/efforts/defaultModel`, `session.resolvedModel` | `SessionRecord.model/effort`; the label `resolvedModel` turns `opus` into e.g. `Opus 5.5` via `modelSaid` (`SessionFrame/model-said.ts`) when the family matches |
| `commands` | hosted: `draft.commands`; terminal: `session.commands` | |
| `taggable` | other sessions with `dead===undefined` and a title | |
| `searchFiles` | `searchFiles(query, manifestId)` (Fleet file search) | |
| `numbered` | `useNumbered(manifestId)`: the repository's open issues and PRs, `find(number)` | |
| `tags` / `onTags` | `session.pendingTags` / `draft.setTags` | |
| `quick` / `talk` | `useThreadLookHere` (`thread-look-here.tsx:308`): saved look setting `thread.look`, Fleet quick lines (`SessionQuick`: `asked`, `learnt`) | |
| `drawn` | local `useKept("session:<id>:drawn")` sketches | |
| `sendLabel`, `sendEmpty` | `starting` (held Job: `Start`, `session.start.message === undefined`) | |

QUESTION: The box's accessible name is `Message` and `⌥M` finds it by `.sv-dock [role="textbox"][aria-label="Message"]`.
No other composer (Operator Talk, Request tab, call-card `SessionBox`) is described here.

---

## 5. Fleet data and what Electron works out in TypeScript

### 5.1 The wire that exists

| Wire | Where | Shape |
|---|---|---|
| `get_job_thread` -> `JobThread` | `crates/ipc/src/job_thread.rs:50` | `{session_id, head: ThreadHead, items: Vec<ThreadItem>, older: Option<String>}`; request `JobThreadRead{session_id, before?, limit?}` `:36` |
| `job_thread.tail` -> `JobThreadTail` | `:72` | `{session_id, from?, items, head}` |
| `JobThreadChanged` | `:86` | `{session_id, from}` |
| `ThreadHead` | `:94` | `asked: Option<ThreadAsk{command, call: CallInFlight}>`, `open_call: Option<OpenCall{text, since}>`, `lapsed: Option<Lapsed{id, questions}>`, `fleet_acts: Vec<FleetAct{id, at, state: over/under_way/coming, kind: nudge/step/slot/restart/told, said, why?, until?}>`, `trail: ThreadTrail{started, bays: Vec<TrailBay{slot, from, until?}>, opened: Vec<TrailOpened{number, at}>}` |
| `ThreadItem` (tag `kind`) | `:195` | `Row{row}` (never tool/check/waiting ask/browser_reminder), `Calls{id,names,rows}`, `Subagent{id,at,subagent}`, `Checks{id,rows}`, `FleetTurn{id,notice,rows,handled}`, `Sketch(ThreadSketch)`, `File{id,at,path,name}`, `Document(ThreadDocument)`, `Change{id,at,what: workflow/model/effort,from,to}`, `NotStarted{id,at,cause,log}` |
| `SessionRow` | `hosted_sessions.rs:565` | `Message, Tool, Command, Compaction, Lease, Release, Window, Started, Linked, Waiter, WindowAsk, Draft, Check, Ran, Requested, Saved, Retro, Sent, Ask, Plan, PlanChanges, Delivery, BrowserReminder, Disputed` |
| `SessionRecord` | `sessions.rs` (`in_flight` at `:1443`) | the Job's record: title, state, turn, model, effort, mode, context, pending tags, work/proposal, start, hosted, dead, commands... `in_flight: Vec<InFlight>` is on the wire. |

**The shipped Electron renderer does not read `get_job_thread` or `job_thread.tail`.** Only the types exist in `packages/protocol/src/job-thread.ts` and `events.ts`.
`SessionThread` folds, groups, orders and decides everything from the raw `SessionRow` list plus the `SessionRecord` (thread sections A/B below).
The native app is the first consumer of the server-side folding; its output must match what `SessionThread` draws.

QUESTION: Does `ThreadItem` (server fold) match `SessionThread`'s client fold row for row (run breaks, Fleet-turn grouping, Check runs, last-agent-message-after-Fleet-turn, document placement)?
This inventory records the client fold as the spec and has not diffed the two.

### 5.2 Worked out in TypeScript, not on the wire

| What | Where | Rule |
|---|---|---|
| State mark (the head's glyph/word) | `packages/screens/src/session-state.ts:103` (`jobStateOf`) | derived from the record's turn, waits, dead, held, paused; table in 1.2 |
| In-flight (what ending a Job loses) | `end-job.tsx`, `InFlightDialog.tsx` read `lanes.job` through `inFlight()` | the wire's `in_flight` is not read |
| Which acts overflow into More | `job-panes/acts.tsx` (`LEAVES` order) | by measured width |
| Tab arrangement, ratios, split handles | `job-panes/store.ts`, `tree.ts` | localStorage per Job; not Fleet |
| Waiting / Needs-you bands | `sessions.tsx:3284-3381` | from `session.calls`, `waits`, handoff and end state |
| Quick lines order, pinned first, learnt minus deleted | `QuickBar.tsx`, `thread-look-here.tsx` | `thread.look` setting + Fleet `SessionQuick` |
| Model label (`opus` -> `Opus 5.5`) | `SessionFrame/model-said.ts` | by family |
| Context tone | `ContextFigure.tsx` `usageLevel` | 80 warning, 95 danger |
| Draft, tags, drawn sketches | `useKept` (localStorage) | per Job; files not stored |
| Typeahead lists (skills, Jobs, files, issues) | `SessionComposer.tsx` `tokenAt` | `searchFiles` and `numbered` are Fleet reads; ordering and truncation (5 files, 6 numbers) are TypeScript |
| Layout of the Job (paned) | `job-panes/store.ts` `defaultArrangement` 0.62 / 0.4 | not Fleet |

---
