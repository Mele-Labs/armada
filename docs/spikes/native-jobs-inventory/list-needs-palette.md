# Native Jobs inventory: list, Needs you, review sheet, palette, notifications

This is what the Electron and React Bridge draws today, so the native rebuild can match it.
It records what is, and proposes nothing.
The Job view (head, thread, composer) is another file; this one only says where these screens link into it.

Paths are relative to the repository root.
`DK` is `apps/desktop/src/renderer/src/cockpit/deck`.
`PC` is `packages/components/src`.
Density follows his `spacing.density` setting (Compact, Default, Roomy).
Default is `packages/tokens/src/spacing.css`; Compact and Roomy are `packages/tokens/src/spacing-sets.css` (`:root[data-spacing="compact"]` line 5, `"roomy"` line 26).
Every pixel value below is Default's, resolved from the token named beside it.
The native app implements all three sets; this table is the whole difference, and type sizes, hairlines, radii and the title row do not move.

| Token | Compact | Default | Roomy |
|---|---|---|---|
| `--space-1` | 3px | 4px | 5px |
| `--space-2` | 6px | 8px | 10px |
| `--space-3` | 9px | 12px | 15px |
| `--space-4` | 12px | 16px | 20px |
| `--space-6` | 18px | 24px | 30px |
| `--space-8` | 24px | 32px | 40px |
| `--space-12` | 36px | 48px | 60px |
| `--h-row`, `--h-row-header`, `--h-menu-item` | 24px | 28px | 32px |
| `--h-control` | 24px | 28px | 32px |
| `--h-control-sm` | 20px | 24px | 28px |
| `--pad-card` | 16px | 16px | 24px |

Constant in every set: `--h-badge` 20px, `--h-row-unit` 36px, `--h-kbd` 20px, `--border-width` 1px, `--radius-sm` 3px, `--radius-md` 6px, `--radius-lg` 8px (`spacing.css`).
Type ladder (`typography.css`): `--text-2xs` 12px / `--leading-2xs` 16px, `--text-xs` 13 / 18, `--text-sm` 14 / 20, `--text-base` 15 / 22, `--text-lg` 18 / 28.
Weights: `--weight-medium` 500, `--weight-heading` 600.
`--tracking-caps` 0.04em (`typography.css:32`).
Durations (`motion.css`): `--duration-press` 90ms, `--duration-fast` 120ms, `--duration-base` 180ms, `--duration-sheet` 220ms, `--duration-pulse` 1600ms, `--duration-decay` 45s.

## 0. Which list the "Jobs" tab draws

| Fact | Where |
|---|---|
| Two surfaces are named "Jobs" in `DECK_TABS`; both are hidden and both map to the Cockpit tab | `packages/shell/src/surfaces.ts` (`SURFACE`, `DECK_TABS`, `drawnTabs`), `apps/desktop/src/renderer/src/App.tsx:~266` (hidden list), `:~630` (`goTo`) |
| The Jobs list is the Cockpit's left column, `section[aria-label="Jobs"]` | `DK/Deck.tsx:~771` |
| The deck is a 3-column grid: Jobs list, centre stage, right bays. Left column width is `--deck-left`, default `calc(var(--space-12) * 6)` = 288px | `DK/deck.css:56`, `:65` |
| No calls panel exists. A call is the pulsing row, the Calls HUD, the Needs you card in the Job thread, the toast and the Dock badge | `DK/Deck.tsx`, `DK/ConsoleCalls.tsx` (the Calls HUD, out of scope: it goes with the Cockpit console after switch day), `NeedsYouPanel.tsx`, section 5 |

There is one Jobs list: the Cockpit's left column.
Both Jobs tabs in `DECK_TABS` are hidden, and there is no separate Jobs page.

## 1. Jobs list

**Data source.**
The row's lamp, status glyph state and group come from `SessionRecord.mark {state, lamp, group, once_read}` (PR #3374, `crates/ipc/src/job_mark.rs`).
For the native app that replaces `order.ts` `toneOf`, `lampOf` and `groupOf`, and `jobStateOf`.
The glyph for each state is Electron's `MARK` table in `packages/components/src/compositions/SessionFrame/SessionFrame.tsx:87` (table in 1.4).
`crates/ipc/src/job_mark.rs` is not in this checkout, so the tone/lamp/group rules below are today's Electron code and are the reference until that file lands.

### 1.1 Where it lives

| What | File |
|---|---|
| List, row render, Filter, queued row, Operator row, page-more row | `DK/Deck.tsx`: `List` 425-833, row 514-755, `Phase` 191, `OperatorRaisesRow` 207, `QueuedRow` 239, `MoreRows` 309, `Filter` 344, `linkedSaid` 391, `CALL_WORD` 401, `callLabel` 417 |
| Row component | `PC/compositions/ListCard/ListCard.tsx`, `ListCard.css` |
| Ordering, tone, lamp, groups | `DK/order.ts` (`toneOf` 36, `lampOf` 48, `groupOf`, `GROUPS`, `fixedOrderOf`, `movedIn`) |
| Scope, query, paging, calls | `DK/store.ts` (`SCOPES` 31, `LIST_PAGE` 57, `listOf` 647, `linkedUnder`, `incomingOf` 526, `callOwnerOf` 578, `openSubject` 277) |
| Repository bands | `DK/bands.ts` |
| Drag reorder | `DK/reorder.ts` |
| Row fields (View menu) | `DK/row-view.tsx` (`VIEW_FIELDS`, `DEFAULT_VIEW`, `COLUMNS`, `rowParts` 237) |
| Row menu | `DK/RowMenu.tsx` |
| Hover card | `DK/inspect-cards.tsx` (`SubjectCard`) |
| Subject model, state mark | `DK/ship.ts` (`sessionMark` 205, builder 395-610, calls 745-815) |
| State vocabulary | `packages/screens/src/session-state.ts` (`jobStateOf`), `PC/compositions/SessionFrame/SessionFrame.tsx` (`SessionState`, `MARK` 87, `bucketOf` 117) |
| CSS, in cascade order (later wins ties) | `PC/depth.css` (row ~358-430, selection ~385, group head ~291), `PC/compositions/ListCard/ListCard.css`, `DK/deck.css` (list 177-680, `.deck-mark` 2059, fold 2335-2395, arrival 2873), `DK/console-bridge.css` (flight strips ~975-1090) |
| Look is forced | `look.ts` sets `data-look="new"`, so the `depth.css` rules apply; `console-bridge.css` is unconditional (`:root:root`) and loads after `deck.css` |

### 1.2 Layout

```
.deck-left
  section.deck-list [aria-label="Jobs"]
    Standby (loading)
    header.deck-band:  "Jobs" | Filter glyph | PanelKey "list"
    ul.deck-list__rows  (scrolls)
      OperatorRaisesRow            (only when the Operator raised something)
      QueuedRow*                   (saved ideas, dashed)
      band? h3.deck-list__head     (repository plate, only when banded)
        li.deck-list__item > ListCard.deck-row   (one Job)
      MoreRows                     (spinner, loads next 50)
    RowMenu (context menu)
```

