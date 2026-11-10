# Native title bar inventory: the bar, usage rings, context ring, build switcher, Fleet down

This is what the Electron and React Bridge draws today, so the native rebuild can match it.
It records what is, and proposes nothing.
It covers the title bar row and its window frame, the plan usage rings with the step-down offer, a Job's context ring with Compact, the Fleet build switcher with Keep and Revert, and what the shell draws when Fleet is down.
The places strip in the bar's centre is named where it sits and not described past that.

Paths are relative to the repository root.
`PC` is `packages/components/src`.
`TB` is `packages/components/src/compositions/TitleBar`.
`RS` is `apps/desktop/src/renderer/src`.
`MN` is `apps/desktop/src/main`.
Density tokens, and the 9 Nov "more room" tweak (larger padding and gaps, taller controls, rounded corners from the space, control-height and radius tokens), are as in [`../native-jobs-inventory/list-needs-palette.md`](../native-jobs-inventory/list-needs-palette.md).
Every pixel value below is Default's, resolved from the token named beside it.
Two tokens this file uses are not in that table: `--h-control-lg` is 32px Default, 28px Compact, 36px Roomy (`packages/tokens/src/spacing.css`:64, `spacing-sets.css:20`, `:41`), and `--h-title-bar` is 44px in every set (`spacing.css:70`).

## 1. The title bar row and the window frame

### 1.1 Where it lives

| What | File |
|---|---|
| The row: three regions, search trigger, Operator button, logo, Fleet dot | `TB/TitleBar.tsx` (`TitleBar` 512-764, `FleetDot` 766, `listed` 788, `RepositoryRows` 867) |
| The row's one menu (Account, Settings, Doctor, narrow list) | `TB/TitleMenu.tsx` (`TitleMenu` 35) |
| Row CSS | `TB/TitleBar.css` |
| Menu CSS | `TB/TitleMenu.css` |
| Shell: the row, the panel, the Operator dock, the status slot | `PC/compositions/TheShell/TheShell.tsx` (`TheShell` 98-175, `Dock` 238, `operatorButtonOf` 327) and `TheShell.css` |
| Shell adapter: builds the picker entries, menu and Operator prop | `packages/shell/src/Shell.tsx` (`Shell` 122, `<TheShell` 187-276) |
| Mount in the app | `RS/App.tsx:828` (`<Shell`), `{...titleRow}` at `:844`, `wordmark` at `:850`, `repository` at `:852-858`, `places` at `:859-870`, `fleet` at `:871-886` |
| Places strip drawn in the centre | `RS/cockpit/deck/TabBar.tsx` (`TabBar` 38), `deck-chrome.css:5-58` |
| Window flags | `MN/index.ts:326-330` (`titleBarStyle: "hiddenInset"`, `trafficLightPosition` x 18, y 16) |
| Key lookup for the bar's keys | `PC/actions` (`keyFor`), used at `TB/TitleBar.tsx:530`, `:744` |

### 1.2 Layout

```
.armada-shell                                           (TheShell.tsx:115)
  .armada-title-bar   [grid, 44px, drag region]         (TitleBar.tsx:535, TitleBar.css:10-35)
    .armada-title-bar__start      (traffic-light gutter 96px, then:)
      .armada-title-bar__wordmark   ArmadaMark 20px (App.tsx:850)
      .armada-title-bar__repository [data-picker]
        divider 1px x 16px
        .armada-title-bar__repository-chip: FolderGit2 glyph + repository dropdown
        Manifest icon button (ghost, 24px square)
    .armada-title-bar__center     (notch of glass; only with places)
      .armada-title-bar__places     TabBar (nav "Places")
      .armada-title-bar__search     Search glyph + label (hidden with places) + key
    .armada-title-bar__end
      .armada-title-bar__job [data-job-slot]     empty slot, takes no room when empty
      .armada-title-bar__usage     plan rings (section 2)
      .armada-title-bar__operator  reopen button, only while the dock is not open
      .armada-title-bar__lockup / __mark     omitted when a wordmark is passed
      .armada-title-bar__fleet     Fleet dot, button when a panel is passed
      .armada-title-bar__menu      Menu button
  .armada-shell__body
    .armada-shell__work
      .armada-shell__panel > .armada-shell__mount     the mounted screen, the only scroller
      .armada-shell__dock-layer     Operator dock over the work, when open
  .armada-shell__status      no height while empty
```

The app passes a wordmark, so the lockup and the small mark are not drawn there (`TB/TitleBar.tsx:653`, `RS/App.tsx:850`).
The Operator is a page and the first tab since 5 Nov, but the shell still receives `dock`.
Whether the reopen button draws is `operatorButtonOf` (`TheShell.tsx:327-355`): none while the full dock is open and not folded.
The `__job` slot is filled by the open Job's Layout control and view icons from the Job panes (`TitleBar.css:364-373`, `TitleBar.tsx:625`).

### 1.3 Sizes and tokens

| Part | Value | Where |
|---|---|---|
| Row | grid `minmax(var(--armada-title-start-floor), 1fr) minmax(min-content, auto) minmax(max-content, 1fr)` with places; without places `minmax(max-content,1fr) minmax(0,auto) minmax(max-content,1fr)` | `TitleBar.css:22`, `:268-273` |
| Row height | `--h-title-bar` = 44px | `TitleBar.css:26` |
| Row padding and gap | inline padding `--space-3` = 12px, column gap `--space-3` = 12px | `TitleBar.css:23`, `:27` |
| Row ground | transparent, no rule under it | `TitleBar.css:32` |
| Traffic lights | `hiddenInset`, cluster top-left at x 18, y 16 | `MN/index.ts:326-330` |
| Traffic-light gutter | `padding-inline-start: calc(var(--space-12) * 2)` = 96px, on the start region | `TitleBar.css:75` |
| Drag region | the row (`-webkit-app-region: drag`); `container-type: inline-size` | `TitleBar.css:33-34` |
| No-drag children | picker, repository, places, search, Operator button, menu, usage, job slot, Fleet dot | `TitleBar.css:43-57` |
| Narrow switch | container query `max-width: 900px` | `TitleBar.css:746` |
| Wordmark | `--text-sm` 14px, mono, `--weight-heading` 600, colour `--fg-default` | `TitleBar.css:135-144` |
| Repository name, read-only form | `--text-sm`, `--fg-subtle`, max width `--w-menu` = 240px, margin-inline-start `--space-3` | `TitleBar.css:147-156` |
| Repository picker form | margin-inline-start `--space-6` = 24px, gap `--space-2` = 8px, colour `--fg-default`; divider 1px wide by `--space-4` = 16px high in `--border-default` | `TitleBar.css:159-175` |
| Picker trigger and Manifest button | height `--h-control-sm` = 24px, 1px `--border-default`, radius `--radius-md` = 6px, fill `--bg-sunken`; trigger padding-left `--space-2 + --space-4 + --space-1` = 28px (room for the glyph), `--font-sans`, `--text-sm`, `--weight-medium`; Manifest button width `--h-control-sm`, colour `--fg-muted` | `TitleBar.css:196-221` |
| Picker glyph | `FolderGit2` 16px, colour `--accent`, at inset-inline-start `--space-2` | `TitleBar.css:187-195`, `TitleBar.tsx:556` |
| Picker states | hover: fill `--bg-hover`, border `--border-strong`; open: fill `--accent-muted`, border `--accent`; focus ring `--focus-ring` offset `--focus-ring-offset`; disabled Manifest colour `--fg-subtle` | `TitleBar.css:222-239` |
| Search trigger | width `--palette-width` = 560px, height `--h-control-lg` = 32px, padding-inline `--space-3`, gap `--space-2`, 1px `--border-default`, radius `--radius-md`, fill `--bg-sunken`, colour `--fg-subtle`, `--font-mono`, `--text-xs` 13px / `--leading-xs` 18px; hover fill `--bg-hover` over `--duration-fast` 120ms | `TitleBar.css:97-123` |
| Search trigger, with places | icon and key only: width auto, transparent border and fill, colour `--fg-muted`, hover colour `--fg-default`; label hidden | `TitleBar.css:334-352` |
| Notch (with places) | the centre stretches the full row height; radius `0 0 var(--armada-notch-radius) var(--armada-notch-radius)` where `--armada-notch-radius` = `--radius-lg * 2` = 16px; ground `--bg-glass`; shadow `--shadow-card`; side padding `--armada-notch-gap - 1px` where `--armada-notch-gap` = `(--h-title-bar - --h-control-lg) / 2` = 6px; top corners flared with two radial-gradient pseudo-elements 16px square | `TitleBar.css:284-323` |
| Places tab | height `--h-control-lg` 32px, padding-inline `--space-2`, `--font-mono`, `--text-2xs` 12px, gap `--space-1`; chosen tab weight `--weight-heading`, fill `color-mix(--fg-default 8%, transparent)`; hairline after the strip 1px by `--space-4 + --space-1` = 20px | `deck-chrome.css:12-58` |
| Tab tick | 1px `--border-default`, inset-block 30%, between tabs, none beside the chosen tab | `TitleBar.css:249-262` |
| Tooltip under the notch | below the notch, margin-block-start `--armada-notch-gap + --space-1` | `TitleBar.css:326-331` |
| End region | flex, gap `--space-3`, justify end | `TitleBar.css:356-362` |
| Operator button | height `--h-control-lg` 32px, padding-inline `--space-2`, gap `--space-2`, 1px `--border-default`, radius `--radius-md`, fill `--bg-raised`, `--text-xs`; glyph `Bot` 16px; hover fill `--bg-hover` | `TitleBar.css:540-566`, `TitleBar.tsx:640` |
| Fleet dot target | wrapper padding `--space-1` = 4px around a `--dot` = 6px circle, radius 50% | `TitleBar.css:585-617`, `spacing.css:433` |
| Menu button | ghost `Button`, size sm, icon only, `Menu` glyph 16px | `TitleMenu.tsx:91-103` |
| Menu panel | width `--space-12 * 5` = 240px, padding `--space-2`, entries gap `--space-1` | `TitleMenu.css:31-42` |
| Menu item | height `--h-control` = 28px, padding-inline `--space-2`, gap `--space-3`, radius `--radius-md`, `--text-sm` 14px / 20px, `--weight-medium`; hover fill `--bg-hover`; focus outline offset `-2px` | `TitleMenu.css:52-83` |
| Menu glyph chip | `--space-6 + --space-1` = 28px square, radius `--radius-md`, fill `--accent-faint`, colour `--accent-hover` | `TitleMenu.css:86-96` |
| Narrow-menu item | height `--h-menu-item` = 28px, padding-inline `--space-2`, gap `--space-2`, radius `--radius-sm` = 3px, `--text-xs` | `TitleBar.css:701-716` |
| Shell body | work margin-left `--pad-page` = 12px; panel margin-right `--pad-page` | `TheShell.css:18-31` |
| Operator dock | width `--w-dock` = 380px, floor `--w-dock-min` = 320px, margin `--space-4` = 16px, radius `--radius-lg` = 8px, layer `z-index: --z-dock` = 20, over the work and not a column | `spacing.css:225`, `:240`, `:394`, `TheShell.css:43-110` |

Literals in the row: the `+ 1` pixel in the measuring code (`TitleBar.tsx:475`) and `0.5px` in the notch gradients (`TitleBar.css:312`, `:321`).

### 1.4 Behaviour that depends on width

| Rule | Value | Where |
|---|---|---|
| Below 900px of row width the start, centre, Operator button and lockup are hidden; the small mark and the narrow list in the menu show; the end region takes the full row | container query | `TitleBar.css:746-762` |
| With places, the centre stays below 900px, in column 1, and the end region takes columns 2 onward | same | `TitleBar.css:765-772` |
| The places draw as glyphs when the row cannot hold their labels | `usePlacesRoom()` reads the room left after the sides, gaps and trigger | `TitleBar.tsx:433-510` |
| The first column's floor is measured and written to `--armada-title-start-floor` | ResizeObserver on row, start, places, search and end | `TitleBar.tsx:457-497` |
| The Operator dock folds to a sheet at `--layout-breakpoint` 1100px | `folded: narrow` from `useNarrow()` | `spacing.css:125`, `packages/shell/src/Shell.tsx`:209 |

### 1.5 Controls

| Control | Label, role and name | What a press does | Shown when |
|---|---|---|---|
| Repository picker | `DropdownMenu`, trigger label is the repository name, aria `Repository`, search field aria `Find a repository` | choosing an entry calls `onSelect(id)`: All, a repository, a Workspace heading, `add-repository`, `new-project`, `new-workspace` (`packages/shell/src/Shell.tsx` `pick`); no Fleet call from the row | `repository` and `repositoryMenu` passed (`TitleBar.tsx:546-590`) |
| Picker rows | glyph `Layers` for All and a Workspace, `FolderGit2` for a repository; acts below the rule use `FolderPlus` and `Plus` in the accent colour; row height `--space-8` = 32px; a repository not set up carries the detail `Not set up` | as above | `TitleBar.tsx:775-864`, `TitleBar.css:775-800` |
| Workspace gear | `Settings` 16px, `role="button"`, aria and tooltip `Open <workspace name>` | `onOpenWorkspace(id)` | a Workspace heading row (`TitleBar.tsx:818-844`) |
| Repository place glyph | trailing mark, 24px square, `--radius-md` | `trail.open()` from `repositoryTrail` | `trailOf` returns one (`TitleBar.tsx:845-861`) |
| Manifest button | tooltip and aria `Open the Manifest`, glyph from the surface roster (`FileCog` fallback), disabled when no repository is read | `onOpenManifest` goes to `SURFACE.manifest` | `repositoryMenu.manifest` passed (`packages/shell/src/Shell.tsx`:232-240) |
| Places | `nav` aria `Places`, tabs with glyph and name: Operator, Cockpit, Jobs, Studios, Logbook, Reviews, Issues, Projects, System | `goTo(surface)` | `packages/shell/src/surfaces.ts`:419-470, `TabBar.tsx:166-196` |
| Places menu | right press, ContextMenu key or Shift+F10: `Hide <tab>`, then a `View` label and one checkable row per place; Cockpit and System carry the hint `Always shown` and are disabled | writes the `rail` layout (`layouts.source.setOwn("rail", ...)`), a local setting | `TabBar.tsx:103-132`, `:218-229` |
| Search trigger | button; name is `Search, or describe work to dispatch`, and with places the name and the `title` are `Search, or describe work to dispatch (⌘K)`; key drawn by `KbdBinding` | `onSearch` opens the command palette (`palette.onOpen`, `App.tsx:843`) | `onSearch` passed (`TitleBar.tsx:602-621`) |
| Operator reopen | button; `title` is `Open Operator — ⌘J` (title word and binding come from the dock); aria `Operator`; glyph `Bot` 16px; the `⌘J` badge mounts only while Cmd is held | `operator.onOpen` calls `dock.onOpen(true)` | `operatorButtonOf` returns one (`TitleBar.tsx:628-650`) |
| Fleet dot | see section 5.2 | opens the Fleet popover | always (`TitleBar.tsx:668-705`) |
| Menu button | `aria-label` `Menu`, `aria-expanded`, `title` is `Menu`, then `, signed out` when no profile, then the account need in lower case (`Waiting for GitHub`, `Setting up`, `Not set up`, an ended sign-in's word) | toggles the menu; Esc and a press outside close it | `TitleMenu.tsx:74-110` |
| Menu: Account or Sign in | row aria `Sign in` when no profile, else `Account`; glyph `UserCheck` | `onAccount` goes to `SURFACE.account` | `account` present (`TitleMenu.tsx:119-144`) |
| Menu: Settings | aria `Settings`, glyph `Settings`, key from the roster (`⌘,`) | goes to `SURFACE.settings` | `settings` passed (`TitleMenu.tsx:145-161`) |
| Menu: Doctor | aria `Doctor`, glyph `Stethoscope` | goes to `SURFACE.doctor` | `doctor` passed (`TitleMenu.tsx:162-177`) |
| Menu, narrow list | `Repositories` group with a `Find a repository` searchbox, then `Search` (glyph `Search`, key), `Dispatch` (glyph `Plus`, key `⌘N`, disabled when `dispatchDisabled`), and `Operator` (glyph `Bot`, `⌘J`) | `onSearch`, `onDispatch` (`onCompose`, `newSession()`), `operator.onOpen` | only below 900px (`TitleBar.tsx:725-760`, `TitleMenu.css:44-50`) |
| Teach menu | right press on the bar, or Shift+F10 in it | `onTeach` calls `teachBridge("title_bar", "")` (`App.tsx:848`) | `teachBridge` defined (`TitleBar.tsx:532`, `:538-541`) |

`dispatchDisabled` is `!live`, with `live` true only for `connection.state === "connected"` (`packages/shell/src/Shell.tsx`:154, `:272`).
A press on a repository entry, Search, Dispatch or Operator is a window act; none sends a Fleet request from the row.

### 1.6 States

| State | Drawn as | Where |
|---|---|---|
| No `places` | search trigger shows its label at the full 560px; no notch | `TitleBar.tsx:608-613`, `TitleBar.css:97` |
| Menu with no profile | amber dot, `--space-2` = 8px, at the button's bottom-right inset `--space-1`, colour `--status-awaiting-review` | `TitleMenu.css:9-18` |
| Menu with an account need | 12px glyph badge from `accountNeed`; a turning one rotates | `TitleMenu.tsx:105-109`, `Account.tsx:340-397` |
| Search key unbound | key not drawn | `TitleBar.tsx:619` |
| Cmd held | the Operator button shows its `⌘J` badge and widens | `TitleBar.tsx:644-648` |
| Keyboard focus | ring `--focus-ring`, offset `--focus-ring-offset` (2px) on search, Operator, Fleet button, usage meter | `TitleBar.css:120-123`, `:393-396`, `:563-566`, `:603-606` |
| Reduced motion | ring sweep becomes a pulse | `TitleBar.css:445-449` |

### 1.7 Keys

| Act | Key | Scope | Where |
|---|---|---|---|
| `command_palette`, Command palette | ⌘K | anywhere | `crates/core-model/domain/actions.toml`:58-65 |
| `operator`, Operator (glyph `bot`) | ⌘J | anywhere | `actions.toml:70-76` |
| `new_job`, Dispatch (glyph `plus`) | ⌘N | anywhere | `actions.toml:340-346` |
| `deck_tabs`, Go to a tab | ⌘1 to ⌘9, the nth drawn tab | anywhere | `actions.toml:1028-1035` |
| `open_settings`, Open Settings (glyph `settings`) | ⌘, | anywhere | `actions.toml:1040-1046` |
| `refresh`, Refresh | ⇧⌘R | anywhere; not drawn on the bar | `actions.toml:268-275` |

The bar's own controls (picker, Manifest button, Menu, Fleet dot, usage meter) have no act row in `actions.toml`.

### 1.8 Copy strings

`Search, or describe work to dispatch`, `Search, or describe work to dispatch (⌘K)`, `Repository`, `Find a repository`, `Not set up`, `Open the Manifest`, `Open <name>`, `Nothing matches “<query>”.`, `Repositories`, `Search`, `Dispatch`, `Operator`, `Open Operator — ⌘J`, `Menu`, `Menu, signed out`, `Sign in`, `Account`, `Settings`, `Doctor`, `Places`, `Always shown`, `View`, `Hide <tab>`.
The picker's All entry reads `All repositories`, or `Nothing set up yet` or `No repository read yet` when no repository is listed (`packages/shell/src/Shell.tsx`:162-164).
The wordmark image title is `Armada` (`RS/App.tsx:850`).

## 2. Usage rings and the step-down offer

### 2.1 Where it lives

| What | File |
|---|---|
| Rings, loading ring, card (`PlanUsageFigures` 233, `PlanUsageLoading` 199, `LimitSection` 133, `LIMITS` 96, `countdown` 115) | `TB/TitleBar.tsx`, CSS `TB/TitleBar.css:375-538` |
| Level thresholds | `PC/usage.ts:1-6` (`usageLevel`) |
| Source of the windows (context, hook) | `RS/plan-usage.tsx:1-43` (`planUsageDrawn` 23, `usePlanUsage` 38) |
| Wire reader | `RS/fleet-plan-usage.ts:19-64` (`createFleetPlanUsage`) |
| Title row hook that joins the two | `RS/title-row.ts:9-18` (`useTitleRow`), called at `RS/App.tsx:306` |
| Step-down state and offer card | `RS/downshift.tsx` (`offerOf` 53, `useDownshiftOffer` 113, `DownshiftAlert` 141) |
| Step-down wire reader and four acts | `RS/fleet-downshift.ts:25-74` (`createFleetDownshift`) |
| Weekly figure used again on the Usage page | `RS/usage-surface.tsx:73` (`planPercent`) |
| Main process requests | `MN/plan-usage.ts:54` (`GET /plan_usage`), `MN/model-downshift.ts:44-60` |
| IPC channels | `apps/desktop/src/shared/api/plan-usage.ts`:25 (`bridge:get-plan-usage`), `apps/desktop/src/shared/api/model-downshift.ts`:34 |
| Fleet routes and events | `crates/api/src/routes.rs`:360-365, `crates/ipc/src/event.rs`:192 (`model_downshift.changed`), `plan_usage.changed` named at `RS/fleet-plan-usage.ts:1` |

### 2.2 Layout

```
.armada-title-bar__usage                         (end region, no-drag)
  Inspect "Plan limits"                          hover or focus opens, a press keeps open
    button.armada-title-bar__usage-meter [aria-label "5-hour limit 42%, Weekly limit 71%"]
      svg 22 x 22
        outer ring (first limit, r 9) track + fill
        inner ring (second limit, r 5) track + fill
      BellRing 12px                              only while an offer waits (tooltip: the attention text)
    InspectCard  kind "Plan", title "Plan limits", key Enter "Keep open", wide
      section "5-hour limit"   name .. percent (mono, in tone)
        dl facts: "Resets in" 2h 14m
        acts row (Buttons, size sm)
      section "Weekly limit"   (same, plus the offer's facts and acts)
```

### 2.3 Sizes and tokens

| Part | Value | Where |
|---|---|---|
| Glyph | svg 22 x 22, centre 11, outer radius 9, inner radius `9 - 4` = 5, stroke 2.5 | `TitleBar.tsx:280-298`, `TitleBar.css:398-402` |
| Track | stroke `--bg-hover` | `TitleBar.css:404-406` |
| Fill | stroke `--tone`, round caps, `strokeDasharray` `<percent> 100` over `pathLength` 100, rotated -90 degrees so it starts at twelve o'clock | `TitleBar.tsx:287-295`, `TitleBar.css:408-411` |
| Tone | `--fg-muted` below 80, `--status-awaiting-review` from 80, `--status-completed-failed` from 95 | `PC/usage.ts:4-5`, `TitleBar.css:451-465` |
| Loading glyph | the same two tracks and an arc of `25 75` on each; outer clockwise, inner reversed; stroke `--fg-muted`; period `--duration-pulse * 1.25` = 2000ms linear | `TitleBar.tsx:204-221`, `TitleBar.css:413-430` |
| Bell | `BellRing` 12px, stroke 2, colour `--status-awaiting-review`, gap `--space-1` after the ring | `TitleBar.tsx:300-306`, `TitleBar.css:386`, `:468-472` |
| Card width | `wide` = `--w-tooltip` = 340px | `PC/primitives/Inspect/Inspect.css:72-74`, `spacing.css:374` |
| Limit section | flex column, gap `--space-1`, padding-block `--space-2`; a second section has a top hairline `--border-subtle` | `TitleBar.css:476-485` |
| Section head | `--text-xs` 13px / 18px, `--weight-heading` 600, `--fg-default`, gap `--space-3`; percent right-aligned, `--font-mono`, colour `--tone` | `TitleBar.css:487-502` |
| Facts | grid `auto 1fr`, gap 2px by `--space-3`, `--text-2xs` 12px / 16px; term `--fg-subtle`, value `--fg-default` right-aligned, tabular figures, mono when flagged | `TitleBar.css:504-531` |
| Acts | flex wrap, gap `--space-1`, margin-top `--space-1`; `Button` size sm, primary or secondary | `TitleBar.css:533-538`, `TitleBar.tsx:175-190` |

### 2.4 Controls

| Control | Name | What a press sends | Shown when |
|---|---|---|---|
| Meter button | aria `<limit name> <rounded percent>%` joined by `, ` (for example `5-hour limit 42%, Weekly limit 71%`) | opens or pins the card; no Fleet call | at least one window of kind `five_hour` or `seven_day` |
| Loading glyph | `role="img"`, aria and tooltip `Analyzing Usage`; not a control, nothing opens | none | `planUsage` is an empty list (`TitleBar.tsx:196`, `:245`) |
| Bell | `role="img"`, aria and tooltip = the offer's `attention` text | none | a limit's `offer.attention` is set |
| Card act `Default to <to>` (primary, stays open) | button | `source.accept` calls `POST /model_downshift/accept` (`MN/model-downshift.ts:48`); the card stays open and shows the result | phase `offered` |
| Card act `Choose…` | button, secondary | `source.settle()` then `openModelSetting()`; settle is `POST /model_downshift/settle` (`model-downshift.ts:60`); the card closes | phases `offered` and `reset` |
| Card act `Back to <was>` (primary) | button | `source.restore` calls `POST /model_downshift/restore` (`model-downshift.ts:52`) | phase `reset` |
| Card act `Keep <to>` | button, secondary | `source.keep` calls `POST /model_downshift/keep` (`model-downshift.ts:56`) | phase `reset` |
| Toast, offered: action `Default to <to>` | `Toast` action; pressing the sentence runs settle then the model setting | accept as above | phase `offered`, `RS/downshift.tsx:148-161` |
| Toast, reset: action `Back to <was>` | `Toast` action | restore as above | phase `reset`, `downshift.tsx:162-168` |

A refusal from Fleet on any of the four acts is not shown; the reader reads the state again (`RS/fleet-downshift.ts:38-41`).
Fleet refuses an act made in the wrong phase with 409 `fleet.downshift_wrong_phase` (`crates/fleet/src/model_downshifting.rs`:24).
Hover or focus opens the card, a press keeps it open, and Esc or a press outside closes it (`TitleBar.tsx:228-232`).

### 2.5 States

| State | Drawn as | Where |
|---|---|---|
| Loading: no window drawn yet | the sweeping glyph, tooltip `Analyzing Usage` | `TitleBar.tsx:196-245` |
| No reading: account off a subscription (`windows` empty and `read_at` set) | `planUsageDrawn` returns `undefined`, so `planUsage` is absent and nothing draws | `RS/plan-usage.tsx:23-26` |
| No source | nothing draws | `RS/plan-usage.tsx:14-18`, `:38-42` |
| A kind other than `five_hour` and `seven_day` (for example `spend_limit`) | not drawn | `TitleBar.tsx:234-245`, `crates/ipc/src/plan_usage.rs`:11-12 |
| Normal, warning, danger | arc and percent in the tones of 2.3 | `TitleBar.tsx:285`, `:158` |
| Offered | bell with tooltip `Default to <to> offered`; fact `Default model` = `<was>`; acts `Default to <to>`, `Choose…` | `downshift.tsx:58-77` |
| Shifted | no bell, no acts; fact `Default model` = `<to> until <Stamp>`, the stamp being the window's reset | `downshift.tsx:78-88` |
| Reset | bell with tooltip `Back to <was> offered`; fact `Default model` = `<to>`; acts `Back to <was>`, `Keep <to>`, `Choose…` | `downshift.tsx:89-109` |
| Where the offer sits | only on the `seven_day` row | `downshift.tsx:127-134` |

### 2.6 Keys

No act row in `actions.toml` names the rings, the card or the offer.
The card names one key, `Enter` with the verb `Keep open` (`TitleBar.tsx:259-264`).

### 2.7 Copy strings

`5h` and `5-hour limit`, `wk` and `Weekly limit` (`TitleBar.tsx:104-111`), `Plan limits`, `Plan`, `Keep open`, `Resets in`, `Analyzing Usage`, `Default model`, `Default to <to>`, `Default to <to> offered`, `Back to <was>`, `Back to <was> offered`, `Keep <to>`, `Choose…`, `Weekly limit <n>% used`, `Weekly limit reset`.
The countdown shows the two largest units: `2d 17h`, `2h 14m`, `9m` (`TitleBar.tsx:114-122`).

## 3. A Job's context ring with Compact

### 3.1 Where it lives

| What | File |
|---|---|
| Ring, card, popover, tooltip forms | `PC/compositions/SessionFrame/ContextFigure.tsx` (`ContextFigure` 283, `Ring` 245, `BreakdownCard` 137, `CompactButton` 31, `tokensSaid` 50, `thresholdOf` 237) |
| Ring and bar CSS | `PC/compositions/SessionFrame/SessionFrame.css` (figure 160-196, bar 202-236, ring 402-440, foot 441, popover 447) |
| Mount in the Job's head | `PC/compositions/SessionFrame/SessionFrame.tsx:695-703` |
| Breakdown shape | `PC/context-breakdown.ts` |
| Compact act | `RS/cockpit/deck/compact-act.ts:11-26` (`useCompactAct`), `RS/cockpit/deck/session-tiles.ts:287-343` (`offeredOf`), called at `RS/sessions.tsx:2754` |
| Wire to view model | `packages/screens/src/sessions-wire.ts`:1850-1868 |
| The word "Compact" in the Aa panel | `PC/compositions/ThreadSettings/ThreadSettings.tsx:36-40` is the text spacing choice `Compact`, a different thing from this act |

### 3.2 Layout

```
.armada-session-frame__context-host [data-level]      (in the Job head, margin-inline-start: auto in the tools row)
  Inspect "Context"                                    (when a breakdown is known)
    button.armada-session-frame__context [aria-label "Context 63%"]
      svg.armada-context-ring 16 x 16 (viewBox 16)
        circle track r 5.25
        circle arc   r 5.25, from twelve o'clock
        line tick    at the autocompact threshold
      span.armada-context-ring__percent   "63%"
    InspectCard  kind "Context", where "126k / 200k", title "Context window", state "63% used"
      .armada-context-bar              one segment per used category, then free, then buffer
      facts: each category, mono
      section "Memory files"           heaviest 3
      section "MCP tools"              heaviest 3 servers
      .armada-context-foot > Compact   when the act is offered
```

Three forms, chosen in `ContextFigure` (`ContextFigure.tsx:283-349`):

| Form | When | Drawn |
|---|---|---|
| Card | `fill.breakdown` is set | button opening the Inspect card above |
| Popover | no breakdown and a Compact act is offered | button opening a `Popover` labelled `Context` with the fill text and the Compact button |
| Tooltip only | no breakdown and no Compact | `role="img"` figure with tooltip `Context · <tokens> of <window> tokens` |

Nothing draws when the window is 0 or less (`ContextFigure.tsx:284`).

### 3.3 Sizes and tokens

| Part | Value | Where |
|---|---|---|
| Ring | `--space-4` = 16px square, stroke 2, no fill; r 5.25 in a 16 viewBox | `SessionFrame.css:411-417`, `ContextFigure.tsx:257-267` |
| Track | stroke `--border-default` | `SessionFrame.css:419-421` |
| Arc | stroke `--tone` (default `--fg-muted`), `strokeDasharray` `<tokens / window, clamped 0 to 1, x 100> 100`, transition of the dash over `--duration-fast` 120ms, none under reduced motion | `SessionFrame.css:423-440`, `ContextFigure.tsx:265` |
| Tick | line from radius 3.5 to 7 at angle `threshold * 2 * PI`; stroke `color-mix(--fg-subtle 60%, transparent)`, width 1 | `ContextFigure.tsx:247-269`, `SessionFrame.css:429-433` |
| Percent | `--font-mono`, `--text-xs` 13px / 18px, `--weight-medium`, colour `--tone`, gap `--space-1` after the ring | `SessionFrame.css:160-167`, `:402-409` |
| Tone | `--fg-muted` below 80, `--status-awaiting-review` from 80, `--status-completed-failed` from 95 | `PC/usage.ts:4-5`, `SessionFrame.css:178-196` |
| Bar | height `--space-1` = 4px, margin-block `--space-1`, radius `--radius-sm` = 3px, gap 1px; segments step down from the tone: 100%, 80%, 65%, 50%, 40%, 30%; free `--bg-sunken`; buffer `--border-default` | `SessionFrame.css:202-236` |
| Card | `wide` = `--w-tooltip` 340px | `ContextFigure.tsx:182` |
| Foot | flex, justify end, padding-top `--space-2` | `SessionFrame.css:441-445` |
| Popover body | flex, gap `--space-4`, padding `--space-3`; fill text mono `--text-xs`, `--fg-muted` | `SessionFrame.css:447-460` |

### 3.4 Controls

| Control | Name | What a press sends | Shown when |
|---|---|---|---|
| Ring button | aria `Context <percent>%` | opens or pins the card or popover | a window is known |
| Compact | button, secondary, size sm, label `Compact`; tooltip `Compact the Job's context`, or the reason it is off | `compact.onPress()`, then closes the popover | `compact` is passed |
| Compact, terminal Job | same | `draft.command(session.id, words)` calls `runSessionCommand`, then `POST /sessions/command` with `{session_id, command}` (`RS/sessions-wired.tsx:1615`, `MN/session-channels.ts:770-777`, `MN/sessions.ts:1459-1461`); Fleet route `crates/api/src/routes.rs`:544 | the Job is a terminal and its commands list `compact` |
| Compact, hosted Job | same | `draft.send(session.id, plain(words))` calls `sendSessionMessage`, then `POST /sessions/message` (`RS/sessions-wired.tsx:902`, `MN/session-channels.ts:332-335`, `MN/sessions.ts:1148-1150`); Fleet route `crates/api/src/routes.rs`:512 | the Job's draft commands list `compact` |

`words` is the `compact` flight tile's words from the owner's flight file, or `/compact` (`RS/cockpit/deck/compact-act.ts:12-13`, `session-tiles.ts:166`).
The message is a plain one with no files, sketches or tags (`session-tiles.ts:169-174`).

### 3.5 States

| State | Drawn as | Where |
|---|---|---|
| Normal, warning, danger | ring and percent in the tones of 3.3 | `ContextFigure.tsx:289`, `:320`, `:328` |
| No autocompact threshold, or at or past the window | no tick | `ContextFigure.tsx:237-242` |
| Compact off | the button disabled, tooltip is the reason | `ContextFigure.tsx:33-37` |
| Reason: the Job ended | `The Job ended` | `session-tiles.ts:177-179`, `:316` |
| Reason: terminal gone quiet | `The terminal is not listening` | same |
| Reason: terminal | `The terminal has not listed compact` | `session-tiles.ts:330` |
| Reason: hosted | `The agent has not listed compact` | `session-tiles.ts:342` |
| Card, no memory files or MCP tools | the section draws with no facts | `ContextFigure.tsx:198-226` |
| Reduced motion | arc has no transition | `SessionFrame.css:435-439` |

### 3.6 Keys

| Act | Key | Scope | Where |
|---|---|---|---|
| `deck_tile`, Press a flight control | ⌥⇧0 to ⌥⇧9; while a Job is open the tiles are, in default order, Ask for update, Auto land, Compact, Report, Map, Judge, Release, Artifacts, Subagents, Checks, so Compact is third by default | deck | `crates/core-model/domain/actions.toml`:549-559 |
| The ring and its card | no act row; card key `Enter` with verb `Keep open` | none | `ContextFigure.tsx:176-181` |

### 3.7 Copy strings

`Context`, `Context <n>%`, `Context window`, `<n>% used`, `Context · <tokens> of <window> tokens`, `<tokens> of <window> tokens`, `Memory files`, `MCP tools`, `Keep open`, `Compact`, `Compact the Job's context`.
Token figures read `53.8k`, `5.0k`, `142k`, `1M` (`ContextFigure.tsx:49-55`).

## 4. The Fleet build switcher

### 4.1 Where it lives

| What | File |
|---|---|
| Section drawn inside the Fleet popover | `PC/compositions/FleetBuild/FleetBuild.tsx` (`FleetBuildSection` 204-546), CSS `FleetBuild.css` |
| Build report to view model | `RS/fleet-build.ts:92-187` (`fleetBuildOf`) |
| Window wiring | `RS/App.tsx:317-331` (builds `fleetBuild`), `:332` (`useSnapshotChime`), `:881` (passes `build`), `:571` (`useBuildFailed`) |
| Keep and Revert bar | `RS/trying-bar.tsx:8-31`, `trying-bar.css`, mounted at `RS/App.tsx:899-903` |
| A restart handed over as a command | `RS/build-change.ts:13-57` (`scripts/preview --restart`, `scripts/restart --main`) |
| Dim and banner while Fleet swaps | `RS/swap-hold.tsx`, mounted at `RS/App.tsx:826` |
| Main process requests | `MN/fleet-build.ts` (`GET /fleet/build` 64, `POST /fleet/build/change` 98, `/revert` 132, `/pick` 163, `/swap` 184, `/keep` 210), channels `apps/desktop/src/shared/api/core.ts`:66-90, `:210-215`, handlers `MN/index.ts:1066-1078` |
| Report shape | `crates/ipc/src/building.rs`:49-110 (`FleetBuildReport`) |

### 4.2 Layout

The switcher is the first block in the Fleet popover's body (`FleetPanel.tsx:196-198`); the popover opens from the dot (section 5.2).
It is stacked, because the panel is narrow (`FleetBuild.css:1-3`).

```
.armada-fleet-build  (column, gap --space-2, bottom hairline --border-subtle)
  Select "Build"  [Main | Preview]                    tooltip "Build Fleet runs on"
  Button  [RotateCw | LoaderCircle] "Refresh preview" | "Update to main" | "Refreshing preview" | "Updating to main"
  Button  [Undo2 | LoaderCircle] "Revert" | "Reverting"          only with a previous build
  "Next build holds" + chips (name + X)              only with picked Jobs
  snapshot row: Building <stage> | failed line | Ready + Jobs + "Bridge reopens" + Swap
  failed line (role alert)                           a restart that did not take, not while restarting
  left-out list                                      Preview only, nothing restarting
  position row: "main" label + marks | restarting mark with stage
  Dialog "Revert to the build of <built>"
  Dialog "Drones are working"
```

The Keep and Revert bar is separate: one line across the top of the mounted screen, above the Fleet notice (`RS/Standing.tsx:70-72`).

### 4.3 Sizes and tokens

| Part | Value | Where |
|---|---|---|
| Section | column, gap `--space-2` = 8px, padding-bottom `--space-2`, margin-bottom `--space-1`, bottom rule `--border-subtle` | `FleetBuild.css:4-12` |
| Select and Buttons | full panel width, content centred; Buttons size sm | `FleetBuild.css:14-22` |
| Mark | mono, `--text-xs` 13px / 18px, colour `--fg-muted`; success `--status-completed-success`, warn `--status-awaiting-review`; glyphs 12px, stroke 2, gap `--space-1 / 2` | `FleetBuild.css:100-123` |
| Restart spinner | 12px `LoaderCircle`, one turn per `--duration-pulse` 1600ms, still under reduced motion | `FleetBuild.css:125-140` |
| Failed line | `--text-xs`, colour `--status-escalated`, wraps anywhere | `FleetBuild.css:24-31` |
| Left-out row | grid `auto 1fr 12px`, gap `--space-2`; branch mono; reason `--fg-muted`, one line with ellipsis, whole text in a tooltip | `FleetBuild.css:33-78` |
| Position row | flex, space-between, gap `--space-2`; label colour `--text-label` | `FleetBuild.css:80-98` |
| Trying bar | flex, space-between, gap `--space-3`, padding `--space-1` by `--space-3` (4px by 12px), fill `--bg-sunken`, bottom border `--border-default`, `--text-sm` 14px / 20px | `RS/trying-bar.css:2-14` |
| Trying acts | gap `--space-2`; text one line with ellipsis | `trying-bar.css:16-27` |
| Popover | panel min-width `--sidebar-default` = 200px, max-width `--palette-width` 560px, padding `--space-3`, radius `--radius-lg`, fill `--bg-overlay`, `z-index: --z-menu` = 40 | `PC/primitives/Popover/Popover.css:19-31`, `spacing.css:95`, `:395` |

### 4.4 Controls

| Control | Name | What a press sends | Shown when |
|---|---|---|---|
| Build select | `Select`, aria `Build`, options `Main`, `Preview`; tooltip `Build Fleet runs on`; disabled while a restart works | choosing the other build calls `changeFleetBuild(build, adopt)`, which is `POST /fleet/build/change` (`MN/fleet-build.ts:98`, `MN/index.ts:1066`); with working Drones listed it asks first | the report is non-null (`fleetBuildOf` returns `undefined` for `null`, `fleet-build.ts:98`) |
| Restart button | label and tooltip per 4.5 | `build.onRestart(adopt)` calls `change(report.on, adopt)`: on Preview it merges in-flight branches again and restarts, on Main it moves to the latest main | with the section; disabled when idle on an aligned main, while a restart works, a revert or a swap is under way |
| Revert | `Undo2` glyph, label `Revert` or `Reverting`; tooltip `Back to the build of <built>, <commit>` | opens the Revert dialog; confirming calls `revertFleetBuild()`, which is `POST /fleet/build/revert` (`MN/fleet-build.ts:132`) | `previous` is set |
| Revert dialog | title `Revert to the build of <built>`, confirm `Revert` | as above | `FleetBuild.tsx:480-526` |
| Drones dialog | title `Drones are working`, confirm `Restart` | sends the held act with `adopt` true | `drones` is non-empty; `fleetBuildOf` always sets `drones: []` (`fleet-build.ts:112`), so it does not open |
| Leave out chip | aria `Leave <name> out of the build`, tooltip `Leave out of the build` | `onUnpick(branch)` calls `pickFleetBuildJob({branch, picked:false})`, which is `POST /fleet/build/pick` (`MN/fleet-build.ts:163`, `App.tsx:324-329`) | picked Jobs exist |
| Swap | label `Swap`, `Swapping` while pending | `swapFleetBuild()` is `POST /fleet/build/swap` (`MN/fleet-build.ts:184`) | snapshot `ready` |
| Keep (trying bar) | `Keep`, secondary, size sm | `keepFleetBuild()` is `POST /fleet/build/keep` (`MN/fleet-build.ts:210`, `MN/index.ts:1078`); a refusal is raised as a toast through `settleFleetBuild` | `trying` is non-empty |
| Revert (trying bar) | `Revert`, secondary, size sm | `revertFleetBuild()` as above, with no confirm | same |
| Refresh preview from a Job | a waiting item whose run command is `scripts/preview --restart` | `buildChangeOf` maps it to `changeFleetBuild("preview", adopt)`; `scripts/restart --main` maps to `main` | `RS/build-change.ts:13-45` |

### 4.5 States

| State | Drawn as | Where |
|---|---|---|
| Label and tooltip, idle on Preview | `Refresh preview`, tooltip `Merge in-flight branches and restart on the preview` | `FleetBuild.tsx:218-230` |
| Label and tooltip, idle on Main | `Update to main`, tooltip `Restart on latest main` | same |
| Label, restarting | `Refreshing preview` or `Updating to main`, `LoaderCircle` replaces `RotateCw`, button pending | `FleetBuild.tsx:260-277` |
| Idle on an aligned main | button disabled, tooltip `On latest main` | `FleetBuild.tsx:217`, `:226` |
| Position, aligned | `Check` 12px green, tooltip and aria `Aligned with main` | `FleetBuild.tsx:436-446` |
| Position, ahead | `ArrowUp` 12px and the count, muted, tooltip `<n> commit(s) ahead of main` | `FleetBuild.tsx:449-461`, `:172-176` |
| Position, behind | `ArrowDown` 12px and the count, amber, tooltip `<n> commit(s) behind main` | `FleetBuild.tsx:462-474`, `:177` |
| Position not counted | no mark | `FleetBuild.tsx:434` |
| Restarting | `LoaderCircle` spinning and the stage word; tooltip `Restarting Fleet on the preview` or `Restarting Fleet on main`; aria `Restarting Fleet` when no stage | `FleetBuild.tsx:411-433` |
| Stages | `Merging branches`, `Fetching main`, `Building Fleet`, `Building Bridge`, `Restarting Fleet`, `Reopening Bridge`, `Retrying build` | `FleetBuild.tsx:39-47` |
| Left out (Preview only) | `CircleSlash` 12px amber, tooltip and aria `Left out of the preview`; branch name; `GitCommitHorizontal` with `Moved since it was left out` when moved; the script's reason | `FleetBuild.tsx:374-408` |
| Snapshot building | `LoaderCircle` and `Building` with the stage | `FleetBuild.tsx:325-337` |
| Snapshot failed | the failed line, `role="alert"` | `FleetBuild.tsx:338-341` |
| Snapshot ready | green `Check` and `Ready`, the Jobs held, `Bridge reopens` when it will, and `Swap` | `FleetBuild.tsx:342-367` |
| Restart failed | red line with the script's one-line reason, hidden while a restart works | `FleetBuild.tsx:369-373` |
| Revert dialog rows | `Running now`, `Goes back to`, `Jobs taken out`, `Jobs brought back`, `Restarts` (`Fleet, and this window` or `Fleet. This window stays open`), `Jobs still working` | `FleetBuild.tsx:492-523` |
| Swap under way | the window is dimmed and takes no presses, with an `Alert` titled `Swapping to the build` showing `<n> s` | `RS/swap-hold.tsx:57-90` |
| Swapped back | `Alert` tone caution, title `Swapped back`, action `Dismiss` | `RS/swap-hold.tsx:88-98` |
| Trying | `Trying <title>, <title>` | `RS/trying-bar.tsx:20` |
| Chime | a sound plays once when the snapshot turns ready | `RS/fleet-build.ts:77-89` |

The section is not drawn when `build` is undefined (`FleetPanel.tsx:198`); `build` is undefined when the report is `null`, which is when Fleet offers none.

### 4.6 Keys

No act row in `actions.toml` names the build select, Refresh preview, Update to main, Revert, Keep or Swap.
`refresh` (⇧⌘R) re-reads what Fleet holds and is not this act (`actions.toml:268-278`).

### 4.7 Copy strings

`Build`, `Build Fleet runs on`, `Main`, `Preview`, `Refresh preview`, `Refreshing preview`, `Update to main`, `Updating to main`, `On latest main`, `Merge in-flight branches and restart on the preview`, `Restart on latest main`, `Revert`, `Reverting`, `Back to the build of <built>, <commit>`, `Next build holds`, `Leave out of the build`, `Building`, `Ready`, `Bridge reopens`, `Swap`, `Swapping`, `Held by the snapshot`, `Left out of the preview`, `Moved since it was left out`, `Aligned with main`, `main`, `Drones are working`, `Restart`, `Cannot be redirected until it finishes`, `Jobs adopted`, `Keep`, `Trying`, `Swapped back`, `Swapping to the build`, `Dismiss`.
The adopt-cost line is `Its pipes die: no redirect, poke or verdict. Its recorded spend is an undercount. It shows as unheard. Its servers stop with Fleet. A Check running mid-gate most likely re-runs from scratch` (`FleetBuild.tsx:180-182`).
A built time reads `5 Nov, 17:05` (`RS/fleet-build.ts:35-40`).

## 5. Fleet down and Restart Fleet

### 5.1 Where it lives

| What | File |
|---|---|
| Connection states | `packages/protocol/src/connection.ts`:118-172 (`reading`, `not_running`, `runtime_file_refused`, `connecting`, `starting`, `unreachable`, `protocol_mismatch`, `connected`) |
| Words per state: headline, detail, next | `packages/shell/src/fleet.ts`:43-139 (`statementOf`, `silenceOf`), `:173-186` (`fleetStateOf`), `:194-203` (`SHORT_LABEL`) |
| Failure built from them | `packages/shell/src/failures/reading.ts`:75- (`fleetFailure`), taken at `RS/App.tsx:574` (`failingIn`) |
| Notice with Restart Fleet | `packages/shell/src/FleetNotice.tsx`:53-139 |
| Notice block | `packages/shell/src/FailureSurface.tsx`:142- (`FailureBlock`) |
| Band above the screen | `RS/Standing.tsx:55-116` |
| Fleet panel (the dot's popover body) | `PC/compositions/FleetPanel/FleetPanel.tsx`, `FleetPanel.css` |
| Panel contents | `RS/left-column.ts:35-86` (`fleetPanelOf`, `fleetPanelClocked`), `:100-131` (`rowsOf`), `:138-153` (`sentenceOf`) |
| Power-on and fault timing | `RS/cockpit/boot.ts:10-103` (`powerOf`, `usePower`), called at `RS/App.tsx:583` |
| Restart action | `MN/restart-fleet.ts:1-80`, channel `bridge:restart-fleet` (`apps/desktop/src/shared/api/core.ts`:209), handler `MN/index.ts:1064` |
| Failure words for a restart that did not take | `packages/shell/src/failures/commands.ts`:174- (`fleetRestartFailure`) |

### 5.2 The dot on the bar

```
.armada-title-bar__fleet  (padding --space-1)
  Popover  align end, label "Fleet"
    trigger: button.armada-title-bar__fleet-button [aria-label and title "Fleet — <label>"]
      dot 6px  |  ArmadaMark 16px pulsing (starting)
    body: FleetPanel (FleetInAPopover holds its own open state, starts open)
```

| Part | Value | Where |
|---|---|---|
| Dot | `--dot` = 6px circle, radius 50% | `TitleBar.css:611-617` |
| Tone by state | `running` green `--status-completed-success`; `not-running` `--status-escalated`; `unreachable` `--status-awaiting-review`; `starting` and `unknown` neutral `--fg-muted` | `FleetPanel.tsx:90-96`, `TitleBar.css:619-629` |
| Starting | `ArmadaMark` 16px in place of the dot, colour `--fg-muted`, opacity to 0.35 on a loop of `--duration-pulse * 2` = 3200ms, still under reduced motion | `FleetPanel.tsx:106-123`, `FleetPanel.css:27-59` |
| Button | padding 0, no border, radius `--radius-sm`, focus ring | `TitleBar.css:593-606` |
| Name and tooltip | `Fleet — <label>` (em dash) via `fleetSaid`; a plain `title`, not the Tooltip primitive | `FleetPanel.tsx:80-82`, `TitleBar.tsx:675-697` |
| With no panel passed | a `role="img"` graphic instead of a button | `TitleBar.tsx:668-686` |

State and label by connection:

| `Connection.state` | Dot state | Label | Dot |
|---|---|---|---|
| `connected` | running | `Running` | green |
| `not_running` | not-running | `Not running` | `--status-escalated` |
| `unreachable` | unreachable | `Unreachable` | `--status-awaiting-review` |
| `starting` | starting | `Starting` | pulsing mark |
| `reading` | unknown | `Reading` | neutral |
| `connecting` | unknown | `Connecting` | neutral |
| `runtime_file_refused` | unknown | `Refused` | neutral |
| `protocol_mismatch` | unknown | `Mismatch` | neutral |

(`packages/shell/src/fleet.ts`:173-203.)

### 5.3 The panel in the popover

```
Panel "Fleet"  (head: label "Fleet" and the state dot, a chevron when it has a body)
  .armada-fleet-panel (column, gap --space-1, padding --space-1)
    FleetBuildSection                          section 4; absent when no build
    FigureList: pid, port, protocol, up        only the rows the state has a value for
    .armada-fleet-panel__mono  sentence        mono, --text-2xs 12px / 16px, --fg-subtle, wraps anywhere
    Doctor row  (button "Open Doctor", chevron)   dot + "Doctor" + outcome + modules read
```

| Row | When | Value |
|---|---|---|
| `pid`, `port` | states `connected`, `connecting`, `starting`, `unreachable` | from the runtime file (`left-column.ts:100-118`) |
| `protocol` | `connected` | `spoken(protocolId)` |
| `up` | `connected` and a parseable `startedAt` | `lasting(now - startedAt)` on the window clock |
| Sentence | `unreachable`: `alive, no answer for <n> · last read <n> ago` (or `nothing read yet`); other fault states: the statement's detail; `connecting`, `connected`, `starting`: none | `left-column.ts:138-153`, `fleet.ts:128-140` |
| Doctor row | `onOpenDoctor` is set: outcome `pass`, `warn`, `fail` or `reading`, in green, amber, `--status-escalated`, neutral; the modules read | `FleetPanel.tsx:228-252`, `FleetPanel.css:91-160` |

The panel draws its head alone when it has no figure, sentence, Doctor line or build (`FleetPanel.tsx:156-161`).
The panel's `reading` callback is asked at the panel's own clock, so the uptime and the silence move each second without redrawing the window (`left-column.ts:64-86`).

### 5.4 Statements by state

| State | Headline | Detail | Next |
|---|---|---|---|
| `reading` | `Reading Fleet's runtime file` | none | none |
| `not_running` | `Fleet is not running` | `no runtime file at <path>`, or `pid <n> is held by nothing · Fleet exited without cleaning up`, or `pid <n> is held by a process that started <holder>, not <wrote> · its port is not Fleet's` | `Start Fleet. Bridge reconnects on its own.` |
| `runtime_file_refused` | `Fleet's runtime file was refused` | `<why>: <detail> · <path>` | `Check what wrote the file. Bridge will not connect to a port it names.` |
| `connecting` | `Connecting to Fleet` | `pid <n> · port <n>` | none |
| `starting` | `Fleet is starting` | `pid <n> · port <n>` | none |
| `unreachable` | `Fleet unreachable` | `pid <n> alive on port <n>, no answer for <n> · last read <n> ago` | `Fleet is up and not answering. What is shown below is not live.` |
| `protocol_mismatch`, Fleet sent no ID | `Fleet is out of date` | `Fleet <id> · Bridge <id>` | `Restart Fleet onto the installed build.` |
| `protocol_mismatch` | `Fleet and Bridge do not match` | same | same |
| `connected` | `Fleet running` | `pid <n> · port <n>` | none |

(`packages/shell/src/fleet.ts`:43-119.)
A state with no `next` takes no notice (`packages/shell/src/failures/reading.ts`:81).
`elapsed` writes `<n>s`, `<n>m` or `<n>h` (`fleet.ts:36-41`).

### 5.5 The notice above the screen

The band is drawn by `Standing` above the mounted screen and leaves the screen working beneath it (`Standing.tsx:1-10`, `:77-85`).
Nothing in it dismisses on a timer (`Standing.tsx:19-21`).

| Piece | Behaviour | Where |
|---|---|---|
| Not drawn while powering on | `fleet` is passed as `null` while `power` is `standby` or `kept`, or while a swap is held | `RS/App.tsx:895-897` |
| Power states | `standby`, `filling`, `kept`, `live`, `fault`; a Fleet down reads `standby` or `kept` for 30s (`POWER_GRACE_MS`) counted from opening or the last answer, then `fault`; a power-on lasts at least 1s (`POWER_LEAST_MS`) and fills over 1.4s (`POWER_FILL_MS`) | `RS/cockpit/boot.ts:10-14`, `:50-61` |
| `FailureBlock` | notice class from `failure.kind` (`fault` or degraded); headline, code chip, next line, details, values; actions: the Restart Fleet button, then a split `Copy debug info` control | `packages/shell/src/FailureSurface.tsx`:170-212 |
| Restart Fleet button | ghost, size sm, ground sunken; `RotateCw` 16px; label `Restart Fleet`; tooltip `Restart Fleet. Working Drones are adopted and cannot be redirected until they finish.` | `FleetNotice.tsx:96-114`, `:23-24` |
| Working | `LoaderCircle`, label `Restarting Fleet`, pending and disabled | `FleetNotice.tsx:106-111` |
| Only on a mismatch | the button draws only when the code is `bridge.fleet.protocol_mismatch` | `FleetNotice.tsx:57`, `:96` |
| New pid, notice still there | `Alert` tone escalated, title `Still not matching`, body `Fleet restarted on the installed build, and it is not this Bridge's. Update Armada, then reopen Bridge.` | `FleetNotice.tsx:60`, `:119-124` |
| No new runtime file in 30s | a second `FailureBlock` from `fleetRestartFailure` with the reason `no_answer`, with a dismiss | `FleetNotice.tsx:20`, `:62-80`, `:125-136` |
| Events dropped | `Alert` tone escalated, title `Events were dropped before Bridge saw them`, action `Noted`, body `<n> events will never arrive. Fleet resynced current state after each drop, so the list below is repaired.` | `Standing.tsx:99-111` |

What a press sends: Restart Fleet calls `restartFleet()` in the main process, not Fleet (`MN/restart-fleet.ts:1-12`).
It runs `launchctl print gui/<uid>/<label>`, checks that the pid launchd holds is the runtime file's pid, then `launchctl kickstart -k` (`MN/restart-fleet.ts:42-80`).
The label is `com.armada.fleet`, or `ARMADA_FLEET_LABEL` (`MN/restart-fleet.ts:38-40`).
Nothing crosses the wire to Fleet.

Failures of a restart, by `why`:

| `why` | Headline | Next |
|---|---|---|
| `not_started_by_armada` | `Fleet was not started by Armada's service` | `Stop it in the terminal it runs in, then start Fleet again.` |
| `not_running` | `Fleet is not running` | `Start Fleet. Bridge reconnects on its own.` |
| `refused` | `launchd refused to restart Fleet` | `Fleet is as it was. Read the log, or restart it from a terminal.` |
| `no_answer` | `Fleet did not come back inside 30 seconds` | `Read the Fleet log, then restart it again.` |

(`packages/shell/src/failures/commands.ts`:183-204.)

### 5.6 Other places Fleet down shows

| Place | What | Where |
|---|---|---|
| Dispatch disabled | the Dispatch entry in the narrow menu is disabled unless connected | `packages/shell/src/Shell.tsx`:154, `:272` |
| Overview | `disconnected={live ? null : statement.headline}` and `power` | `RS/App.tsx:1164-1165` |
| Swap hold | Fleet stopping during a swap is not drawn as a disconnect | `RS/App.tsx:333-334`, `RS/swap-hold.tsx:1-10` |

### 5.7 Keys

No act row in `actions.toml` names the Fleet dot, the panel, Restart Fleet or the notice.

## 6. Worked out in TypeScript

Every row is a place where a TypeScript file derives state, and not a value drawn from a Fleet field.
The last column says whether Fleet already sends the input or the result.

| Piece | TS path:line | Fleet already sends it? |
|---|---|---|
| Ring colour bands: warning from 80, danger from 95, for plan limits and the context ring | `packages/components/src/usage.ts`:4-5 (used at `TB/TitleBar.tsx:158`, `:285`, `ContextFigure.tsx:148`, `:289`) | missing; no band field in `crates/ipc/src` or `crates/fleet/src` |
| Plan percent: rounded for the label, clamped 0 to 100 for the arc | `TB/TitleBar.tsx:140`, `:277`, `:282` | `percent_used`, whole percent, past 100 on an exceeded spend limit: `crates/ipc/src/plan_usage.rs`:14; the clamp is TS |
| Which two limits draw, and their names `5h`, `wk` | `TB/TitleBar.tsx:96-112`, `:234-244` | `kind` is sent as open text: `plan_usage.rs:12`; the mapping is TS |
| Rings loading versus nothing: an empty list draws the loading glyph, an account off a subscription draws nothing | `RS/plan-usage.tsx:23-27`, `TB/TitleBar.tsx:245` | partly: `read_at` and an empty `windows` are sent and documented at `plan_usage.rs:20-30`; the loading rule is TS |
| "Resets in" countdown, two largest units | `TB/TitleBar.tsx:114-122`, `:149` | `resets_at` is sent: `plan_usage.rs:17`; the format is TS |
| Which window gets the offer | `RS/downshift.tsx:127-134` (`kind === "seven_day"`) | Fleet judges only the `seven_day` window: `crates/fleet/src/model_downshifting.rs`:27, `:86-88`; attaching it to the title row is TS |
| Downshift eligibility: crossed the line, offered once per window, not pinned by an environment variable, model not already the target | none in TS; `RS/downshift.tsx:122` only reads the state | yes: `ModelDownshift::Offered`, `crates/ipc/src/model_downshift.rs`:14; decided at `crates/fleet/src/model_downshift.rs`:62-76; the line is `plan-downshift-line`, default 80 (`crates/config/settings.toml`:1112-1121) |
| Downshift phase to card: attention text, facts, which acts show, which is primary and which stays open | `RS/downshift.tsx:53-110` | phase and model names are sent: `model_downshift.rs:11-19`; strings, act set and primary choice are TS |
| Downshift phase to toast text | `RS/downshift.tsx:141-168` | `percent`, `was`, `to` are sent: `model_downshift.rs:14`, `:16`, `:18`; the sentences are TS |
| Settle on `Choose…` and on the toast press | `RS/downshift.tsx:123-126`, `:149-152` | the settle route exists: `crates/api/src/routes.rs`:365; settling when the setting opens is TS |
| A refused downshift act is read again, not shown | `RS/fleet-downshift.ts:38-41` | the 409 is sent: `crates/fleet/src/model_downshifting.rs`:24; the handling is TS |
| Context percent | `PC/compositions/SessionFrame/ContextFigure.tsx:285` | missing; Fleet sends `context_tokens` and `context_window` only: `crates/ipc/src/sessions.rs`:82-84 |
| Context arc length, clamped 0 to 1 | `ContextFigure.tsx:265` | missing (derived from the two fields above) |
| Autocompact tick position, and whether one draws | `ContextFigure.tsx:237-242`, `:247-254` | the threshold is sent: `crates/ipc/src/context_breakdown.rs`:58; the share of the window and the suppress rules are TS |
| Breakdown bar segments: which categories draw, in what order, `deferred` left out | `ContextFigure.tsx:106`, `:188` | categories and `kind` are sent: `crates/ipc/src/context_breakdown.rs` (`ContextCategory`); the filter is TS |
| Heaviest three memory files and MCP servers; a file's short name; servers summed from tools | `ContextFigure.tsx:57-102`, `:200-225` | files and tools are sent, the ten heaviest tools: `context_breakdown.rs:49-51`; the top three and the sum by server are TS |
| Token figure text `53.8k`, `1M` | `ContextFigure.tsx:49-55` | missing |
| Whether Compact is offered and why it is off | `RS/cockpit/deck/session-tiles.ts:287-343` | missing; it reads the Job's `dead` state and whether `compact` is among the commands the agent or terminal lists |
| What Compact sends: the owner's tile words or `/compact`, as a message for a hosted Job and a command for a terminal Job | `RS/cockpit/deck/compact-act.ts:12-24` | the two routes exist: `crates/api/src/routes.rs`:512, `:544`; the choice and the words are TS |
| Which build is live (`on`) | `RS/fleet-build.ts:100` | yes: `crates/ipc/src/building.rs`:51 |
| Ahead and behind main | `PC/compositions/FleetBuild/FleetBuild.tsx:214`, `:449-474` | counts are sent: `building.rs:61` (`BuildPosition`, `:43-44`); "aligned" is TS |
| Idle on an aligned main, which disables the restart button | `FleetBuild.tsx:217`, `:264` | missing |
| Restart button label, tooltip, and the build it targets (`working ?? on`) | `FleetBuild.tsx:216-230` | `restarting` and `on` are sent: `building.rs:64`, `:51`; the words are TS |
| Restart stage words | `FleetBuild.tsx:39-47` | the stage is sent as an enum: `building.rs:68`; the words are TS |
| Whether Revert shows | `FleetBuild.tsx:279` | yes by presence: `previous`, `building.rs:85` (absent where launchd does not run Fleet from `builds/current`, `building.rs:82-84`) |
| Revert enabled and pending | `FleetBuild.tsx:284-285` | `reverting` is sent: `building.rs:93` |
| What Revert takes out and brings back | `RS/fleet-build.ts:128-137` | yes: `takes_out`, `brings_back`, `bridge_reopens` on `KeptBuildPrevious`, `building.rs:168-181` |
| Whether the Keep and Revert bar shows | `RS/trying-bar.tsx:17` (`trying.length === 0`) | yes: `trying`, `building.rs:105`, computed in `crates/fleet/src/kept_builds.rs`:223 (`trying_in`) and filled at `crates/fleet/src/building.rs`:518 |
| Keep act | `RS/App.tsx:901` | the route exists: `MN/fleet-build.ts:210`; Fleet marks Keep first: `crates/fleet/src/building.rs`:815 |
| Which Jobs a restart adopts (the Drones dialog) | `RS/fleet-build.ts:112` sets `drones: []` | missing from the report; `carried` lists titles of Jobs with a turn under way: `building.rs:89`, drawn only in the Revert dialog (`FleetBuild.tsx:517-523`) |
| Picked Jobs, snapshot state, left-out list, moved mark | `RS/fleet-build.ts:118-171` | yes: `building.rs:77`, `:97`, `:101` |
| Built time text `5 Nov, 17:05` and the 7-character commit | `RS/fleet-build.ts:35-48` | `built_at` and `commit` are sent on the kept builds: `building.rs:161`, `:169`; the formatting and the cut are TS |
| Swap hold: dim the window while `swapping`, elapsed seconds, the "swapped back" line taken from the start of `failed` | `RS/swap-hold.tsx:27-57` | `swapping` is sent: `building.rs:109`; the seconds counter and matching the text `Swapped back` at the start of `failed` are TS |
| Snapshot-ready chime on the move to `ready` | `RS/fleet-build.ts:77-89` | the state is sent; the move detection is TS |
| Fleet dot state from the connection | `packages/shell/src/fleet.ts`:173-186 | missing; the connection is Bridge's own reading of the runtime file in the Electron main process, and Fleet sends no such state |
| Fleet label word per state | `packages/shell/src/fleet.ts`:194-203 | missing |
| Fleet headline, detail and next per state, including the three kinds of absence | `packages/shell/src/fleet.ts`:43-119, `:143-156` | missing |
| Silence text `no answer for <n> · last read <n> ago` | `packages/shell/src/fleet.ts`:128-140 | missing; uses the window's clock and the last read instant |
| Elapsed text `<n>s`, `<n>m`, `<n>h`; uptime text | `packages/shell/src/fleet.ts`:36-41, `RS/left-column.ts:124-129` | `startedAt` and pid are in the runtime file; the text is TS |
| Which panel rows draw per state | `RS/left-column.ts:100-131` | missing |
| Power-on versus fault: 30s grace, 1s least, 1.4s fill, kept-from-disk | `RS/cockpit/boot.ts:10-103` | missing |
| Whether the notice draws at all (`next` present; held while powering on or swapping) | `packages/shell/src/failures/reading.ts`:81, `RS/App.tsx:895-897` | missing |
| Restart Fleet only on `protocol_mismatch`; the 30s no-answer rule; the "apart" check that a new pid still mismatches | `packages/shell/src/FleetNotice.tsx`:20, `:57`, `:60`, `:62-80` | missing; the restart is a `launchctl` call in the main process (`MN/restart-fleet.ts:42-80`) and Fleet is not asked |
| Dispatch disabled unless connected | `packages/shell/src/Shell.tsx`:154, `:272` | missing |
| Operator reopen button hidden while the full dock is open | `PC/compositions/TheShell/TheShell.tsx:327-336` | not applicable; a window state |
| Places drawn, with the old Jobs tab never drawn and digits following what is drawn | `packages/shell/src/surfaces.ts`:419-481 | not applicable; a local layout |
| Which labels the places give up when the row is narrow | `TB/TitleBar.tsx:433-510` | not applicable; a layout measure |