| Region | Size and type | CSS |
|---|---|---|
| Header band, base rule | `.deck-band`: min-height `--h-row-header` = `--deck-band` = `--h-control-sm` (24px); padding `0 var(--space-3)` (12px); gap `--space-3`; mono, `--text-xs` 13px / 18px, weight 600, `--tracking-caps` 0.04em, uppercase, colour `--fg-muted`; the Filter and the panel key sit at the right (`margin-left: auto`) | `DK/deck.css:12`, `:18`, `:182-198` |
| Header band, as drawn | On a Cockpit panel the band is a chip, not a header row: absolute, `inset-block-start` and `inset-inline-end` `--space-1` (4px), padding `0 --space-1`, gap `--space-1`, `min-height: 0`, radius `--glass-radius-control` (= `--radius-md`), fill `--deck-hole`, 1px ring `--glass-edge`, `opacity: 0` and `1` on panel `:hover` or `:focus-within` over `--duration-fast` | `DK/deck.css:208-243` |
| Band name "Jobs" | not drawn: a 1px clipped box that still names the band (`.deck-band__name`) | `DK/deck.css:229-238` |
| Chip lane | `.deck-list__rows` gets `margin-top: calc(var(--space-1) * 2 + var(--h-control-sm))` (32px) so the chip lies clear of rows; `console-bridge.css:986-989` sets `margin: 0` on `.deck-left > .deck-list > .deck-list__rows` and by selector weight wins, so the lane is 0 in the flight-strip look | `DK/deck.css:244-246`, `DK/console-bridge.css:986-989` |
| Rows list, base | `--deck-line` = `--h-control` (28px); `--row-min` = `--deck-line` (the "28px" in the comment above it is the base rule's, overridden by the strip below, so that comment is stale for this list); `--row-pad` = `0 var(--space-3)`; `--row-radius` = `--radius-md` (6px); padding `--space-1`; flex column; `overflow-y: auto` | `DK/deck.css:316-330` |
| Rows list, flight-strip skin (the spec) | margin 0; padding `--space-2` (8px); gap `--space-3` (12px) between plates | `DK/console-bridge.css:985-990` |
| Repository plate | `.deck-list__head`: min-height `--space-6` (24px), margin-bottom `--space-1`, padding `0 --space-2`, bottom hairline `--border-subtle`, no fill, no radius, no shadow, colour `--fg-muted`, size `--cb-code` = `calc(var(--text-2xs) * 0.8)` = 9.6px, tracking `0.18em` (literal); family, weight, case and line-height come from the base rule: `--font-mono`, 600, uppercase, `--leading-xs` 18px | `DK/console-bridge.css:1006-1017` over `DK/deck.css:390-404` |
| Group gap | `.deck-list__group` gap `--space-1`; a second plate has `margin-top: --space-3`; no divider between items | `DK/console-bridge.css:~996-1004` |
| Row (strip) | min-height `calc(var(--space-6) + var(--space-3))` = 36px; padding-inline `--space-3` start, `--space-2` end; 1px `--border-subtle` border; radius 0; fill `--bg-sunken`; hover, `data-hover` and `data-cursor` fill `--bg-hover`; selected: border `--accent`, fill `--accent-muted`; queued: dashed border, no lamp bar; focus ring `--focus-ring` offset `--focus-ring-offset` (2px) | `DK/console-bridge.css:1021-1090` |
| Lamp bar | `::before`, absolute, `z-index: 1`, inset `0 auto 0 0`, width `--space-1` (4px), colour `--lamp-ink`: idle `--border-strong`, running `--cb-cyan` (`--status-running`), failed `--status-completed-failed`, done `--status-completed-success`, needs `--cb-amber` (`--status-awaiting-review`) | `DK/console-bridge.css:1024-1074`, tokens `:13-14` |
| Child strip | min-height `calc(var(--space-6) + var(--space-2))` = 32px; the item's `::before` is a square rail with a 2px inline-start and block-end border in `--border-default`; `.deck-list__rail` is 2px inline-start in `--border-default` | `DK/console-bridge.css:1076-1092` |
| ListCard grid | columns `minmax(0,1fr) auto auto`, areas `"title at marks"`; with a tag or lines under it: `minmax(0,1fr) auto`, areas `"title marks" "tag at" "lines at"`; column-gap `--space-2` | `ListCard.css:1-30` |

Text on a row:

| Part | Size | Weight | Family | Tracking | Case |
|---|---|---|---|---|---|
| Title | `--text-row` = `--text-xs` 13px / `--leading-xs` 18px | `--weight-row` 500; `--weight-heading` when selected | `--font-sans` | `--tracking-row` 0 | as typed, one line with ellipsis (two lines with `data-wrap`) |
| Chips (issue, PR, workflow) | `--size-meta` = `--text-2xs` 12px | normal | `--font-mono`, tabular | normal | as typed |
| Time (`.armada-list-card__at`) | `--size-meta` 12px | normal | sans, tabular | normal | as typed; colour `--fg-subtle` |
| Lines under title | `--size-meta` 12px | normal | sans | normal | colour `--fg-muted` |
| Repository plate | `--cb-code` 9.6px / `--leading-xs` 18px | 600 | `--font-mono` | 0.18em | uppercase |

Marks column: each `.armada-list-card__mark` is `--space-4` (16px) wide; the wide status mark is `calc(var(--space-4) * 2 + var(--space-1))` = 36px; gap `--space-1`.
Marks hold the clash, tools, reported and earlier slots (empty places, buttons drawn over them from `Deck.tsx`), then the status.
Hard-coded or off-token: `0.18em` plate tracking; `1.6s` pulse duration; `0.06` and `0.14` pulse opacities; `--cb-*` tokens come from `console-bridge.css`, not `packages/tokens`.

### 1.3 Controls

| Control | Label / tooltip | Glyph | Action | Shown when |
|---|---|---|---|---|
| Filter trigger | tooltip and aria `Filter`, or `Filter: <said>` while narrowed (`said` = scope label, query cut to 14 chars with `…`, or `<scope> · <query cut to 8>`) | `FilterGlyph` | opens menu with search field and five scopes | always; glyph lit (`data-narrowed`) when scope is not Active or query is non-empty |
| Search field (in Filter) | aria `Search Jobs` | none | sets `deck.query`; Enter moves focus to first row | menu open |
| Scope items | `Active`, `Paused`, `Quiet`, `Ended`, `All`; the current one is checked | none | `setScope` | menu open |
| Row | aria `<title>` | none | opens the Job (`onPick`) | always |
| Hover card (Inspect) | `SubjectCard` beside the row; name = title | none | read only; shows `calling` text and tools text | hover or focus |
| Phase mark | tooltip and aria = phase word from `build.ts` (`wordOf`) | `glyphOf(build)` 14px | none | a build is known for the Job |
| State mark | tooltip = `said` from `jobStateOf` | see 1.4 | none | not paused |
| Paused mark | `Paused` | `PausedMark` (`CirclePause`) | none | Job is paused |
| Handoff mark | `HandoffMark` text | spinner-like mark | none | `one.handoff === "starting"` |
| Link glyph | `Linked Job` + `, started by <title> in <repo>` + `, handed from <group>` | `Link` 12px | none | `one.linked` set |
| Tie mark | `TieMark` (`operator/marks.tsx:93`) | per file | none | session rows |
| Earlier mark | `EarlierMark` | per file | `onEarlier(id)` opens the earlier Job; key `h` on the row does the same | `one.earlier` set |
| Tools mark | `ToolsMarkOf` (`tools-fixes.tsx:198`) | per file | opens the Job | agent came up without some tools |
| Reported mark | `ReportedMark` (`issues-tab/marks.tsx:132`) | per file | opens the report | Job reported a bug in Armada |
| Clash mark | `Conflicts with <label> · <paths>` (`clashSaid`) | `ClashMark` | none | View > Conflicts on (default on), kind not `pr` |
| Operator raises row | title and aria `Operator`; mark tooltip `The Operator raised something with no Job` | `Bot` 14px | opens Operator panel | `operator.state.raisesPulse === "row"` and raises non-empty |
| Queued row | title = first line of the idea; play press or Enter starts it; Remove deletes it | per file | `startQueued(id)` then opens the Job | saved ideas for the picked repository |
| Row menu | see below | per entry | per entry | right press, `ContextMenu` key, or Shift+F10 on a Job row |
| More rows | aria `Loading more Jobs`, `role="status"` | `LoaderCircle` 14px | `showMoreRows()` when it scrolls within 50% of the end | `drawn.length < subjects.length` |

Row menu (`DK/RowMenu.tsx`), in order:

| Entry | Label | Glyph | Shortcut | Condition |
|---|---|---|---|---|
| pause | `Pause` or `Resume` | `CirclePause` / `Play` | `PAUSE_KEY` (`session-tiles.ts`) | `draft.pause` exists |
| autopilot | `Auto Pilot` | `Plane` | none | always |
| name | `Name it for me` | `WandSparkles` | none | `draft.nameForMe` exists |
| teach | `Teach` | `GraduationCap` | none | Teach bridge exists |
| try | `Try it in your app` | none | none | always |
| close | `Close` (danger) | `X` | none | `draft.close` exists |
| separator, then the View group | `Active PRs`, `Issues`, `Now`, `Time`, `Conflicts`, `Status`, `Workflow` as toggles | none | none | always; on its own (no Job under the press) for a press on empty list |

View fields default (`row-view.tsx:49`): `pr` off, `issue` off, `now` off, `time` off, `conflicts` on, `status` on, `workflow` off.
Saved under setting `bridge.rowView`.

### 1.4 States and how each is drawn

Chain: `jobStateOf` (`session-state.ts`) gives a `SessionState`; `bucketOf` (`SessionFrame.tsx:117`) gives a bucket; `toneOf` (`order.ts:36`) gives `calm | ask | bad`; `lampOf` (`order.ts:48`) gives the lamp.

| SessionState | Glyph (lucide) | Bucket | Tooltip (`said`) |
|---|---|---|---|
| `working` | `CircleDot` (animated `WorkingMark`) | working | `Working`, or `Woken by <title>` |
| `running` | `SquareTerminal` | working | `Running: <what>` |
| `subagent` | `Split` | working | `Subagent working` |
| `waiting` | `Eye` | needs | `Waiting on you` |
| `done` | `Power` | needs | `Done` |
| `stuck` | `Clock` | needs | `Stuck` |
| `stopped_asks` | `OctagonAlert` | needs | `Waiting on you` |
| `failing` | `ShieldX` | needs | `Checks failed on #<n>` |
| `unstarted` | `X` | needs | `Not started` |
| `stopped` | `Square` | waiting | stopped text (`stoppedSaid`) |
| `held` | `Radar` | waiting | `Held open by <name>`, `Waiting on <name>`, `#<n> in the merge queue`, `#<n> to auto-merge` |
| `limited` | `BatteryLow` (`LimitMark`, not animated) | waiting | `limitHeldSaid(limitedUntil)` |
| `after` | `Link` | waiting | `startsWhenSaid(waits)` (`session-waits.ts`) |
| `draft` | `MessageSquareDashed` | draft | `Reading the request` |
| `blank` | `CircleDashed` | draft | `Blank: no slot, no branch` |
| `idle` | `Check` | idle | `Idle` |
| `replied` | `Reply` | idle | `Unread reply` |
| `quiet`, `ended` | `Unplug` | idle | `Not reachable`, `Ended` |
| `paused` | `CirclePause` | idle | `Paused` |

Other strings from `jobStateOf`: `Failed to start: <cause>`, `Held until <waitSaid>` (Operator hold, row shows `Clock`), `The Operator is reading its question` (row shows `Bot`, tone calm, no pulse).
Glyph meanings are in `packages/icons/conventions/session_state_borrowing.toml` and `packages/icons/icons/<name>.toml`.

| Row state | Drawn as | Code |
|---|---|---|
| Needs him | `needs` prop on ListCard, `data-lamp="needs"`, amber lamp `--cb-amber`, faint `--status-awaiting-review` tint at 0.06 opacity pulsing to 0.14 over 1.6s (`armada-list-card-needs`), reduced-motion stops it | `Deck.tsx:~530` `needs = session && toneOf(one)==="ask" && !here && !read`; `ListCard.css` |
| Calling | `data-calling` set when a call exists and is not a slot-creating wait, or when `needs` | `Deck.tsx:~584-590` |
| Running | `data-lamp="running"`, lamp `--cb-cyan` | `order.ts:48`, `console-bridge.css` |
| Failed | `data-lamp="failed"`, lamp `--status-completed-failed` | same |
| Done | `data-lamp="done"`, lamp `--status-completed-success` | same |
| Idle | `data-lamp="idle"`, lamp `--border-strong` | same |
| Selected (open) | `selected={here}`, `data-selected`, border `--accent`, fill `--accent-muted`, title weight `--weight-heading`; the row is quiet (no pulse) while open | `console-bridge.css`, `ListCard.css` |
| Cursor and hover | `data-cursor`, `data-hover`, fill `--bg-hover` | `Deck.tsx:~597` |
| Paused | `data-paused`, `PausedMark` replaces phase and state mark | `Deck.tsx:~480` |
| Reply | `data-reply`, state mark 12px instead of 16px | `Deck.tsx:~498` |
| Past (quiet, ended) | `data-past` | `ship.ts` |
| Linked child | `data-linked`, `--deck-depth` indent, rails (`.deck-list__rail`, `--deck-rail`), smaller strip | `Deck.tsx:~543`, `console-bridge.css` |
| Queued idea | `data-queued`, dashed border, no lamp bar | `console-bridge.css` |
| Arrived | `data-arrived` lift-in animation (`deck.css:2873`) | `Deck.tsx:~540` via `noteArrivals` |
| Dragging | `data-lifted`, `data-drop="before"|"after"` | `Deck.tsx:~546` |
| Pending | `data-pending`, default cursor, no hover | `ListCard.css` |
| Tone rules | red (`bad`) only if a Check fails now and idle under 15 minutes (`QUIET_AFTER = 15`) and not `held`; older failures draw calm | `order.ts:25-43` |

Call names (tooltip on a calling row, `callLabel`): `Question` (`ask_card`, `window_ask`), `Permission`, `Plan`, `Slot`, `Delivery`, `Walk`, `Done`, `Done?` (`task_done`), `Draft`, `Sketch`, `Document`, else `Waiting` or the capitalised kind; text is `<word>` or `<word>: <fact>`.

### 1.5 Ordering, grouping, filtering

| Rule | Detail | Code |
|---|---|---|
| Standing groups | `Needs you`, `Working`, `Stuck or held`, `Done`; a Job is `needs` if tone is not calm, else its phase (`needs` phase becomes `done`) | `order.ts:GROUPS`, `groupOf` |
| Within a group | loudest first: `bad` 0, `ask` 1, `calm` 2 | `order.ts:LOUD`, `groupedOf` |
| Displayed order | the order he placed (`fixedOrderOf`), then unplaced Jobs in standing order; a state change never moves a row | `order.ts:fixedOrderOf` |
| Drag reorder | live Jobs only; `movedIn(order, key, target, after)`; keys `⌥↑` `⌥↓` (registry `deck_reorder`) | `reorder.ts`, `store.ts:moveRow` |
| Scope `active` | live, not paused | `store.ts:listOf` |
| Scope `paused` | paused Jobs | same |
| Scope `quiet` / `ended` | past Jobs with that `past` value (older ones only while a search is on, `behind` ones never) | same |
| Scope `all` | every live then every past | same |
| Search (`query`) | `found` map narrows rows; the row shows what matched in place of its age | `find.ts`, `Deck.tsx:~514` |
| Checks filter | `deck.checks` keeps only Jobs with running Checks | `store.ts` |
| Open Job outside the scope | appended so the open Job always has a row | `store.ts:~700` |
| Repository bands | only on All with more than one repository served, or whenever a Mod Job is listed; label per repository, then `Mods`, then `Other` | `bands.ts:bandingOf`, `OTHER_BAND`, `MODS_BAND` |
| Nesting | a Job started by another listed Job stands right under it, any depth, within one band | `store.ts:linkedUnder` |
| Hidden | Mod Jobs that build a Custom tab; instrument Jobs unless asking with no bay | `store.ts:~658` |
| Paging | first `LIST_PAGE` = 50 rows, 50 more as the end nears | `store.ts:57`, `MoreRows` |
| Saved ideas | drawn at the head of their repository group (or of the list) above the Jobs | `Deck.tsx:~800-815` |

### 1.6 Keys

Source of truth: `crates/core-model/domain/actions.toml`, read by `isPressed`; deck handler `apps/desktop/src/renderer/src/cockpit/DeckHost.tsx:~305-600`; key tables `DK/keys-model.ts` (`PANELS`, `PANEL_SLOTS`, `PAGE_LETTERS`).

| Key | Registry id | Scope | Does |
|---|---|---|---|
| `j` `k` `↓` `↑` | `move_focus` | list | moves the cursor between rows |
| `↑` on first row | none (note on `deck_search`) | deck | opens Filter |
| `Enter` | `open_focused` | list | opens the focused Job |
| `o` | `open` | list | same act, named for the palette |
| `l` `b` `c` `q` `f` `a` `d` | `deck_panel` | deck | `l` list, `b` bays, `c` first row that needs him else first call |
| `/` | `deck_search` | deck | opens Filter with the search field focused |
| `⌥↑` `⌥↓` | `deck_reorder` | deck | moves the row up or down |
| `g` then `c` `j` `s` `y` | `deck_page` | deck | page chord: Cockpit, Jobs, Sessions, System |
| `t` | `deck_timeline` | deck | shows the timeline |
| `⌥⇧0` to `⌥⇧9` | `deck_tile` | deck | presses a flight control tile |
| `w` | `call_recall` | dashboard | brings a waiting call back |
| `?` | `key_sheet` | dashboard | shows the keys |
| `h` | not in registry ("waits on the look") | row | opens the earlier Job |
| `ContextMenu`, `Shift+F10` | none | row | opens the row menu |
| `Esc` | `close` | anywhere | closes menu or layer |

### 1.7 Strings

`Jobs`, `Filter`, `Filter: <said>`, `Search Jobs`, `Active`, `Paused`, `Quiet`, `Ended`, `All`, `Loading more Jobs`, `Operator`, `The Operator raised something with no Job`, `Needs you`, `Working`, `Stuck or held`, `Done`, `Other`, `Mods`, row menu strings in 1.3, state strings in 1.4, `Linked Job...` in 1.3.

### 1.8 Accessibility

| Element | Role and name |
|---|---|
| List section | `section`, aria-label `Jobs`, `tabIndex=-1` |
| Row | button from ListCard, aria-label = Job title |
| Phase and state marks | `role="img"`, aria-label = tooltip text |
| Filter trigger | aria-label = `Filter` text; search aria-label `Search Jobs` |
| More rows | `role="status"`, aria-label `Loading more Jobs` |
| Row menu | `DropdownMenu`, trigger label `Job` |

### 1.9 Data and types

| Name | Shape | File |
|---|---|---|
| `Subject` | `kind` (`session | pr | ...`), `key` (`session:<id>`), `id`, `title`, `mark` (glyph, said, tone), `phase`, `at`, `idleMinutes`, `paused`, `past`, `earlier`, `linked` (depth, rails, group, from), `handoff`, `reply`, `tab`, `bayAsk`, `pr`, `root` | `DK/ship.ts` |
| `Scope` | `"active" | "paused" | "quiet" | "ended" | "all"` | `store.ts:25` |
| `Session` (draft) | list row source: `paused`, `toolsMissing`, issue, workflow | `packages/screens/src/draft/sessions.ts:1096` |
| `SessionRecord` | wire: `state`, `waiting_for`, `hosted.start`, `hosted.paused`, `title`, `started_at` | `packages/protocol/src/sessions.ts` |
| `Item` | a call: `key`, `title`, `kind`, `fact`, `waiting.item.source` | `apps/desktop/src/renderer/src/Dashboard.tsx` |
| `RowView` | boolean per `ViewField` | `row-view.tsx` |

Calls come from `useItems("command-central")`; `incomingOf` (`store.ts:526`) and `callOwnerOf` (`store.ts:578`) pick one call per Job.

`console-bridge.css` is imported unconditionally by `DK/Deck.tsx:164` and applies through `:root:root`.
The flight-strip skin (36px rows, lamp bar, square corners, plate headings) is today's look and is the spec.
The "28px" in the comment above `.deck-list__rows` at `DK/deck.css:316` (`--deck-line: var(--h-control)`, `--row-min`) is stale: it describes the hairline row the strip replaces.
Density follows `spacing.density` (see the table at the top); the strip's min-height is `--space-6 + --space-3`: 27px Compact, 36px Default, 45px Roomy.
The `h` key on a row opens its earlier Job, and it is bound today (`Deck.tsx:669`).
It is in the spec even though it is not in `actions.toml`; its source comment says it waits on the look.

Measured on the mock Fleet at 1440x900 (walk `cockpit-list-by-repository`, Bridge served at `http://localhost:40024/`): the hover chip does overlap the first plate or row, and `.deck-list__rows` has `margin-top: 0px`, so the `deck.css:244-246` lane of 32px is not in effect.

| Box | x | y | width | height | right | bottom |
|---|---|---|---|---|---|---|
| `section.deck-list` | 12 | 56 | 288 | 832 | 300 | 888 |
| `.deck-list__rows` | 13 | 58 | 286 | 829 | 299 | 887 |
| `.deck-band` chip (hovered, opacity 1) | 263 | 62 | 32 | 24 | 295 | 86 |
| its Filter button inside the chip | 267 | 62 | 24 | 24 | 291 | 86 |
| first `.deck-list__head` plate (`armada`) | 21 | 66 | 270 | 24 | 291 | 90 |
| first `.deck-row` under that plate | 21 | 94 | 36 high, 270 wide | | 291 | 130 |

The chip and the first plate overlap by 28px across and 20px down (x 263-291, y 66-86), so the chip lies over the plate's right end and its Filter button covers it.
The plate's text sits at the left, so no text is covered; the chip is at `z-index: 2` and the plate is under it.
With no plate (one repository), the first row has the same top and right edges as the plate here (y 66, right 291), so the chip would lie over the first row's right end and its state marks.
Opacity is 0 until the panel is hovered or focused inside, then 1.
Screenshot: `.playwright-mcp/jobs-list-chip-hover.png`.
Today no lane is reserved for the chip.

## 2. Needs you

Needs you is a band and panel joined to the top of the Job's composer, plus pulses elsewhere.
The list-side pulse is in 1.4; the Calls HUD and Dock are in section 5.

### 2.1 Where it lives

| What | File |
|---|---|
| Panel | `PC/compositions/NeedsYouPanel/NeedsYouPanel.tsx` (715 lines), `NeedsYouPanel.css` (426 lines) |
| Other bands in the same family | `PC/compositions/NeedsYouPanel/WaitingBand.tsx` (a wait that is not him), `HandoffBand`, `StartAct`, `useEscapeFold.ts`, `useScrimFold.ts` |
| Row rendering and marks | `PC/compositions/SessionLedger/SessionLedger.tsx` (`waitingMark` 892, `WAITING_ACT` 275, `QUESTION` 330, `REPLY` 312, `HELD_CHILD` 318, `READ_FROM_REPLY` 324, `LedgerRows`, `RestartKey`), `SessionLedger.css` |
| Host (what the panel is given) | `apps/desktop/src/renderer/src/sessions.tsx:~2354-2660` and `:~3300-3380` |
| Waiting data | `apps/desktop/src/renderer/src/cockpit/waiting.ts` |
| Key | registry `needs_you`, `⇧⌘Y`, scope `thread` |

### 2.2 Layout

```
.armada-needs-panel   (absolute, bottom edge = composer top edge, no height, z-index 2, pointer-events none; children auto)
  .armada-needs-panel__band
     button.__trigger [aria-label "Needs you"]
        BellRing 12px | "Needs you" | glyph marks per thing (12px) | ChevronUp 12px (fold)
     RestartKey*  (a restart onto a build, its own key beside the trigger)
  .__reveal > .__shell > .__body
     list:   h3 eyebrow "Questions" + rows | "To review" + one row | Walks / Commands / PRs groups
     detail: head [Back + thumbs] + content
```

| Property | Value | File |
|---|---|---|
| Hue | `--hue: var(--step-waiting)`; `data-kind="end"` uses `--step-advanced`; `"failed"` uses `--status-completed-failed`; `"waiting"` uses `--status-handed-in` | `NeedsYouPanel.css:3-30` |
| Band, body, composer | one hue, no gap, so they read as one object | comment `NeedsYouPanel.css:1` |
| Reason text of a failed handoff | `--text-xs` 13px / `--leading-xs` 18px, `--fg-muted` | `NeedsYouPanel.css:79-85` |

Sizes, all Default-set values (`NeedsYouPanel.css` unless a file is named):

| Part | Value | Line |
|---|---|---|
| Panel box | `position: absolute; inset-block-end: 100%; inset-inline: 0; z-index: 2; flex column`; `pointer-events: none` on the box and `auto` on its children | 3-14, 95-97 |
| Band | `height: var(--h-badge)` 20px; flex; fill `--hue`; ink `--fg-inverse`; `overflow: hidden`; top corners `--radius-md` 6px folded, `--radius-lg` 8px open, over `--duration-sheet` 220ms | 100-113 |
| Band in the Job pane (the look) | top corners `--glass-radius-chip` (= `--radius-md`) in both states | `PC/depth.css:865-868` |
| Trigger and restart key | flex, centred; gap `--space-2` 8px; padding `0 --space-3` 12px; no border; trigger `flex: 1`, `min-width: 0`; `aria-disabled` cursor default | 114-137 |
| Restart key | `flex: none`; fill `--accent`, hover `--accent-hover`; `--text-xs` 13px, weight 500 | 138-150 |
| Band label "Needs you" | `--text-2xs` 12px / `--leading-2xs` 16px; weight 600; tracking `--tracking-caps` 0.04em; uppercase | 159-166 |
| Band glyph marks | inline-flex, gap `--space-1` 4px; each mark 12px | 167-171 |
| Fold chevron | `margin-inline-start: auto`; rotates 180deg when open over `--duration-sheet` | 172-181 |
| Reveal | grid rows `0fr` folded to `1fr` open over `--duration-sheet` 220ms; `visibility` flips at the end of the close | 183-199 |
| Body | flex column, gap `--space-3` 12px; padding `--space-2 --space-3` (8px 12px); side borders 1px `--hue`, no top or bottom border; fill `--bg-raised`; `max-height: calc(var(--thread-h, 55vh) * 0.95 - var(--h-badge))`; scrolls; opacity 0 folded, 1 open (`--duration-base` 180ms after `--duration-press` 90ms) | 207-228 |
| Body in the Job pane (the look) | fill `color-mix(in srgb, var(--fg-default) 8%, var(--glass-field))` | `PC/depth.css:869-871` |
| Thread dimmer | `.armada-session-thread__stage::after`: inset `-8px -12px 0` (`--space-2`, `--space-3`), `z-index: 1`, `color-mix(--bg-base 50%, transparent)`, opacity 1 while the panel is open, a press folds it | 229-246 |
| List and detail entry | rise `--space-2` 8px and fade over `--duration-base` | 248-259 |
| List | flex column; gap `--space-4` 16px between groups; `padding-block-end: --space-3` 12px | 261-266 |
| Detail | flex column; `align-items: flex-start`; gap `--space-2` 8px | 268-274 |
| Second press beside a row | `align-self: center; margin-inline-end: --space-2` | 276-280 |
| Head (Back and thumbs) | flex, space-between, `align-self: stretch` | 282-288 |
| Thumbs | flex, gap `--space-1` 4px, `margin-inline-end: --space-2`; each thumb button `--space-6` 24px square, glyph 12px | 290-298, 353-358 |
| Operator options | grid, columns `calc(var(--space-12) * 2)` 96px and `calc(var(--space-12) + var(--space-2))` 56px; gap `--space-1`; `margin-inline-end: --space-2` | 300-310 |
| Start act | `inline-size: calc(var(--space-12) * 2)` 96px, right-aligned; the button weight 600; Starting: gap `--space-1`, `--fg-muted`, 13px, weight 600, its glyph spins over `--duration-pulse` 1600ms (stops under reduced motion) | 312-351 |
| Open question | `.armada-question-deck__answers` `min-height: calc(var(--space-6) * 5)` 120px, `overflow-y: auto`, padding `--space-1`, margin `-1 * --space-1` | 404-411 |
| Reduced motion | every transition in this block is `none`; the list and detail entry animations are `none` | 413-426 |

Rows and eyebrows inside the panel come from `SessionLedger.css`:

| Part | Value | Line |
|---|---|---|
| Group | flex column; gap `--space-2` 8px | 19-24 |
| Eyebrow (base rule) | margin 0; `--fg-muted`; `--text-2xs` 12px / `--leading-2xs` 16px; weight 600; `--tracking-caps` 0.04em; uppercase | 25-34 |
| Eyebrow (the look, which wins) | `--font-display` (= `--font-sans`); `--text-head` (= `--text-xs`) 13px / `--leading-head` 16px; `--weight-head` 600; `--tracking-head` 0.12em; uppercase | `PC/depth.css:22-30`, tokens `packages/tokens/takes.css:22,27,32` |
| Rows list | flex column; gap `--space-2` 8px; no margin, padding or marker | 35-42 |
| Row | flex, wraps, `min-width: 0`; 1px `--border-subtle` border; radius `--radius-md` 6px; fill `--bg-raised`; `--text-xs` 13px / 18px; on arrival it fades from `--status-running` at `--row-tint-recent` over `--duration-decay` 45s; hover: border `--border-strong`, fill `--bg-hover` | 67-88, 181-183 |
| Open button (the row's press) | flex 1; centred; gap `--space-2` 8px; padding `--space-2 --space-3` (8px 12px); no border; radius inherited; `font: inherit`; focus ring `--focus-ring` offset `--focus-ring-offset` 2px | 89-109 |
| Row text | flex 1; one line with ellipsis | 110-115 |
| Offer row (a restart Fleet offers) | text padding `--space-2 --space-3`, wraps anywhere; key padding `--space-1 --space-2`, radius 6px, fill `--accent` (hover `--accent-hover`), weight 500, gap `--space-2` | 503-540 |
| Row marks | 12px glyph in `.armada-session-mark` (`role="img"`) | `NeedsYouPanel.tsx:442` |
| Release and dismiss | shown only while the row is hovered or holds focus | 549-564 |

Hue by kind (`NeedsYouPanel.css:3-30`): default `--step-waiting`; `end` `--step-advanced`; `failed` `--status-completed-failed`; `waiting` `--status-handed-in`.

### 2.3 Controls

| Control | Label / tooltip | Glyph | Action | Shown when |
|---|---|---|---|---|
| Trigger | aria `Needs you`; label text `Needs you` | `BellRing` 12px, `ChevronUp` 12px when not empty | toggles panel open; when empty and the sheet has items, opens the sheet | panel has things, a restart, or sheet items |
| Thing marks on band | tooltip and aria = the thing's `tip` (the row name) | per row via `waitingMark` | none | one per thing waiting |
| Review marks on band | tooltip and aria = `said` (`Plan`, `Sketch`, `Draft`, `Document`) | `ListTodo`, `PencilRuler`, `MessageSquareText`, `FileText` | none | sheet has items |
| Restart key | `RestartKey` with row name | per file | restarts onto the build | row has `restart` and no `onDismiss` |
| Question row | aria `Open <name>` | `MessageCircleQuestion` (`Question`) or per row | opens it in place (`choose`) or goes where it points (`press` / `onOpen`) | `asks(row)` |
| Thumbs | `A question` (`ThumbsUp`), `Not a question` (`ThumbsDown`), both ghost sm icon-only 12px | as named | `onConfirmed` / `onMisread` | row has `onMisread`; `A question` hidden once `confirmed` |
| Row menu (right press, `ContextMenu`, Shift+F10) | triggerLabel `Question`; items `A question`, `Not a question` | `ThumbsUp`, `ThumbsDown` | same as thumbs | row has `misread` |
| Operator option buttons | aria `<option label>: <name>`; first is secondary, rest ghost | none | `onPick` | Operator row with options |
| StartAct | per `StartAct.tsx` | per file | starts a held Job | row has `start` |
| Review row | aria `Review`, `aria-haspopup="dialog"`, text = `said` list joined by `, ` | the review glyphs | opens the review sheet | sheet has items; group eyebrow `To review` |
| Action groups | eyebrows `Walks`, `Commands`, `PRs`; rows from `LedgerRows kind="waiting"` | `AppWindow`, `Terminal`/`ListChecks`, `GitPullRequestArrow` | one press does the act | `acts(row)` |
| Back | tooltip `Back`, shortcut `Esc`, aria `Back` | `ChevronLeft` 16px | returns to list; with a lone question, folds the panel | detail open |

`WAITING_ACT` marks (tooltip text): walk `AppWindow` "Opens the walk"; answer `MessageSquare` "Goes to the question"; approve_pr `GitPullRequestArrow` "Opens the pull request"; run `Terminal` "Copies the command"; message `SquarePen` "Goes to the message"; steps `ListChecks` "Steps"; none `BellRing` "Waiting on you"; reply `Reply` "Reply"; held child `Link` "Opens the linked Job"; read from reply `ScanText`; operator `Bot` (text = `row.operator`).

### 2.4 States

| State | Drawn as | Code |
|---|---|---|
| Empty, no restart, no sheet | component returns null | `NeedsYouPanel.tsx:376` |
| Folded | band only | `data-open=false` |
| Open, list | groups in order: Questions, To review, Walks, Commands, PRs | `:475-640` |
| Open, detail | Back head and content; content is the question's card, a Fleet card, or one ledger row expanded | `:641-675` |
| One lone question | opens straight to its answer unless the sheet row must show | `only`, `:280` |
| Arrival | a new question id opens the panel once, and again on re-entering the Job; focus moves into it only if not typing | `:271-300`, `focusIsFree` |
| Typing | a question arriving while he types in the composer opens the panel `quiet`, focus stays | `:323` |
| Disabled trigger | `aria-disabled` when empty without sheet | `:434` |
| Dimmed thread | a press on the thread folds the panel | `useScrimFold` |
| Scope of "needs" | a plan awaiting approval is not here; the thread's card shows it whole | comment `:145` |
| Pulse on list | see 1.4 | |

### 2.5 Ordering

Questions first (the open `question`, then Fleet `cards`, then `rows.filter(asks)`), then the `To review` row, then `ACTIONS` groups in the order `Walks`, `Commands`, `PRs` (empty groups omitted).
`banded(row)` (a restart with no dismiss) is the band's own key and is not in the list.
`acts(row)` = not banded, no options, and matches a group.
`asks(row)` = neither banded nor acts.
Arrow keys step between `LIST_KEYS` (open buttons, offer keys, `[data-aside]`, run-act buttons), clamped at the ends.

### 2.6 Keys

| Key | Where | Does |
|---|---|---|
| `⇧⌘Y` (`needs_you`, scope `thread`) | window listener, not on repeat | opens the review sheet when it has items, else toggles this panel |
| `Esc` | panel open | detail: back to list; list: fold; with `data-keeps-escape` while open |
| `Backspace` | detail, not in a text field | back |
| `↓` `↑` | list | next or previous row |
| `ContextMenu`, `Shift+F10` | a row with misread | opens the menu |
| bare digits | question options | `call_pick` (registry) |

### 2.7 Strings

`Needs you`, `Questions`, `To review`, `Walks`, `Commands`, `PRs`, `Review`, `Back`, `A question`, `Not a question`, `Question`, `Open <name>`, `<option>: <name>`, `Plan`, `Sketch`, `Draft`, `Document`, plus the `WAITING_ACT` strings in 2.3.

### 2.8 Accessibility

`section[aria-label="Needs you"]`, trigger `aria-label="Needs you"` with `aria-expanded` (absent when empty) and `aria-haspopup="dialog"` when empty with a sheet; each group a `section` with the group label; rows `li[aria-label=<name>]`; marks `role="img"`.

### 2.9 Data

`LedgerEntry` (`SessionLedger.tsx`): `key`, `act`, `text`, `name`, `options`, `onReply`, `onOpen`, `onDismiss`, `onMisread`, `onConfirmed`, `confirmed`, `restart`, `start`, `operator`.
Wire: `WaitingItem`, `WaitingSource`, `WaitingAct`, `SessionStanding` in `packages/protocol/src/sessions.ts`.
`CALL_WORD` in 1.4 maps `source` to a word.
`BLOCKED` sources for sound: `ask_card`, `permission`, `window_ask`, `plan`, `slot_choice`, `delivery`, `task_done`, `browser_ask` (`session-attention.ts:19`).

Walks are Job tabs and are not in the review sheet; the code is the truth.
The doc comment at `NeedsYouPanel.tsx:148` that lists "walk" among the sheet's items is stale: `JobReviewSheet.tsx` builds only `plan`, `sketch`, `document` and `draft` items, and a walk is a `Walks` row in this panel.
The reply box inside a ledger row (`LedgerRows`, `replying`, `SessionLedger.tsx:~900-1200`) belongs to the Job view inventory, `docs/spikes/native-jobs-inventory/job-view.md`.

## 3. Review sheet

### 3.1 Where it lives

| What | File |
|---|---|
| Sheet composition | `PC/compositions/ReviewSheet/ReviewSheet.tsx` (512 lines), `ReviewSheet.css` (151 lines) |
| Shell it sits in | `PC/primitives/Sheet/Sheet.tsx`, `Sheet.css` |
| Host that builds the items | `apps/desktop/src/renderer/src/session-view/JobReviewSheet.tsx` (425 lines) |
| Opened from | `sessions.tsx:3326-3330` (`setReviewSheetFor(session.id)`) |
| Key | `⇧⌘Y` (`needs_you`) in `NeedsYouPanel.tsx:357-375` |

### 3.2 Layout

```
Sheet kind="job-review" floating size="widest" (max-width 76% of the ground) bleed
  head:   title = Job name | [Send back ⌥R] [Approve <name> ⌘↵] | close (Esc)
  body:   .armada-job-review (flex)
    left  list   width calc(var(--space-12) * 5) = 240px, --bg-sunken, right border 1px --border-default, scrolls
    right detail  body (padding --pad-card, --text-sm 14px / --leading-sm 20px, scrolls) + optional note foot
```

| Part | Value | CSS |
|---|---|---|
| Item row | height `--h-row-unit` 36px; padding `0 var(--space-3)`; gap `--space-2`; left border `--edge-active` transparent, `--accent` when selected; text `--text-sm` | `ReviewSheet.css` |
| Item hover and selected fill | `--surface-row-hover`, `--surface-row-selected` | same |
| Answered and sent-back | text `--fg-subtle` | same |
| Kind glyph | 12px, `--fg-muted`; inherits the muted colour for answered | same |
| Answered mark | `BadgeCheck`, colour `--status-completed-success` | same |
| Sent-back mark | `Reply`, colour `--fg-muted` | same |
| Name | one line, ellipsis | same |
| Fill body (sketch, document, draft) | `.armada-job-review__fill`, min-height `calc(var(--h-row-unit) * 8)` = 288px | same |
| Note foot | padding `--space-3 --space-4`, top border, `--bg-raised`; Textarea 3 rows; right-aligned acts, gap `--space-2` | same |
| Row context menu | z-index `calc(var(--z-modal) + 1)` | same |

### 3.3 Controls

| Control | Label | Glyph | Shortcut | Action | Shown when |
|---|---|---|---|---|---|
| Send back (head) | `Send back` | none | `⌥R` | opens the Note foot | current item `answer.sendBack` true; disabled while noting |
| Primary (head) | `<verb> <label ?? item name>`, e.g. `Approve Plan v2` | none | `⌘↵` | `onPrimary(id)` | item has `answer`; disabled while noting |
| Close | sheet close | per Sheet | `Esc` | `onClose` | always |
| Item row | name | kind glyph | `↑` `↓` `Home` `End` | selects and draws it | always |
| Note textarea | aria `Note` | none | `⌘↵` sends, `Esc` leaves keeping text | none | noting |
| Note Send back | `Send back` | none | `⌘↵` | `onSendBack(id, note)`; disabled when blank | noting |
| Note Cancel | `Cancel` | none | `Esc` | leaves the note | noting |
| Item menu | triggerLabel `Item`; entry `Dismiss` (or `row.dismissSaid`) | `X` | none | `onDismiss` | item has `menu` |

Kind marks (`REVIEW_MARKS`): `Plan` `ListTodo`; `Sketch` `PencilRuler`; `Draft` `MessageSquareText`; `Document` `FileText`.
Verbs: `Approve` for plan, sketch, document and draft; `Send` for a draft that is a rule (`card.rule`), which has no Send back.

### 3.4 States

| State | Drawn as |
|---|---|
| Item open | normal ink; kind glyph in `--fg-muted` |
| Item answered | muted ink, `BadgeCheck` mark, tooltip `said` (`Approved`, `Sent`, `Dismissed`, or `Answered`) |
| Item sent back | muted ink, `Reply` mark, tooltip `Sent back` |
| Selected | `aria-current="true"`, `tabIndex` 0 (others -1), accent bar and selected fill |
| Loading body | four `Skeleton` lines (widths 40%, full, 85%, 60%) |
| Noting | head acts disabled; foot with Note replaces nothing else; `Esc` leaves the note first, then closes |
| After an answer | the next open item after it comes up, else the first open one (`nextOpenItem`); the sheet closes after the last |
| Body carries its own answers (a question's options) | `answer` absent, so the head shows no Send back or primary |

### 3.5 Ordering

Items sorted oldest first by `since` (`JobReviewSheet.tsx:~285`).
Only unanswered `waiting_for` items of source `plan`, `sketch`, `document`, `draft` get an entry; rows with no sheet entry stay on the Needs you card (`cardRowsOf`).
A Job with exactly one owed document (`lonePage`) opens that document directly instead of the sheet (`sessions.tsx:3328`).

### 3.6 Keys

| Key | Does |
|---|---|
| `⌘↵` | primary; in the note, send |
| `⌥R` | Send back |
| `Esc` | leave note, then close |
| `↑` `↓` `Home` `End` | move the list and draw at once |
| `Tab` | stays inside the sheet |
| `ContextMenu`, right press | item menu |

### 3.7 Strings

`Waiting on you` (list group aria), `Send back`, `Approve`, `Send`, `Cancel`, `Note`, `Dismiss`, `Approved`, `Sent`, `Sent back`, `Answered`, `Dismissed`, `Item`, `Plan`, `Sketch`, `Draft`, `Document`.

### 3.8 Accessibility

Sheet is a dialog; list is `role="group"` `aria-label="Waiting on you"`; items are buttons; detail is a `section` with aria-label = item name; marks are `role="img"`.

### 3.9 Data

`ReviewItem` (`ReviewSheet.tsx`): `id`, `mark`, `name`, `menu`, `state` (`open | answered | sent_back`), `said`, `answer` (`verb`, `label?`, `sendBack`), `loading`, `body`.
Props: `open`, `job`, `items`, `keep` (draft key), `onPrimary`, `onSendBack`, `onSelect`, `onClose`, `floor`.
Host builds each from a `LedgerEntry` plus parts (`plan`, `drafts`, sketch and document attachments via `attachmentsOf`).

The sheet bodies (plan card, `SketchBody`, `DocumentBody`, `DraftRow`) belong to the Job view inventory, `docs/spikes/native-jobs-inventory/job-view.md`.

## 4. Command palette for Jobs

### 4.1 Where it lives

| What | File |
|---|---|
| Entries, sections, acts | `packages/shell/src/Palette.tsx` (sections ~535-600; session entries ~777-790; `metaOfJob` 1163; `KIND` 1182; `actionsOf`/`ownActs` ~1300-1420) |
| Component | `PC/primitives/CommandPalette/CommandPalette.tsx`, `.css`, `actions-panel.tsx`, `row-meta.tsx` |
| Mount and rows | `PaletteMount.tsx`, `palette-job-row.ts`, `palette-page.ts`, `palette-rows.ts:~210-250` |
| Contract | `docs/contracts/design-system/keyboard-and-command-palette.md` (grep only) |

### 4.2 Layout

Palette is `role="dialog"` `aria-label="Command palette"`, with a `role="combobox"` input and `role="listbox"` list (`id="armada-palette-list"`); each row `role="option"`.
Default placeholder `Search actions, jobs and settings`; the input aria-label is the placeholder (or the step title in a step flow).
An empty result is `role="alert"`.
Row: glyph, label (title, then subtitle), kind flush right via `KIND`, then meta.
A Job row's meta (`metaOfJob`): `repo` and `state` when present.

Section headings, in order (`Palette.tsx:523-607`): the open page (titled with the page, only if it has rows), then the focused thing (`on`, else `Nothing focused`), `Navigation`, `Commands` (only if a mod brings some), `Run in the checkout`, `Repositories`, `Jobs`, `Pull requests`, `Issues`, `Tasks`, `Settings`.
With the setting *Palette opens with* = `Pinned and recent` the empty query shows `Pinned` then `Recent` (`Palette.tsx:~895-920`); `This page first` shows the whole list headed by the page; `Everything` shows the whole list without the page's rows.
A word he gave a row, typed whole, puts that row first, under no heading.

Sizes, Default set.
Where `depth.css` restates a rule under `:root[data-look="new"]` (forced on) it wins, and both are given.
The base rules are `PC/primitives/CommandPalette/CommandPalette.css` (`CP`), the look is `PC/depth.css:938-1028`.

| Part | Base (`CP`) | The look (wins) |
|---|---|---|
| Layer | fixed, inset 0, `z-index: var(--z-palette)` 90, flex centred, `align-items: flex-start`, `padding-top: 20vh`, fill `--scrim`, fades in over `--duration-base` (CP:27-41) | |
| Box | width 100%, `max-width: calc(var(--palette-width) * 1.2)` = 672px (`--palette-width` 560px, `spacing.css:354`), `max-height: var(--palette-max-height)` 400px, radius `--radius-lg` 8px, fill `--bg-overlay`, border 1px `--border-default`, shadow `--shadow-overlay`, `overflow: hidden`, enters over `--duration-base` rising 8px from 0.98 scale (CP:56-69) | border colour `rgb(255 255 255 / 0.12)`; shadow `--shadow-overlay` plus `--glass-highlight` (inset 1px, `--fg-default` at 24%); enters over 120ms rising `--space-1` 4px; no animation under reduced motion (depth.css:953-966, 1024-1028) |
| Anchored (opened from the title bar's field, or dragged) | `position: absolute`, `width: min(var(--palette-wide), calc(100vw - var(--space-6)))` (672px or the window minus 24px), `translate: -50% 0`, no top padding (CP:259-268) | |
| Field row | flex, 1px bottom rule `--border-subtle` (CP:72-77) | rule `--head-rule` (= `--border-subtle`) (depth.css:967) |
| Input | `height: var(--h-control)` 28px, padding `0 --space-3` 12px, no border or radius, transparent, `--font-sans`, `--text-lg` 18px / `--leading-lg` 28px, placeholder `--fg-subtle`, no focus outline (CP:93-118) | `height: var(--space-12)` 48px, padding `0 --space-4` 16px, 18px (depth.css:973-977) |
| Command chip before the input (`__of`) | height `--space-6` 24px, `margin-left: --space-3`, padding `0 --space-2`, radius `--radius-sm` 3px, fill `--fg-default` at 10%, `--text-sm` 14px, no wrap; the input after it has `padding-left: --space-2` (CP:79-109) | `margin-left: --space-4` 16px (depth.css:970) |
| List | flex 1, `min-height: 0`, scrolls, padding `--space-1 0` (CP:120-125) | padding `--space-1 --space-2 --space-2` = 4px 8px 8px (depth.css:978) |
| Section heading | height `--h-row-header` 28px, padding `0 --space-3`, `--fg-subtle`, `--text-2xs` 12px / 16px (CP:127-135) | height `calc(var(--space-8) - var(--space-1))` 28px, padding `0 --space-2` 8px, `--font-mono`, 12px, weight 600, tracking `--tracking-caps` 0.04em, uppercase (depth.css:981-989) |
| Step head (a second step) | min-height `--h-row-header`, padding `0 --space-3`, `--fg-default`, `--text-xs` 13px / 18px, weight 500, ellipsis (CP:138-151) | |
| Row | flex, centred, gap `--space-2` 8px, `height: var(--h-menu-item)` 28px, padding `0 --space-3` 12px, `--font-sans`, `--text-sm` 14px, `--fg-default`, background transition `--duration-fast` (CP:153-164) | `height: var(--palette-row)` = `--space-8` 32px (24 Compact, 40 Roomy; deliberately a spacing step, not a control height), padding `0 --space-2` 8px, radius `--radius-md` 6px (depth.css:956, 990-996) |
| Stacked row (a dispatch row carrying chips) | `height: auto`, `min-height: --h-menu-item`, `padding-block: --space-2`, `align-items: flex-start`; glyph and kind take `--leading-sm` 20px height; the label stacks with gap `--space-1` (CP:327-349) | `min-height: var(--palette-row)`, `padding-block: --space-2` (depth.css:998-1002) |
| Row, hover | | fill `--bg-step` (depth.css:1003-1005) |
| Row, active (under the cursor) | fill `--bg-hover` (CP:166) | one neutral rounded fill, `--fg-default` at 10%; no bar, no accent (depth.css:1008-1011) |
| Row, dormant (nothing answers it) | `--fg-subtle`, `cursor: default`; a `__dormant` note at its end, 12px / 16px `--fg-subtle` (CP:179-182, 241-246) | active or hover fill `--bg-step`; glyph `--fg-subtle` (depth.css:1013-1019) |
| Row, danger | label and glyph `--status-completed-failed`, no fill; the matched span is underlined 1px, offset `--space-1 / 2` (CP:173-175, 234-239) | |
| Row, trailing (what was typed, offered as work) | sticky bottom 0, top border 1px `--border-subtle`, fill `--bg-overlay`; the dispatch glyph is `--accent`; the typed words `--fg-default`, weight 500 (CP:305-323) | transparent, no top border (depth.css:1020-1023) |
| Glyph | `width: var(--armada-glyph)` = `--space-4` 16px, `--fg-muted`, centred, flex none (CP:22, 184-191) | |
| Label | flex row, baseline, gap `--space-3` 12px, flex 1, ellipsis (CP:199-205, 418-424); title `flex: none`, ellipsis (CP:426-433) | |
| Subtitle | `--fg-subtle`, `--text-xs` 13px, inline-flex gap `--space-2`, ellipsis (CP:435-450) | |
| Match | label drops to `--fg-muted`, the matched span stays `--fg-default`; dormant: `--fg-muted` span (CP:215-232) | |
| Kind column | flex none, `width: calc(var(--space-12) * 2)` 96px, right-aligned, `--fg-subtle`, `--text-xs` 13px, no wrap; while a quick key is revealed (`data-revealing`) the word blurs `--space-1` 4px and drops to 60% opacity over `--duration-fast`; `.armada-palette__quick` is absolute over the column (CP:452-483) | |
| Chips (dispatch row) | height `--space-6` 24px, padding `0 --space-2`, 1px `--border-default`, radius `--radius-sm`, fill `--bg-raised`, `--font-mono`, 12px / 16px, weight 500; open: border `--accent`; glyph `--fg-muted`; wrap gap `--space-1` (CP:352-392) | |
| Empty (`role="alert"`) | padding `--space-3` 12px, `--fg-muted`, `--text-sm` 14px / 20px, `text-wrap: pretty` (CP:248-255) | |
| Foot (key legend) | flex, wrap, `justify-content: flex-end`, gap `--space-2 --space-4` (8px 16px), padding `--space-2 --space-3` (8px 12px), top border 1px `--border-subtle`, `--fg-subtle`, `--font-sans`, `--text-xs` 13px / 18px; each item inline-flex, gap `--space-1` (CP:395-413) | |
| Drag | the sections, foot and act panel title show `cursor: grab`; moving shows `grabbing` and no text selection; snap guides are 1px `--accent` lines through the window's middle (CP:271-301) | |
| Stacking | a dialog scrim over the palette is `--z-palette + 1` (CP:580-582) | |

The row's kind and quick-key behaviours are `row-meta.tsx` (59 lines); the ⌘K panel is `actions-panel.tsx`.

⌘K act panel (`.armada-palette__actions`, CP:486-572):

| Part | Value |
|---|---|
| Box | absolute; `right: --space-2` 8px; `bottom: calc(var(--space-8) + var(--space-1))` 36px; flex column; `width: calc(var(--palette-width) / 1.6)` 350px; `max-height: calc(100% - var(--space-12) - var(--space-8) - var(--space-2))` (100% minus 88px); 1px `--border-default` border; radius `--radius-lg`; fill `--bg-overlay`; shadow `--shadow-overlay` |
| Title | padding `--space-2 --space-3 --space-1`, `--text-xs` 13px, `--fg-subtle`, ellipsis |
| List | flex 1, scrolls, padding `0 --space-1 --space-1`; rows are the palette rows above; a row's chord is a `Kbd` with `margin-left: auto` |
| Held value | `max-width: 50%`, `margin-left: auto`, `--text-xs`, `--fg-subtle`, ellipsis |
| Refusal | `padding-right: --space-3`, `--status-escalated`, `--text-xs`, no wrap |
| Foot | flex, centred, top border 1px `--border-subtle`; its field is `height: var(--h-menu-item)` 28px, padding `0 --space-3`, `--text-sm`, no border, transparent, placeholder `--fg-subtle` |
| Rule between groups | 1px high, margin `--space-1 --space-2`, `--border-subtle` |
| With the panel open | the list's `min-height` is `calc(var(--h-menu-item) * 11 + var(--space-4))` = 324px |

`KIND` text by id prefix: `session:` `Job`; `pull:` `Pull request`; `issue:` `Issue`; `task:` `Task`; `cmd:` `Command`; `nav:` `Page`; `set:` `Setting`; `run:` `Check`; `repo:` `Repository`; `tab:` `Filter`.

### 4.3 Job acts (⌘K panel on a Job row)

Built in `ownActs` (`Palette.tsx:1315`); each chord runs while the palette is up, none is Global.

| Act | Label | Glyph | Chord | Shown when |
|---|---|---|---|---|
| open | `Open` | none | `↵` | always |
| pause or resume | `Pause` / `Resume` | `Pause` / `Play` | `⌘P` | `Resume` when the Job is paused |
| fork | `Fork` | `GitFork` | `⌥⌘F` | always |
| open-pull | `Open pull request` | `GitPullRequest` | `⇧⌘P` | Job has a pull request; set apart |
| merge-pull | `Merge pull request` | `GitMerge` | `⇧⌘M` | Job has a pull request |
| copy-id | `Copy Job id` | `Copy` | `⇧⌘C` | always; set apart |
| pin or unpin | `Pin` / `Unpin` | none | `⇧⌘F` | always; set apart; palette stays open (`stays`) |
| stop | `Stop` | `Ban` | `⌘⌫` | always; destructive; set apart; confirms |

Acts listed in a Job's `without` array are filtered out (`acts.filter(... job?.without ...)`).
Pull acts: `Open in Reviews` `↵`, `Open on the forge` `⇧⌘O`, `Start a Job on it` `⇧⌘N`.
Other rows have their own acts (`Dispatch`, `Tick`, `Run`, `Set alias`, `Remove alias`, `Record hotkey`, `Remove hotkey`); they are not Job acts.

### 4.4 Which Jobs appear

| Rule | Code |
|---|---|
| Live Sessions become `session:<id>` entries with title, repo and state | `Palette.tsx:~777-790` |
| Ended and quiet Jobs are excluded (`dead === undefined`) | `Palette.tsx` session filter |
| A Job is matched by title and the fields the palette ranks | `palette-rows.ts` |
| A Job's `state` meta comes from `PaletteSession.state` | `metaOfJob` |
| Pinned ids are kept by the host; `Pin` is on every kind of row | `actionsOf` |

### 4.5 Keys

| Key | Does |
|---|---|
| `⌘K` | opens the palette (registry `command_palette`, scope `anywhere`) |
| `⌘K` on a row | opens that row's act panel |
| `↵` | runs the default act (`Open`) |
| chords in 4.3 | run that act on the highlighted Job |
| `Esc` | closes the panel then the palette |

### 4.6 Strings

`Command palette`, `Search actions, jobs and settings`, the section headings in 4.2 (`Navigation`, `Commands`, `Run in the checkout`, `Repositories`, `Jobs`, `Pull requests`, `Issues`, `Tasks`, `Settings`, `Pinned`, `Recent`, `Nothing focused`), the setting's values (`Pinned and recent`, `This page first`, `Everything`), `All repositories`, the `KIND` words, and the act labels in 4.3 and 4.3's other rows (`Open in Reviews`, `Open on the forge`, `Start a Job on it`, `Dispatch`, `Tick`, `Run`, `Set alias`, `Remove alias`, `Record hotkey`, `Remove hotkey`).

### 4.7 Data

`PaletteSession` (`id`, `title`, `repo`, `state`, `paused`, `pull`, `without`), `PaletteEntry`, `PaletteAction` (`id`, `label`, `icon`, `shortcut`, `apart`, `stays`, `destructive`), `PaletteMeta` (`repo`, `state`).

Every palette act in the code is in scope, including `Open pull request ⇧⌘P` and `Merge pull request ⇧⌘M` for a Job with a pull request.
Where the contract's act table (`docs/contracts/design-system/keyboard-and-command-palette.md:535-547`) differs from `Palette.tsx`, the code is the authority.
The act lists for Job, Job holding a pull request, Pull request, Issue, Task and Command match the contract row for row.
The differences:

| Contract says | Code does | Where |
|---|---|---|
| "Any other row: Set alias and Record hotkey only" | every row that is not `dormant` also gets `Set alias` (set apart, takes a word), `Remove alias` (only once an alias is set, palette stays up), `Record hotkey` (takes a chord) and `Remove hotkey` (only once a hotkey is set, stays up); a Job, Pull request, Issue, Task and Command get them after their own acts | `Palette.tsx:1217-1275` (`actionsOf`) |
| the table lists no alias or hotkey act for any row kind | a `destructive` entry (a Job's `Stop` is the act, the entry is the row) has no `Record hotkey` or `Remove hotkey`; a `dormant` row has none of the four | `Palette.tsx:1217-1275` |
| "A Job's act that nothing answers is left out" | filtered by the Job's `without` list, matched on act id | `Palette.tsx:1400` |
| Pin acts are `Pin ⇧⌘F` | the label flips to `Unpin` when the id is already pinned; both are set apart and stay up | `Palette.tsx:1324-1340` |
| no separator information | `apart` (a rule above the act) is set on `Open pull request`, `Copy Job id`, `Pin`/`Unpin`, `Stop`, `Start a Job on it`, `Set alias` | `Palette.tsx:1318-1440` |
| no glyphs named | Job: `Pause`/`Play`, `GitFork`, `GitPullRequest`, `GitMerge`, `Copy`, `Ban`; Pull request: `GitPullRequest`, `ExternalLink`, `Plus`; `Open`, `Dispatch`, `Tick`, `Run` glyphs per file | `Palette.tsx:1342-1480` |

The contract's "Pinned and recent", "This page first" and "Everything" are the values of the setting *Palette opens with* (`PALETTE_OPENS`, `Palette.tsx:~138`); the code's section headings are in 4.2.
Palette sizes are in 4.2.

## 5. Notification banner, sound, Dock badge, toast

One set of calls drives every signal: `useItems("command-central")`, narrowed by `incomingOf` (`DK/store.ts:526`) to one call per Job (`callOwnerOf`).
The pulsing row, the Cockpit tab pip, the Calls HUD, the toast and the Dock badge agree because they read the same set.

### 5.1 Where it lives

| Signal | File |
|---|---|
| macOS banner for a Job that started waiting | `apps/desktop/src/main/session-attention.ts` (rule, words, tone), `apps/desktop/src/main/index.ts:~677-725` (effect) |
| Banner hold | `apps/desktop/src/main/banners.ts` (`showHeld`) |
| Banner sound | `apps/desktop/src/main/telling.ts` (`soundOf`), `apps/desktop/sounds/armada-blocked.wav`, `armada-waiting.wav`, `tones.mjs` |
| Other OS notices | `downshift-notice.ts`, `agent-sign-in-notice.ts` (same `showHeld`) |
| Dock badge | `apps/desktop/src/renderer/src/cockpit/DockCount.tsx`; main `index.ts:1142-1144` |
| In-app toast | `apps/desktop/src/renderer/src/cockpit/CallAlerts.tsx`, `arrivals.ts` (`alertOf` 51), `PC/primitives/Toast/Toast.tsx`, `Toast.css` |
| Tab pip | `PC/primitives/Tabs/Tabs.css:149` |
| Calls HUD | `DK/ConsoleCalls.tsx`, tile in `DK/ConsoleBays.tsx:~215-260` and `flight-controls.ts` (built-in tile `needs`, label `Calls`, glyph `megaphone`) |
| UI sounds (separate) | `apps/desktop/src/renderer/src/ui-sounds.ts`, `app-effects.ts:68` |
| Contract | `docs/contracts/design-system/sound.md` |
| Setting | `features.notifications` (live), `features.uiSounds`; `crates/config/settings.toml:~258` |

### 5.2 Banner

| Aspect | Behaviour | Code |
|---|---|---|
| Trigger | an item newly entering a live Session's `waiting_for`, once; an unpaused Session; the start card of a held Job counts as an item `id: "start"`, text `Not started`, blocking | `session-attention.ts:55-110` |
| Not news | what was waiting when Bridge connected only seeds; nothing before `readAt` is non-null; an item re-posted under the same id is the same item; a paused Job tells nothing | `session-attention.ts:read` |
| Batch | entries collected in a 2000ms window (`QUIET_MS`), not extended; one banner per window | `:open` |
| Title | the Job title of the oldest entry, or `Job` if none | `session-attention.ts` |
| Body | the oldest item's text; with more, `<text>\nand <n> more` | same |
| Tone | `blocked` if any batched item blocks, else `waiting` | same |
| Blocking sources | `ask_card`, `permission`, `window_ask`, `plan`, `slot_choice`, `delivery`, `task_done`, `browser_ask`, and the start card | `BLOCKED` |
| Gate | no banner while any Bridge window has focus; none when `features.notifications` is false; none when `Notification.isSupported()` is false | `index.ts:~700-706` |
| Press | `summon({ sessionId })` lands on the Job | `index.ts:~712` |
| Hold | `showHeld` keeps a reference until click, close or failure | `banners.ts` |
| Permission | macOS's; the first `show()` makes macOS ask once | `telling.ts` comment |
| Other notices | downshift and agent-sign-in banners show with or without focus; press focuses Bridge | `index.ts:~677-700` |

### 5.3 Sound

| Tone | Sound name | Plays when | Shape |
|---|---|---|---|
| Blocked | `armada-blocked` | a batch holds a blocking item | G4 then D4, falling, 170ms apart |
| Waiting | `armada-waiting` | anything else | one A4 |

Each note: sine, 200ms, 12ms rise, exponential decay; one level for both (`sound.md`).
The tone is the banner's own `sound` field (a name), so macOS permission, Focus and per-app settings mute it with the banner.
A packaged build carries the wavs in Resources; a development build installs them into `~/Library/Sounds`.
The downshift offer takes the Waiting tone.
Interface sounds (`Move`, `Press`) are synthesised in the renderer behind `features.uiSounds` and are not part of the banner.

### 5.4 Dock badge

| Aspect | Behaviour |
|---|---|
| Value | `incomingOf(useItems("command-central"), held, visited).length`, one per Job |
| Source | the window computes it, because only the window knows which Jobs he has been in (`visited`) |
| Send | `window.armada.dockCount(count)`, only after `state.readAt !== null` |
| Apply | main accepts non-negative integers only; `app.dock.setBadge(calls === 0 ? "" : String(calls))` |
| Banner state | independent of focus; shown either way |

### 5.5 In-app toast

| Aspect | Behaviour | Code |
|---|---|---|
| Text | one call: `“<title>” needs you.`; several: `<n> calls need you.` (curly quotes) | `arrivals.ts:alertOf` |
| Order | oldest call first; the key is the oldest's | same |
| Shown when | a call arrives after the first reading settles (`settle.ms` 1500) and the Cockpit is not on screen | `CallAlerts.tsx` |
| Stands | `ALERT_MS` 15000ms; at most `MOST` 3, the oldest drops when a fourth comes | `CallAlerts.tsx` |
| Cleared | when the Cockpit comes forward, and when its call no longer stands | same |
| Press | removes it and calls `onShow(key)`, which brings the call forward on the Cockpit | same |
| Look | region fixed bottom right, `right` and `bottom` `--space-6` 24px, `z-index: --z-toast` 80, column, gap `--space-2` 8px (`Toast.css:~99-110`); toast: flex, centred, gap `--space-3` 12px, `max-width: --w-toast` 420px, padding `--space-3` 12px, radius `--radius-md` 6px, fill `--bg-overlay`, 1px `--border-default`, `--shadow-overlay`, `--fg-default`, `--text-sm` 14px / 20px; enters over `--duration-base` rising `--space-2`; `role="status"`; the whole text is one button (`onPress`), underlined on hover; this toast has no status dot and no action | `Toast.css:19-110`, `Toast.tsx:29-50` |
| Press goes to | `goTo(SURFACE.overview)` then `lands.bring(key)` | `App.tsx:1285` |

### 5.6 Strings

Banner: title = Job title; body = item text, `Not started`, `<text>\nand <n> more`.
Toast: `“<title>” needs you.`, `<n> calls need you.`.
Dock: a bare integer string.
Calls tile: `Calls`.

### 5.7 Data

`Told` (`sessionId`, `title`, `body`, `tone`), `Tone` (`blocked | waiting`), `Alert` (`key`, `keys`, `sentence`), `Held`, `Item`, `WaitingItem` (`id`, `text`, `since`, `source`), `SessionRecord.waiting_for`.

"Which Jobs he has been in" is `apps/desktop/src/renderer/src/cockpit/viewed.ts`, and it moves to a Fleet view; Fleet will serve it to the native app.
Today's behaviour, which the Fleet view has to reproduce:

| Aspect | Today |
|---|---|
| Store | `localStorage` key `armada.bridge.session-viewed`, a map of Job id to a time in ms; another window's writes arrive on the `storage` event |
| Use | the Cockpit raises a call for a Job's waiting item only if he has not been in that Job since the item began waiting (`touchedAt` in `waiting.ts`) |
| In now | a Job open on the Sessions page or in its walk window, with the window visible, reads as viewed "now" (`viewedAt` returns `Date.now()`) until he leaves, and its time is stamped again when he does (`markViewed`, `useViewing`) |
| At the machine | `viewingNow` also needs input within `AWAY_MS` = 60 000ms |
| Reset | `forgetViewed()` clears the map |

The count the Dock shows is separate: it reads the deck store's `held` (calls answered) and `visited` (calls opened) sets through `incomingOf` (`DK/store.ts:526`), both held in the window and not persisted.
The Calls HUD (`ConsoleCalls.tsx`, the Calls tile in `ConsoleBays.tsx`) goes with the Cockpit console after switch day and is out of scope for the native Jobs screens.
Cockpit tab pip (`PC/primitives/Tabs/Tabs.css:149-158`, `Tabs.tsx:132-135`):

| Part | Value |
|---|---|
| Shape | `.armada-tabs__alert`: `inline-block`, `width` and `height` `var(--space-2)` 8px, `border-radius: 50%`, `margin-left: --space-2` 8px, `vertical-align: middle` |
| Colour | `--status-escalated`, the needs-you hue; never a figure |
| Name | `role="img"`, aria-label and tooltip `A call is waiting` (`TabBar.tsx:16`) |
| Shown when | the tab is `cockpit`, `waiting` is true and the current place is not `cockpit` (`TabBar.tsx:178`) |

