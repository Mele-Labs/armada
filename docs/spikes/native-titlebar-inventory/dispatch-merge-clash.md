# Native title bar inventory: the dispatch sheet, the annotation layer, the merge queue strip, write-clash marks

This is what the Electron and React Bridge draws today, so the native rebuild can match it.
It records what is, and proposes nothing.
It covers the dispatch sheet and the strip a send leaves, the dev annotation layer, the merge queue strip along the window's bottom edge, and the write-clash mark.
The last section lists every place these pieces work a value out in TypeScript, and says whether Fleet already sends it.

Paths are relative to the repository root.
`R` is `apps/desktop/src/renderer/src`.
`M` is `apps/desktop/src/main`.
`DP` is `R/dispatch-panel`.
`DK` is `R/cockpit/deck`.
`PC` is `packages/components/src`.
`IPC` is `crates/ipc/src`.
`FL` is `crates/fleet/src`.
`BN` is `crates/bridge-native/src`.
`PS` is `packages/screens/src`.
`SH` is `packages/shell/src`.
`PR` is `packages/protocol/src`.
`AD` is `apps/desktop/src`.
`AT` is `crates/core-model/domain/actions.toml`.
`DPP` is `docs/practices/protocol`.
Density tokens and the 9 Nov "more room" tweak are as in [`../native-jobs-inventory/list-needs-palette.md`](../native-jobs-inventory/list-needs-palette.md).
Every pixel value below is Default's, resolved from the token named beside it.

## 0. Where the brief and the tree differ

| The brief says | The tree says | Where |
|---|---|---|
| The sheet opens on `n` and ⌘N | The registry key is ⌘N only; the bare `n` was retired on 31 Oct 2026 | `AT:340`, `:349` |
| The merge queue strip is `merge-line.tsx` | `merge-line.tsx` draws the Merge line panel on the rail page; the strip along the bottom edge is `MergeQueue` in the Cockpit deck | `R/merge-line.tsx:40`, `DK/Deck.tsx:1148` |
| Electron draws the Fleet merge line view (PR #3367) and calls (PR #3379) | Electron reads neither; it folds `get_merge_lines` itself and works its calls out from that fold | section 3.1 |
| The annotation layer dispatches a new Job through the dispatch sheet | The layer starts a held Job itself with `startSession`; it never opens the sheet | `R/annotate/send.ts:178`, no dispatch import under `R/annotate/` |
| `main/capture/` is the annotation layer | `main/capture/` is the walk and Studio capture window; the dev layer is `R/annotate/` with `M/annotations.ts` | section 2.1 |
| The mark is drawn on a Jobs list row and the Job head | It is drawn on the Jobs list row, the bays tile and the fleet map; the Job's view strip draws a different dot | section 4.2 |

## 1. The dispatch sheet

### 1.1 Where it lives

| What | File |
|---|---|
| The sheet, its fields, the repository row, the band | `DP/DispatchPanel.tsx` (`DispatchPanel` 245-695, `useHung` 698-717) |
| The one draft, every act, every Fleet call | `DP/dispatching.tsx` (`useDispatch` 190-937, `usePanel` 563-808) |
| The mount over the window and the strip a send leaves | `DP/entries.tsx` (`DispatchSheet` 12-16, `TitleDispatchedStrip` 45-85) |
| Kind names and marks | `DP/kinds.ts` (`KIND_LABEL` 7, `KIND_ICON` 13) |
| `@` and `#` menus | `DP/SessionMenu.tsx` (`SessionMenu` 125-186, `useAtMenu` 199-422) |
| The sentence over the request | `DP/WaitChip.tsx`, `DP/waits.ts` |
| Which repository the words name, chip order | `DP/reading.ts` |
| An Autopilot's read settings | `DP/AutopilotBody.tsx` |
| The strip after an Autopilot is sent | `DP/DispatchedStrip.tsx`, `DP/dispatch-strip.css` |
| CSS | `DP/dispatch-panel.css` (sheet, drawer, chips, wait sentence, menu) |
| Mounted once, in `App` | `R/App.tsx:1205` (`<DispatchSheet />`), `:1206` (`<TitleDispatchedStrip />`), `:535` (`useDispatch`) |
| The key that opens it | `R/one-bar.tsx:83-131` (`useOpenedToDispatch`) |
| Model and effort lists, `start`, `queue`, `readsWait` | `R/sessions-wired.tsx:1314` (`start`), `:1332` (`readsWait`), `:1359` (`queue`), `:1617-1629` (`models`, `efforts`) |

The sheet has its own `<textarea>` (`DP/DispatchPanel.tsx:498`); it is not the Job composer.
It borrows the composer's pieces: `AttachmentChip`, `isLargePaste` (8,000 tokens, `PC/compositions/SessionComposer/large-paste.ts:5`), `SendKbd`, `Pick` and `Select` for Model and Effort, and the mention rows (`.armada-mention__row`) for the menus.

### 1.2 Kinds

| Kind id | Word | Glyph | Offered when | Where |
|---|---|---|---|---|
| `session` | `Job` | `SquareTerminal`, 20px on the line | the window holds a Sessions draft | `DP/kinds.ts:7-16`, `DP/dispatching.tsx:201-207` |
| `autopilot` | `Autopilot` | `Plane` | `window.armada.proposeAutopilot` is a function | `DP/dispatching.tsx:200`, `:207-212` |

The kind is `autopilot` only while chosen for this opening; the next opening is a Job again (`DP/dispatching.tsx:198`, `:213`, `:268`).
Tab steps through the offered kinds from the request (`DP/dispatching.tsx:524`, `:781-788`); with one kind the tabs and the `⇥` hint are not drawn (`DP/DispatchPanel.tsx:553`, `:592`).
The New workflow variant draws no kinds and a Scope radio group (`DP/dispatching.tsx:537-551`, `:672`).

### 1.3 Keys

| Key | Registry id | Scope | Does | Where read |
|---|---|---|---|---|
| ⌘N | `new_job` | `anywhere` | opens the sheet on the draft as it stands, or an empty one | `actions.toml:340-349`, `R/one-bar.tsx:110-126`, `DP/dispatching.tsx:810` |
| ⌥A | `dispatch_after` | `anywhere` | opens the sheet on a Job with an `@` typed and its menu open; inside the sheet it types the `@` at the caret | `actions.toml:128-137`, `R/one-bar.tsx:102-107`, `DP/DispatchPanel.tsx:352-357` |
| ⌥1 to ⌥9 | `dispatch_repository` | `anywhere` | chooses the chip with that number; with none chosen it also sends | `actions.toml:1062-1072`, `DP/DispatchPanel.tsx:359-365` |
| ⇧⌘↵ | `queue_session` | `anywhere` | Queue for later, on a Job | `actions.toml:116-126`, `DP/dispatching.tsx:95`, `:777-780` |
| ⌘↵ | `send_message` | `anywhere` | sends an Autopilot step (`sendsOn`) | `actions.toml:102-114`, `DP/DispatchPanel.tsx:547-549` |
| ↵ | none | in the request, on a Job | Start | `DP/dispatching.tsx:791-799` |
| ⇥ | none | in the request | next kind | `DP/dispatching.tsx:780-788` |
| ⇧⇥ | none | in the request, with a wait sentence | focus goes to the sentence's first part | `DP/DispatchPanel.tsx:346-348`, `:537-545` |
| ← and → | none | on the repository row | move between chips, wrapping | `DP/DispatchPanel.tsx:378-387` |
| Esc | none | in the sheet | closes the `@` menu, else the row's asking, else collapses; the draft stays | `DP/DispatchPanel.tsx:366-376`, `DP/dispatching.tsx:558-561` |
| `d` | `deck_panel` | `deck` | focus goes to the Dispatch tile | `actions.toml:536-547` |

The key listens in the capture phase, so the surface under it never hears it (`R/one-bar.tsx:78-81`, `:128`).
It does nothing while any `[role="dialog"]` is up, and the sheet is one (`R/one-bar.tsx:119-123`, `DP/DispatchPanel.tsx:687`).
A modifier key (⌘, Ctrl, Alt) works from inside a text field; a bare key does not (`R/one-bar.tsx:111-123`).
A row that marks `n` its own (`data-own-keys~='n'`) keeps a bare key (`R/one-bar.tsx:113-118`).
The sheet's own hint beside the request is a hard-coded `N` (`DP/DispatchPanel.tsx:558-560`); the registry key is ⌘N.
Other entries that open the same sheet: the Cockpit's Dispatch tile (`DK/Deck.tsx:1883-1896`), the title bar menu item (`PC/compositions/TitleBar/TitleBar.tsx:736-746`), the palette row (`R/palette.ts:259`), a palette dispatch row (`R/one-bar.tsx:67-68`), every New session entry (`R/App.tsx:543`).

### 1.4 Layout

```
.armada-dispatch-panel [role=dialog aria-modal aria-label="Dispatch"]   fixed, inset 0, scrim, z --z-palette
  section.armada-dispatch-panel__frame [aria-label="Dispatch" data-open data-kind data-reading data-asking]
    WaitChip                              only on a Job with a wait read
    .armada-dispatch-panel__line
      prompt (kind glyph, tooltip = kind word)
      .armada-dispatch-panel__words
        mirror (aria-hidden, lights the tagged Job's `@title`)
        textarea [aria-label="Request"]
      ⇥ Kbd                               only with two kinds, only while the frame has focus
      N Kbd                               tooltip "Dispatch", shortcut N
    .armada-dispatch-panel__snippets [role=group aria-label="Attached"]   pastes kept as files
    SessionMenu                           while an @ or # is being typed
    .armada-dispatch-panel__drawer        opens when data-open
      .armada-dispatch-panel__repos [role=radiogroup aria-label="Repository"]
        reading mark (FolderGit2 or LoaderCircle, 12px)
        chip* [role=radio aria-label="Repository <name>"]  name + Kbd (⌥1 to ⌥9)
        More menu                         past nine
      .armada-dispatch-panel__band
        Tabs (Job / Autopilot)            only with two kinds
        .armada-dispatch-panel__send
          Model Select, Effort Select     Job only
          Queue for later                 Job only, where the queue is served
          Start / Read it / Start Autopilot
      .armada-dispatch-panel__settings    Autopilot body, or the workflow Scope radios
```

Source: `DP/DispatchPanel.tsx:396-457` (row), `:459-680` (frame), `:682-694` (dialog).

### 1.5 Sizes and tokens

| Part | Value | CSS |
|---|---|---|
| Scrim | `--scrim`; fade `--duration-base` = 180ms | `DP/dispatch-panel.css:11-17` |
| Frame position | top 12vh, centred on `left: 50%` | `:19-28` |
| Frame width | `min(calc(var(--palette-width) * 1.75), calc(100vw - var(--space-6)))` = min(980px, 100vw - 24px) | `:23-24` |
| Frame max height | `calc(100vh - 12vh - var(--space-6))`; the drawer scrolls inside | `:26` |
| Frame | min-height `calc(var(--h-row-unit) * 2)` = 72px; 1px `--border-subtle`; radius `--radius-md` 6px, `--radius-lg` 8px when open; ground `--bg-sunken` | `:38-52`, `:54-58` |
| Open shadow | `--shadow-overlay` | `:57` |
| Focus | border `--accent`; extra ring `calc(var(--border-width) * 3)` = 3px in 20% accent | `:60-69` |
| Asking | border and glow in `--hue` = `--status-awaiting-review`; inset glow 16% over `calc(var(--space-12) * 1.5)` = 72px; outer 16% over `--space-6` 24px | `:39`, `:72-78` |
| Line | padding 21px top and bottom (`(72 - 2 - 28) / 2`), `--space-6` 24px sides; gap `--space-4` 16px | `:83-89` |
| Request | `--font-sans`, `--text-lg` 18px / `--leading-lg` 28px; height 28px closed, `calc(var(--leading-lg) * 5)` = 140px open; `--fg-default`, placeholder `--fg-subtle` | `:111-141`, `:129-131` |
| Height and drawer motion | `--duration-sheet` = 220ms; `--ease` | `:126`, `:153-167` |
| Snippets row | gap `--space-2` 8px; padding `0 24px 12px` | `:104-109` |
| Drawer body | `--font-mono`, `--text-xs` 13px / `--leading-xs` 18px | `:169-176` |
| Repository row | padding `--space-3` 12px / `--space-4` 16px; gap `--space-2` 8px; top rule `--border-default`; ground `--bg-sunken` | `:225-236` |
| Reading mark | height `--space-6` 24px; padding `0 --space-1` 4px; spin `--duration-pulse` 1600ms | `:239-262` |
| Repository chip | height `--space-6` 24px; padding `0 4px 0 12px`; gap 12px; 1px `--border-default`; radius `--radius-sm` 3px; ground `--bg-raised` | `:264-282` |
| Read chip | border `--accent` | `:304-306` |
| Chosen chip | fill `--accent`, text `--fg-inverse`, weight `--weight-heading` 600, `--shadow-primary`; hover `--accent-hover` | `:309-326` |
| Band | min-height `calc(var(--h-row) * 1.25)` = 35px; padding `4px 12px 4px 16px`; gap 12px; top rule; ground `--bg-raised`; text `--fg-muted` | `:179-191` |
| Selects | height `--h-control-sm` 24px; `--text-xs`; Pick min-width `calc(var(--space-12) * 1.5)` = 72px | `:211-221` |
| `@` menu | static in the sheet's flow; max-height `calc(var(--h-menu-item) * 5)` = 140px; no radius | `:444-460` |
| Wait sentence | mono `--text-xs`; ground `--bg-raised`; line padding `--space-2` 8px / `--space-4` 16px; gap 8px | `:464-479` |
| Wait part | height `--space-6` 24px; padding `0 8px`; 1px `--border-default`; radius 3px; ground `--bg-sunken`; max-width `calc(var(--palette-width) * 0.6)` = 336px; set by hand: border `--accent` | `:485-517` |
| Tag in the request | radius 3px; ground `--bg-raised`; 1px `--border-default` ring; text transparent | `:430-435` |
| Settings | padding 12px / 16px; top rule; Autopilot grid gap 12px | `:533-560` |
| Menus past nine | portalled; `--z-palette` | `:391-393` |
| Toasts while the sheet is up | `calc(var(--z-palette) + 1)` | `:396-398` |
| Reduced motion | the reading spin stops | `:384-388` |

### 1.6 Controls

| Control | Accessible role and name | Glyph and tooltip | What a press does | Shown when |
|---|---|---|---|---|
| Sheet | `dialog`, `Dispatch`, modal | none | a press on the scrim calls `onDismiss` (`closeSheet`) | open |
| Request | textarea `Request` | none | words kept under `dispatch:request` (`useKept`); every change clears the Autopilot read, the refusal, the pending act and any strip | always |
| Prompt | tooltip = kind word | kind glyph | none | always |
| Reading mark | `img`, `Repository` + name, or `Repository, finding`, or `Repository, not found` | `FolderGit2`; `LoaderCircle` while detecting; tooltips in 1.8 | none | row drawn |
| Repository chip | `radio`, `Repository <name>`, `aria-checked` = chosen | key cap from `digitKeyFor("dispatch_repository", n)` | picks the repository; focus returns to the request unless the row is asking | first nine chips |
| More menu | trigger label = chosen hidden repository, else `More`; aria `More repositories` or `Repository <name>`; search `Find a repository` | dropdown | picks the repository (`pick(rootOf(id))`) | chips past nine |
| Tabs | tab per kind | none | `onKind` | two kinds |
| Model | select `Model`, Pick glyph `Cpu` | first option = default model name or `Auto` | `onTune({model, effort})` kept under `dispatch:session-tuning` | Job |
| Effort | select `Effort`, Pick glyph `Zap` | first option `Auto`, then Fleet-less list `low medium high xhigh max` | same | Job |
| Queue for later | button, `Queue for later` | ⇧⌘↵ drawn while ⌘ is held | saves the idea locally and closes the sheet | Job, queue served, no snippet attached |
| Start | button, label = `act` | ↵ or ⌘↵ drawn while ⌘ is held (`SendKbd`) | see 1.7 | always |
| Attached chip | `AttachmentChip` with remove | file name | removes the file | a snippet is held |
| `@` and `#` rows | `listbox` named by `label`, `option` | glyphs in 1.9 | writes the row into the request | while a mention is typed |
| Wait parts | buttons `Job <title>`, `Condition <label>`, `Starts from <label>`, `Needs <label>` | tooltips in 1.11 | cycle or open the Job menu | a wait read exists |
| Autopilot Instruction | input `Instruction`, mono | none | edits the read | Autopilot read |

Act word: `Start` on a Job; `Read it`, then `Start Autopilot` on an Autopilot once a read stands (`DP/dispatching.tsx:694`).
The act is disabled when the window is not connected (`DP/dispatching.tsx:806`) or no words and no snippet stand (`DP/DispatchPanel.tsx:666`).
Queue for later is also disabled while a start is out or the words are empty (`DP/DispatchPanel.tsx:654`).

### 1.7 What each act sends

| Act | Fleet route and body | Then |
|---|---|---|
| Repository read, 250ms after the words rest (`READ_MS`, `DP/reading.ts:13`) | `GET /repositories/of_request?q=<words, first 16,000 characters>`; answer `{manifest_id?, likely[]}` (`M/request-repository.ts:11`, `:30-40`; `IPC/repositories.rs:69-79`) | none answered reads as no repository |
| Wait read, 400ms after the words rest (`sessions-wired.tsx:160`), one per tagged Job and words, 24 kept (`:162`) | `POST /sessions/read_wait` `{session_id, request}`; answer `{condition, base, needs?, plain?}` (`M/sessions.ts:1300`, `IPC/session_waits.rs:110-131`) | the sentence over the request; `plain` makes the tag a reference only |
| Start (Job) | `POST /sessions/start` with `StartSession`: `manifest_id` of the chosen repository, `request` = the trimmed words (or `The request is in the attached snippet.` for snippets alone), `model` and `effort` only when picked, `after` `{session, condition, base, needs?}` when the words wait on a Job, `held: true` when they do not wait, `project_rows` when the sheet was opened with rows (`DP/dispatching.tsx:298-315`, `:383-425`, `M/picked.ts:32-100`, `IPC/hosted_sessions.rs:36-97`) | the answer's Job id |
| After a Start with no wait, or with files | `POST /sessions/message` with the words, uploaded snippet files and the `#` tags whose titles are still in the words; a plain `@` tag rides as a Job tag (`DP/dispatching.tsx:441-454`, `M/sessions.ts:1149`) | |
| After a Start | the Job opens (`openSession(id)`), the sheet is cleared (`clearAll`) | a refusal is the window's toast (`raiseStartRefused`, `sessions-wired.tsx:54`, `:870`); the sheet keeps its words |
| Start from a built brief (`startDraft`) | the same start with the sheet never up, deduped by `<root>\n<text>` while one is out (`DP/dispatching.tsx:318-337`, `:359-378`) | no repository: the sheet opens on the words |
| Queue for later | none; `localStorage` key `armada.bridge.queued-ideas` holds `{id, text, root, repository, model?, effort?}` (`R/queued-ideas.ts:12-26`, `sessions-wired.tsx:1359-1366`) | the sheet clears; nothing opens |
| Read it (Autopilot) | `POST /autopilots/<manifest id>/proposal` `{sentence, local_now}` (`M/issues.ts:244-258`); answer `AutopilotProposal` (`IPC/autopilot.rs:183-195`) | the body shows Instruction and the fields |
| Start Autopilot | `POST /autopilots/<manifest id>` with `AutopilotSettings` (`M/issue-channels.ts:101-104`, `IPC/autopilot.rs:158-181`) | the sheet clears and the strip appears (1.10) |
| Pick a chip when the act is waiting on a repository | a Job starts at once; an Autopilot reads at once; Queue saves at once (`DP/dispatching.tsx:687-693`) | |

A Job start holds the Job on its start card (`held: true`) so Fleet's proposer reads the words (`DP/dispatching.tsx:421`, `IPC/hosted_sessions.rs:68-97`).
The start waits on a real wait reading first, so a Job is never started on defaults while the proposer is out (`DP/dispatching.tsx:382-383`).
Nothing starts without a repository: a read still out is waited for (`waiting`), and none found asks (`asking`) (`DP/dispatching.tsx:629-635`).

### 1.8 States

| State | Drawn as | Where |
|---|---|---|
| Closed line | the line alone; the drawer is `visibility: hidden` and `inert` | `DP/dispatch-panel.css:153-161`, `DP/DispatchPanel.tsx:587` |
| Open | the drawer grows on the frame after the sheet mounts | `DP/dispatching.tsx:243-249` |
| Reading `idle` | `FolderGit2`, muted; tooltip and name `Repository` | `DP/DispatchPanel.tsx:102-105` |
| Reading `detecting` | `LoaderCircle` spinning, accent; tooltip `Finding the repository`; name `Repository, finding` | `:106-109`, css `:256-262` |
| Reading `unresolved` | muted; tooltip `No repository found`; name `Repository, not found` | `:110-113` |
| Reading `detected` | accent; tooltip `Found in the request` | `:114-117` |
| Reading `picked` | accent; tooltip `Picked by you` | `:118-121` |
| Reading `scoped` | accent; tooltip `The one the rail is on` | `:122-125` |
| Chip `read` | accent edge | css `:304-306` |
| Chip `chosen` | filled accent | css `:309-326` |
| Asking for a repository | frame takes the call hue; the row is filled in it; chips outlined in `--fg-inverse`; the row takes the keys, the chosen or first chip focused | css `:72-78`, `:342-376`, `DP/DispatchPanel.tsx:286-296` |
| Sending / reading | the act's `pending` state | `DP/DispatchPanel.tsx:665` |
| Queueing | Queue for later `pending` | `:653` |
| Not connected | request, chips, selects and buttons disabled | `DP/dispatching.tsx:806`, `DP/DispatchPanel.tsx:428` |
| Menu open | `SessionMenu`; active row `armada-mention__row--active`; Esc closes only the menu | `DP/SessionMenu.tsx:159-163`, `:379-383` |
| Wait read | sentence over the line; a part set by hand has an accent edge and ", set by you" in its tooltip | `DP/WaitChip.tsx:33-34`, css `:515-517` |
| Autopilot refused | `<p role="alert" class="armada-autopilot__refused">` with Fleet's sentence | `DP/AutopilotBody.tsx:50-54` |
| Menu past nine, chosen hidden | trigger filled in the accent | css `:334-339` |
| Loading | no skeleton; the only loading mark is the reading spinner | none in `DP/` |
| Empty | nothing drawn for an empty menu (`rows.length === 0` returns null) | `DP/SessionMenu.tsx:142` |

### 1.9 The `@` and `#` menus

| Trigger | Opens when | Rows | Row glyph | Choose |
|---|---|---|---|---|
| `@` | at the start of a word, narrowing while some Job title still starts with what was typed | live Jobs whose title contains the query, then up to 5 files once something follows the `@` (`FILE_ROWS`, `DP/SessionMenu.tsx:66`) | `SquareTerminal`, `File` | writes `@<title> ` or `@<path> ` |
| `#` | at the start of a word, a repository known, `numbered` served | open issues and pull requests, a typed number looked up after 150ms (`REST_MS`, `:68`), at most 6 (`NUMBERED_ROWS`, `:67`), numbers first by descending id, then keys | `GitPullRequest`, `Ticket`, `Bug`, `Terminal` | writes the whole title and holds the tag, sent while the title is in the words |

Menu labels: `Jobs`, `Tag a Job or a file` (files shown), `Tag an issue or pull request` (`DP/SessionMenu.tsx:353-358`).
Keys in a menu: ↑ and ↓ wrap, ↵ or ⇥ choose, Esc closes it (`:376-404`).
A press on a row is `onMouseDown` so the field keeps focus (`:165-169`).

### 1.10 The strip after an Autopilot is sent

A Job opens itself and leaves no strip; only an Autopilot does (`DP/DispatchedStrip.tsx:1-4`).
The strip holds the latest send until the next dispatch or a dismiss (`DP/dispatching.tsx:269`, `:529`).

```
.armada-dispatch-strip [role=group aria-label="Last dispatch" data-kind tabIndex=-1]   fixed over the title bar's field
  mark (Plane 16px, accent; tooltip "Autopilot")
  title (the repository name, mono, ellipsis)
  ↵ Kbd                       visible only while the strip has focus
  Open [secondary sm]
  Dismiss [ghost sm iconOnly, X 14px, tooltip "Dismiss"]
```

| Part | Value | Where |
|---|---|---|
| Position | `position: fixed` at the title bar search field's box, read once as it appears (`--hung-top/left/width/height`); anchor `.armada-title-bar__search` | `DP/dispatch-strip.css:56-69`, `DP/DispatchPanel.tsx:698-717`, `R/one-bar.tsx:21` |
| Look | ground `--bg-raised`; 1px `--border-default`; radius `--radius-lg` 8px; mono `--text-xs`; padding `4px 12px 4px 16px` with block padding 0; gap `--space-3` 12px; z `--z-menu` 40 | `DP/dispatch-strip.css:5-18`, `:56-69` |
| Focus | border `--accent` | `:71-73` |
| Enter | fade and rise 8px over `--duration-base` 180ms | `:25-30` |
| Drag | `-webkit-app-region: no-drag` | `:68` |

| Key on the strip | Does | Where |
|---|---|---|
| ↵ (focus on the strip) or a press anywhere on it | opens the repository's Autopilot lane on the Cockpit and asks the Cockpit to put its bar up once | `DP/entries.tsx:68-70`, `DP/dispatching.tsx:289-294` |
| Esc | dismisses | `DP/entries.tsx:64-67` |
| a printable key | starts the next draft: `openSheet(key)` | `DP/entries.tsx:71-80` |

The cursor lands on the strip when it appears (`DP/entries.tsx:53-55`).
The strip draws only where the sheet is down and a send stands (`DP/entries.tsx:50`).

### 1.11 Copy strings

| String | Where |
|---|---|
| `Describe the work, or paste a link to a ticket.` | `DP/dispatching.tsx:54` |
| `What should it do?` | `:55` |
| `Every repository on this Mac`, `This repository` | `:59-62` |
| `The request is in the attached snippet.` | `:87` |
| `Start`, `Read it`, `Start Autopilot` | `:694` |
| `Queue for later` | `DP/DispatchPanel.tsx:657` |
| `Repository`, `Repository, finding`, `Repository, not found` | `:102-113` |
| `Finding the repository`, `No repository found`, `Found in the request`, `Picked by you`, `The one the rail is on` | `:106-125` |
| `More`, `More repositories`, `Find a repository` | `:443-447`, `SH/repository-entries.ts:32` |
| `Job`, `Autopilot` | `DP/kinds.ts:7-10` |
| `Job / Autopilot` (the `⇥` tooltip) | `DP/DispatchPanel.tsx:554` |
| `Attached`, `Request`, `Model`, `Effort`, `Auto` | `:564`, `:504`, `:605`, `:625`, `:617`, `:637` |
| `Starts when`, `'s`, `from`, `needs`, `·` | `DP/WaitChip.tsx:100-127` |
| `The Job it waits on`, `When it starts`, `Where it starts from`, `What it needs`, `, set by you` | `:12-17`, `:34` |
| `Start condition` (group), `Job <title>`, `Condition <label>`, `Starts from <label>`, `Needs <label>` | `:98`, `:106`, `:88` |
| `ends`, `<title> is done`, `PR is pushed`, `PR lands`, `main`, `its PR branch` | `DP/waits.ts:72-102` |
| `Instruction` | `DP/AutopilotBody.tsx:28` |
| `Last dispatch`, `Open`, `Dismiss` | `DP/DispatchedStrip.tsx:60`, `:73`, `:79` |
| `Scope` | `DP/dispatching.tsx:539` |

The workflow brief the New workflow sheet sends is built here: `New workflow: <words>`, `Scope: <where>`, an instruction to use the `propose_new_workflow` tool, and a pointer to `.armada/workflows/feature.json` (`DP/dispatching.tsx:65-74`).

## 2. The in-app annotation layer

### 2.1 What it is, and where ⌥⌘A works today

The layer is the dev annotation layer: point at anything in Bridge, write a note, then send the notes to a Job picked from a menu or to a new held Job.
It is `R/annotate/` with its files written by `M/annotations.ts`.
It is not the Studio capture (⌥⌘C, shipped) and not the walk window's capture; those are separate (section 2.7).

| Link in the gate | Where |
|---|---|
| The registry does not list ⌥⌘A: "the development annotation layer's own … ships in no packaged build" | `AT:91` |
| `DEV_BUILD = isDevBuild(app.isPackaged, process.defaultApp)` | `M/index.ts:135` |
| A dev build is not packaged, or packaged in name only and started with a source folder | `M/dev-build.ts:10-12` |
| Main passes `--armada-annotate` to the window only in a dev build | `M/index.ts:337-341`, flag at `AD/shared/annotations.ts:63` |
| The IPC handlers are registered only in a dev build | `M/index.ts:740-742` (`handleAnnotations`) |
| The preload exposes `window.armadaDev` only when the argument is present | `AD/preload/index.ts:1612-1639` |
| The renderer loads the layer's chunk only when `canReport()` is true: `window.armadaDev` is set or `import.meta.env.DEV` | `R/main.tsx:213-216`, `R/report.ts:9` |
| In a browser (vite dev server, the mock) the sink writes through the dev server | `R/annotate/mount.tsx:17-23`, `M/annotations-server.ts` |
| Files are written under `<repository root>/.armada/annotations/<id>.json`, with `<id>.png` and example pictures beside | `M/annotations.ts:103-105`, `:140-149`, `AD/shared/annotations.ts:16-19` |
| A kept build with no repository above it reads the tree from its `build.toml` | `M/annotations.ts:54-70` |
| The Job a send starts is in the repository Fleet serves for the checkout, not the worktree | `M/annotations.ts:77-101`, `:264` |

⌥⌘A is read by key place: `code === "KeyA"` with ⌘ and ⌥ and neither Ctrl nor Shift (`R/annotate/Layer.tsx:75-82`).
The layer listens in the capture phase and toggles (`:354-364`); whether it is on survives a reload in `sessionStorage` key `armada.annotate.on` (`:98`, `:277`, `:380`).
Report also turns it on and opens the card on an element (`:453-463`).

Where else ⌥⌘A is bound:

| Where | Does | Source |
|---|---|---|
| A Job browser pane is on screen in Bridge's document | starts Capture on that browser ahead of the layer, unless the layer is already on | `R/job-browser/chord.ts:1-24`, `M/job-browsers.ts:650-653` |
| The walk or Studio capture window | starts Capture, as ⌥⌘C does | `M/capture/window.ts:119-126` |
| The mock walk window | arms capture | `PS/fixtures/walk-window.tsx:637-641` |

### 2.2 Where the layer lives

| What | File |
|---|---|
| The layer: toggle, hover outline, pins, card, panel, bar, sends | `R/annotate/Layer.tsx` (`Layer` 260-1637, `Card` 1662+) |
| The tools on a card and the example pictures | `R/annotate/Tools.tsx` (`Tools` 135+, `Examples` 633+) |
| What a send calls on Fleet | `R/annotate/send.ts` (`sendToSession` 146-338) |
| Reading an element: component, selector, owners, source | `R/annotate/capture.ts`, `fiber.ts`, `selector.ts`, `place.ts` |
| The sink (files through main or the dev server) | `R/annotate/sink.ts`, `M/annotations.ts`, `M/annotations-server.ts` |
| The note shape | `AD/shared/annotations.ts` |
| The note a send builds | `AD/shared/pinned-note.ts:30-105` (`annotationNote`) |
| Own React root beside the app's | `R/annotate/mount.tsx:13-31` |
| Repository the Job starts in | `M/picked.ts:18-25` (`repositoryToStart`), `:32-100` (`startOn`) |
| Sends | `M/session-channels.ts:245-286` (start), `:337` (notes), `M/sessions.ts:1149` (message), `:1187` (notes) |

### 2.3 Layout

```
.armada-annotate [data-armada-annotate]                    own root, over the app
  Outline (hovered element, with its component name)
  pin button* [class armada-annotate__pin, label "Note N, open: <text>"]   numbered, at each note's element
  Card "New note" [role=dialog]                            while a draft stands
    Band (Crosshair 14px, "New note")
    Path (component, selector, source)
    Textarea "Note"
    Tools [aria-label="Tools"]
    Examples [aria-label="Pictures"]
    Cancel, Save note
  Card "Note" [role=dialog]                                for an opened note
    Band (MapPin 14px, "Note N" or "Done note")
    text, Tools, Examples, "Sent to Job <title>"
    Start job (split, menu "Send to a Job"), Mark done / Reopen, Delete
  aside [aria-label="All notes"]                           the panel, while the status is open
    tabs Open / No job / Has job / Done, SearchField "Search notes"
    groups by screen ("No screen" for none), rows with Select, Start job split, Mark done, Delete
  .armada-annotate__bar [role=status]                      pinned over everything
    "Annotating", Screenshot window, "<n> open, <n> done", Start job split, ⌥⌘A KbdChord
```

Source: `R/annotate/Layer.tsx:1106-1636`.
Sizes are in `R/annotate/annotate.css`, which is not resolved here.

### 2.4 Tools: aim, box, held numbered notes

| Capability | How it works | Where |
|---|---|---|
| Aim at an element | while on, `pointermove` outlines the element under the pointer and names its component or tag; a click on it opens a draft card; the app's own pointer events are swallowed | `Layer.tsx:101-109`, `:465-494`, `:1108-1113` |
| A stray click | does not throw away a draft with words, changes or pictures | `Layer.tsx:127-132`, `:478` |
| Draw a box | not offered; a note is on an element or on the whole window | none in `R/annotate/` |
| Whole-window note | ⌥⌘S or the Screenshot window button: the window's picture and Bridge's state are taken as pressed, then the card opens | `Layer.tsx:84-91`, `:366-376`, `:614-639`, `:1593-1599` |
| Numbered notes | open notes numbered oldest first; a done note carries no number and renumbers nothing | `Layer.tsx:1062-1072`, `:1121-1139` |
| Pins follow the element | a frame loop re-measures each pin and the hovered and draft boxes, and sets state only when they changed | `Layer.tsx:496-547` |
| Notes saved | each note is a file; a failed picture is told and the note is saved without it | `Layer.tsx:583-608`, `M/annotations.ts:140-149` |
| Style changes and examples | the card's Tools edit size, colour, fill, padding, gap, text and variant on the live element and record the changes; pictures are pasted or dropped onto the card | `Tools.tsx:247-337`, `Layer.tsx:867-886` |
| Per-note state | `status` open or done; `sent` holds the Job it went to | `Layer.tsx:223-246`, `:889-903` |
| Notes read again | whenever the layer comes on, and when Fleet rewrites a note file | `Layer.tsx:378-412` |

### 2.5 Send target picker and what Send sends

The picker is a `SplitButton` labelled `Start job` (the `startLabel` prop; callers may pass `Dispatch`) whose menu is named `Send to a Job` and lists the live Jobs (`Layer.tsx:263-266`, `:1254-1281`, `:1612-1623`).
Live means `state === "live"` and either hosted or a terminal that is listening (`Layer.tsx:427-445`).
The main action sends to a new Job; a menu row sends to that Job.
Sends are offered from the bar (every open unsent note on this screen), from an open note's card, from a panel row, and from the panel's selection (`Layer.tsx:967-972`, `:1005-1011`, `:1522-1560`, `:1342-1360`).
A panel row without the act keeps its column (`Layer.tsx:1521`).

| What | Route and body | Where |
|---|---|---|
| New Job for the notes | `startSession(undefined, root, request, undefined, undefined, true)` then `POST /sessions/start` `{manifest_id or root, request = the notes' words joined by newlines, held: true}`; no title, no model, no effort; no repository found reads `Job not started: no repository found above Bridge` | `send.ts:164-195`, `M/picked.ts:32-100` |
| Notes on elements | one turn: `sendSessionNotes({session_id, notes: [{note: annotationNote(...), ring?}]})`; main burns each ring and number into its picture, then `POST /sessions/notes` | `send.ts:244-258`, `M/session-channels.ts:337`, `M/sessions.ts:1187` |
| A whole-window note | its own message: `sendSessionMessage({session_id, text: requestOf(note) [+ background], attachments})` then `POST /sessions/message`; the picture rides as `annotation-<id>.png` | `send.ts:260-287`, `M/sessions.ts:1149` |
| Pictures the person added | one more message after the turn: `Pictures he added to show what he means, for the notes in the last message:` plus a line per note | `send.ts:309-332` |
| Job facts | the open Job's labelled facts ride on the first thing sent, after his words, unless the target is that Job; a whole-window note leads them with `Background, not the task: the Job he had open when he pinned this` | `send.ts:124-125`, `:204`, `:241-269` |
| After a send | each note Fleet took is saved with `sent`; the Job opens (`openSession`); a refusal is told as `Notes not sent to "<title>": <Fleet's sentence>` | `Layer.tsx:938-957`, `send.ts:288-300` |
| Without Bridge's Fleet (the mock) | `Sending needs Bridge and its Fleet; in the mock the note stays a file` | `send.ts:91-96` |

The sheet is not opened: `R/annotate/` imports no dispatching hook, and the Job is started held on its start card.

### 2.6 Keys and states

| Key | Does | Where |
|---|---|---|
| ⌥⌘A | toggles the layer (unless a Job browser pane is shown and the layer is off) | `Layer.tsx:75-82`, `:354-364`, `R/job-browser/chord.ts:16-24` |
| ⌥⌘S | whole-window note, while on | `Layer.tsx:84-91`, `:366-376` |
| Esc | cancels the draft and closes the opened note | `Layer.tsx:1039-1043` |
| ⌘↵ | saves the draft | `Layer.tsx:1044-1048` |

| State | Drawn as | Where |
|---|---|---|
| Off | nothing | `Layer.tsx:1051` |
| Hovered | outline with the component name | `:1108-1113` |
| Draft | held outline, `New note` card; Save note disabled until words are typed | `:1143-1206` |
| Pinned open or done | numbered pin; `data-status`, `data-sent`; a done pin only on the Done tab | `:1073-1074`, `:1124-1139` |
| Sent | `Sent to Job <title>` and a Job card (`IssueJobCard`, or `JobCard` with `Not held by this window` when the window does not hold the Job) | `:196-207`, `:1243-1245` |
| Cannot send | meta line with the mock sentence above | `:1246-1248` |
| Off this screen | a pin with no box is not drawn; the status adds `, <n> not on this screen`; a row tooltip reads `Show on this screen` or `On another screen` | `:1079`, `:1608`, `:1466-1467` |
| Sending | the split is disabled while any send is out | `:1258`, `:1615` |
| No picture | `No screenshot: on another screen`, or `No screenshot for <n> of <m>: on another screen` | `:217-221` |
| Delete armed | `Press again to delete` | `:1297`, `:1411`, `:1576` |
| Empty panel | `No notes` | `:1419` |

### 2.7 What the native walk capture already has

The native walk window already draws Capture (aim or box, a numbered ring burned into the window picture, held notes, Send notes).
It follows Electron's walk and Studio capture, not the dev layer: its comment says it matches `StudioCapture` and `burn.ts` (`BN/walk/capture.rs:1-5`).
It exists only inside the walk window; its keys are scoped `walk` (`BN/walk/window.rs:33-43`).

| Electron annotation feature | Native walk capture | With path:line |
|---|---|---|
| ⌥⌘A toggles Capture | partial: bound as `alt-cmd-a` to `TakeNote`, in context `walk` only, so not in the main window | native `window.rs:39`, `:450-458`; Electron `Layer.tsx:75-82` |
| Esc leaves Capture | present: `escape` to `CancelNote` in `walk-note` and `walk` | native `window.rs:41-42`, `:468-476` |
| Aim at an element | partial: the smallest drawn, labelled element under the pointer, from the accessibility labels, within the screen area; no component or selector | native `capture.rs:63-69`, `window.rs:478-488`; Electron `Layer.tsx:465-494` |
| Draw a box | present: a drag past 4px holds the box; Electron's dev layer has none | native `capture.rs:12`, `:76-83`, `window.rs:499-503` |
| Name of what is held | partial: role word and name, or `A box you drew` | native `capture.rs:31-39`; Electron `Layer.tsx:1745-1766` |
| Note card | partial: auto-growing textarea, placeholder `What should change here`, ⌘↵ holds the note; no tools, no pictures | native `window.rs:526-535` |
| Held numbered notes | partial: held in memory, numbered by place in the send, `Note N held`; removable from the Notes menu; lost with the window | native `window.rs:105-117`, `:539-562`, `:409-431`; Electron `Layer.tsx:1062-1072`, main `held-notes.ts:57-158` |
| Ring and number burned into the picture | present: whole-window PNG, ring and a number pill | native `capture.rs:101-140`; Electron `M/capture/burn.ts` |
| Send notes as one turn | present: `Send notes`, tooltip `Sends every held note to the Job as one turn`; ⌘↵ sends while any are held | native `window.rs:241-251`, `:269-317`, `:739-744` |
| Send route | partial: `POST /sessions/notes` only, and `/sessions/message` for Approve | native `fleet.rs:62-90`, `:57`; Electron `send.ts:244-287` |
| Send target picker | missing: the walk answers to its own Job | native `fleet.rs:11-17`; Electron `Layer.tsx:1254-1281` |
| Start a held Job from notes | missing | Electron `send.ts:164-195` |
| Whole-window note (⌥⌘S) | missing | Electron `Layer.tsx:614-639` |
| Notes panel, tabs, search, selection | missing | Electron `Layer.tsx:1303-1587` |
| Style-change tools and example pictures | missing | Electron `Tools.tsx:247-337` |
| Persisted notes | missing: Electron's dev layer keeps files under `.armada/annotations`; its walk capture keeps held notes in `userData` | `M/annotations.ts:103-105`, `M/capture/held-notes.ts:1-6` |
| Component, selector, owners, source in the note | partial: only `role "name"` as selector and element text | native `fleet.rs:70-73`; Electron `shared/pinned-note.ts:57-80` |
| Outside a walk, anywhere in the app | missing | native `window.rs:33-43`; Electron `main.tsx:213-216` |

Electron's Studio capture (⌥⌘C, registry `capture_note`, ships) is the shipped counterpart: it lands a Note on the open Studio (`actions.toml:81-91`, `R/capture/Layer.tsx:1-7`).

## 3. The merge queue strip

### 3.1 Which strip, and which Fleet view it draws

| Piece | What | Where |
|---|---|---|
| The strip | one row along the window's bottom edge: a merge button, main's light, a chip for each pull request, nearest main first | `DK/Deck.tsx:1148-1432` (`MergeQueue`) |
| Its mount | drawn into the shell's status slot `.armada-shell__status` through a portal; stays in the deck where the shell has none | `DK/Deck.tsx:1122-1135`, `:2254-2256` |
| Several repositories on All | a stack of rows, one per repository, scrolling down and snapping to a row | `DK/Deck.tsx:1366-1398`, `:1442+` (`QueueStack`) |
| Main's light card | red bands, checkout, forge age | `DK/MainLight.tsx:75-187`, `DK/forge.tsx` |
| Hover card on a chip | the ledger preview | `DK/chip-card.tsx:24-167` |
| The Merge line panel | the same line as a page of tiles, on the rail | `R/merge-line.tsx:40-271`, `R/App.tsx:981`, `PC/compositions/MergeLine/MergeLine.tsx` |
| By-hand pull request | the Manifest form's line and panel | `R/merge-by-hand.tsx:49-113` |
| Mock | five pull requests at each queue state | `R/mock/scenarios/merge-queue.ts:108-123` |

Electron draws neither Fleet view.
`viewsOf` folds `state.mergeLines` with `mergeLineViews` (`R/merge-line.tsx:25-28`, `PS/merge-line.ts:98-134`), the older `get_merge_lines` read that `DPP/the-merge-line-fleet-reads.md:8` says Electron keeps.
The merge line view (PR #3367) is `get_merge_line_view` and `merge_line_view.changed` (`IPC/merge_line_view.rs:19-45`); its TypeScript mirror is `PR/merge-line-view.ts:18-19` and the event is typed at `PR/events.ts:80-83`, and nothing under `apps/desktop/src` reads either.
The calls (PR #3379) are `get_calls`, `calls.changed` and `open_call` (`IPC/standing_calls.rs:59-110`, `PR/standing-calls.ts`); nothing under `apps/desktop/src` reads them either.
Electron works its calls out itself: `callingHomes` and the keys `main:<root>`, `checkout:<root>`, `pull:<number>` (`DK/homes.ts:19`, `DK/Deck.tsx:1154`, `:1205`, `R/cockpit/answers.ts:132-140`), which are the same keys Fleet's calls carry (`IPC/standing_calls.rs:41-44`).

The panel hides when the layout's `dashboard.panels` omits `merge-line` and when no repository is served a line (`DK/Deck.tsx:1159-1160`); no sentence about the absence is drawn.
The rail row and palette entry are left off the same way (`R/merge-line.tsx:31-37`, `R/App.tsx:269`).
`q` (registry `deck_panel`, scope `deck`) moves focus to the queue panel (`actions.toml:536-547`).

### 3.2 Layout

```
.armada-shell__status
  .deck.deck-statusbar [data-power]
    section.deck-panel.deck-queue [aria-label="Merge queue" data-band-only tabIndex=-1]
      Standby (loading)
      .deck-queue__row
        Merge line button [ghost sm iconOnly, GitMerge 16px, aria-label "Merge line"]   only with onGoTo
        ol.deck-chips                                single repository
          li main light                            .deck-chip[data-main] or MainLight
          li ol.deck-queue__track [aria-label="Merge queue"]   the pull requests in the forge's queue, by place
          li .deck-queue__rule                     between the queue and the rest
          li chip*                                 the rest, the open Session's first
          li .deck-queue__gap                      between repositories
        QueueStack                                 on All with several: one ol.deck-chips per repository, main light first
        PanelKey "queue"
```

Source: `DK/Deck.tsx:1400-1431` (one), `:1384-1397` (stack), `:1336-1365` (queue, rule, rest).
On All with several repositories served and a Job or pull request selected, only that repository's row draws; with none selected, every one (`DK/Deck.tsx:1162-1165`).
A draft pull request is left off unless it is in the queue or the open Job holds it (`:1337-1339`).
The open Job's own pull requests lead the rest (`:1343-1345`).

### 3.3 Sizes and tokens

| Part | Value | CSS |
|---|---|---|
| Status strip | full width; top rule only, `--border-default`; no radius, no shadow; ground `--bg-sunken`; bottom padding `--space-1` 4px | `DK/deck.css:2874-2889` |
| Row | min-height `calc(var(--h-control-sm) + var(--space-1))` = 28px; padding `0 var(--pad-page)`; gap `--space-2` 8px | `:2890-2894` (base `:932-939`) |
| Stack | height 28px; vertical scroll snap; scrollbars hidden | `:914-930`, `:2895-2897` |
| Chip list | gap `--space-3` 12px; no wrap; scrolls across, wheel-down moves it sideways | `:940-956`, `DK/Deck.tsx:1412-1416` |
| Chip | height `--h-badge` 20px; padding `0 --space-4 0 --space-3` = 0 16px 0 12px; gap 8px; 1px `--border-glass`; pill radius 10px; mono `--text-2xs` 12px; ground 6% `--fg-default` | `:982-997` |
| Chip hover (buttons) | 12% ground, text `--fg-default` | `:1001-1004` |
| Chip selected | border `--select-edge` or `--fg-default`; ground `--select-ground` or `--bg-hover`; weight 600 | `:1010-1016` |
| Chip related | 22% ground, weight 600 | `:1017-1025` |
| Queue track | gap 8px; padding `2px 4px`; 1px `--border-subtle`; pill radius `calc(var(--h-badge) / 2 + var(--space-1))` = 14px; ground `--chip` | `:958-968` |
| Rule | 1px by `--h-badge` 20px, `--border-default` | `:969-974` |
| Gap between repositories | `--space-4` 16px; 8px inside the list | `:908-911`, `:979-981` |
| Place number | `--fg-default`, weight 600 | `:975-978` |
| Dot | `--dot` default `--fg-subtle`; ring (inset 1px) when open, filled when `data-queued`; size `--lamp` or `--dot` | `:1046-1058` |
| Dot colours | blocked `--status-escalated`; running `--status-running`; ready `--status-completed-success`; failing `--status-completed-failed`; tone ask `--status-awaiting-review`; tone bad `--status-completed-failed`; main calm `--status-completed-success`; stale 40% | `:1059-1088` |
| Calling | 6% `--status-awaiting-review` wash and the dot pulsing `deck-lamp-pulse` over 1.6s; stops under reduced motion | `:1026-1045` |
| Quiet mark | `Unplug` 12px, `--status-escalated` | `:1089-1093` |
| Checkout mark | `FolderGit2` 12px, `--status-awaiting-review` | `:1137-1141` |
| By-hand mark | `UserCheck` 12px, `--status-awaiting-approval` | `:1143-1147` |
| Rotation | the stack moves to the next repository every 15,000ms while pointer, focus and wheel are elsewhere; a main turning red brings its row into view; ↑ and ↓ move it by hand | `DK/Deck.tsx:1434-1440` |

### 3.4 Controls

| Control | Accessible role and name | Glyph and tooltip | What a press does | Shown when |
|---|---|---|---|---|
| Merge line button | button, `Merge line` | `GitMerge` 16px; tooltip `Merge line` | `onGoTo(DECK_LINK.queue)`: goes to the Merge line page | `onGoTo` given; else the glyph alone |
| Main, green | `img`, `Main is green` / `Main is green in <name>` (plus `, <checkout sentence>`) | dot; tooltip = the sentence, then `Read <age> ago` | none | no red, no held checkout |
| Main light, red or held | `LightCard` chip named by the same sentence | dot `bad`; `Unplug` when the forge is quiet; `FolderGit2` for a held checkout | hover or focus opens the card; a press pins it | a red main or a held checkout |
| Pull request chip | button, `aria-label` = the dot's tip (`#<n> <title> · <branch> · <state>, place <n> · Needs merging by hand · Job: <title>`) | place number, dot, `#<n>`, `UserCheck` for by-hand | `openPullRequestInPanel(pr)`: opens it in the side panel; the chip carries `PullRequestMenuOn` handlers | the pull request is held by the ship |
| Landing or unowned chip | `img`, same name | same | none | no ship pull request |
| Hover card | `Inspect` with `LedgerPreview`; key hint `Enter` `Open` on a pressable chip | rows in 3.6 | the Job row opens the Job | hover or focus |

### 3.5 States

A pull request's `state` is `queued`, `running`, `failing`, `ready`, `blocked` or `open` (`R/cockpit/horizon.ts:16`).

| Source | `state` | Words | Where |
|---|---|---|---|
| `queue.state` `waiting_for_ci` | running | `Waiting for ci to join the merge queue` | `horizon.ts:36-39` |
| `in_queue` | queued | `In the merge queue` | `:40-43` |
| `queued` | queued | `In the merge queue, waiting its turn` | `:44-47` |
| `awaiting_checks` | running | `In the merge queue, running its checks` | `:48-51` |
| `mergeable` | ready | `In the merge queue, ready to merge` | `:52-55` |
| `unmergeable` | failing | `In the merge queue, cannot merge` | `:56-59` |
| `ci` `passed` | ready | `ci passed` | `:69-72` |
| `ci` `running` | running | `ci running` | `:73-76` |
| `ci` `failed` | failing | `ci failed` | `:77-80` |
| `ci` `waiting_on_main` | blocked | `ci red because main is` | `:81-84` |
| no queue, no ci | open | `Open` | `:262` |

The mark for each state is the registry's: `queued` is `LAND_STATE.waiting` (`Clock`), `running` is `LAND_STATE.gating` (`CircleDot`), `failing` is `CHECK_OUTCOME.failed` (`ShieldX`), `ready` is `CHECK_OUTCOME.passed` (`ShieldCheck`), `blocked` is `QUEUED_REASON.blocked_by_dependency` (`Link`), `open` has none (`horizon.ts:19-26`, `PC/generated/vocabulary.ts:34`, `:38-39`, `:81`, `:82`).
The chip's tone is `bad` when the ship's pull request stage is `red`, `ask` when its owner's pull request is mergeable, else `calm` (`DK/Deck.tsx:1271`).
A landing entry (`view.line`, always empty since `armada land` was retired) maps `waiting` to queued, `preparing`, `gating` and `merging` to running, `held` to blocked, `landed` to ready, and `red`, `conflict` and `stopped` to failing (`horizon.ts:312-322`).

| Strip state | Drawn as | Where |
|---|---|---|
| Loading | `Standby kind="queue"` over the panel | `DK/Deck.tsx:1391`, `:1407` |
| None served, or the layout hides it | nothing | `:1159-1160` |
| Main green, forge quiet | dot at 40%, outline only, `Unplug` mark, tooltip `GitHub not answering since <time>` | `forge.tsx:50-59`, `deck.css:1082-1088` |
| Several repositories quiet | `GitHub not answering for <name> since <time>, …` | `forge.tsx:58` |
| Reading age | tooltip tail `Read just now`, `Read <n>s ago`, `Read <span> ago`, ticking every 5s | `forge.tsx:61-70` |
| Main red | the light opens a card of the Merge line's red band for each red repository, then the held checkout, then forge age and quiet | `DK/MainLight.tsx:153-184` |
| A call stands for the chip | `data-calling`: the wash and the dot pulse | `DK/Deck.tsx:1319`, `deck.css:1026-1045` |
| Selected pull request | `data-selected` when the side panel shows it | `DK/Deck.tsx:1312-1317` |
| Related to the open Job | `data-related` | `:1318` |

### 3.6 Copy strings

| String | Where |
|---|---|
| `Merge queue` (section and track name and tooltip), `Merge line` (button) | `DK/Deck.tsx:1351-1352`, `:1387`, `:1174-1179` |
| `Main is red in <names>`, `Main is red in <name>`, `Main is green`, `Main is green in <name>` | `:1211` |
| `Needs merging by hand` | `R/cockpit/horizon.ts:88` |
| `Job: <title>` (inside a tip) | `:247` |
| `, place <n>` | `:238` |
| Card rows: `Author`, `Checks`, `Place` (`Turn` on a landing), `Opened`, `Updated`, `In queue`, `Branch`, `Review` (`Approved`, `Changes requested`, `Required`), `Main` = `Conflicts`, `Merge` = `Needs merging by hand`, `Job` | `DK/chip-card.tsx:18-22`, `:40-145` |
| Card state: the words above, `, draft` appended for a draft | `chip-card.tsx:32` |
| `Failed: <checks>`, `Running: <check>` (a landing's check line) | `horizon.ts:327`, `:329` |
| `GitHub not answering since <time>`, `Read just now` | `forge.tsx:57`, `:63` |

The red band (the card, and the Merge line page) says:

| String | Where |
|---|---|
| `Main is red`, `New checks are running on main:` (status aria the same) | `PC/compositions/MainHead/MainHead.tsx:288`, `:292-314` |
| `Main is green`, `Main is green, new checks are running`, `Main is red, new checks are running` (the mark) | `:152-159` |
| Rows `Check` (`CI job` where unmapped), `Test`, `Broke in`, `Claim`, `Fixing` | `:355`, `:371`, `:391`, `:438`, `:454-460` |
| Buttons `Dispatch a new Job`, `Send back to…`, `Send back`, `Take over` | `:329`, `:338`, `:430`, `:274` |
| Dialog `Send the work back`, confirm `Send back`, group `Jobs`, field `Message` | `:758-795` |
| Dialog `Take over from <machine>`, confirm `Take over` | `:571`, `:581` |
| Refusals `Not made, store unreachable`, `Held on another machine` | `:137-140` |
| Tooltips `Working on it`, `Claimed`, `Claimed, stale`, `Merged #<n>, the pull request that turned main red` | `:462-467`, `:544`, `:743` |

What the band's acts send: a Job already open gets `commands.sendToSession(id, text)`; a new Job is `dispatching.startDraft(request, root)`, held on its start card with the sheet never up (`R/merge-line.tsx:113-123`, `R/App.tsx:984`).
The brief text is built in `MainHead.tsx:679-693` (`briefOf`: `<check> fails on main.`, `Test: <test>`, `Merged in #<n> (<branch>).`).
The held checkout card's contents are in `DK/YourCheckout.tsx` and are not inventoried here.

### 3.7 The Merge line panel on the rail page

The page draws one `MergeLine` section per repository served a line, each folding on its own (`R/merge-line.tsx:58-106`, `:144`).
`aria-label` is `Merge line` or `Merge line, <repository>`; the fold button is `Collapse <name>` or `Expand <name>` with `ChevronUp` or `ChevronRight` 14px (`PC/compositions/MergeLine/MergeLine.tsx:298`, `:333-345`).

| Part | Value | Where |
|---|---|---|
| Panel | 1px `--border-default`, radius `--radius-lg` 8px, ground `--bg-sunken`; main red adds a `--status-completed-failed` border and glow | `MergeLine.css:9-18`, `:56-62` |
| Head | min-height 35px; padding `4px 12px 4px 16px`; ground `--bg-raised` | `:66-75` |
| Horizon | padding `--space-4` / `--space-6`; belt gap 24px; dots 16px with a 2px ring, filled when in the turn; main's light 24px | `:140-250` |
| Body | padding 16px; gap `16px 24px`; ruled grid 24px | `:255-273` |
| Lane and side columns | six-track subgrid; the lane `flex: 3 1 480px`, the side `flex: 2 1 336px` | `:279-307` |
| Tile | padding `8px 12px`; 2px border mixing 40% state hue into `--border-subtle`; radius 6px; `--text-sm` 14px / 20px | `:382-399` |
| Lit tile | red or conflict: hue border, 16% fill, glow | `:410-415` |
| Landed tile | receded, hue on the leading edge | `:418-422` |
| Focused tile | `aria-current`: `--accent` border, `--accent-muted` fill | `:425-428` |

Sections and headings: `In line`, `Open pull requests`, `Recently landed`, `Sent back`, and `Batch` for a bracketed group; an empty panel draws a picture named `Empty` and no words (`MergeLine.tsx:244`, `:260-266`, `:403`, `:427`, `:410`, `:750-758`).
Row states and words (`LAND_STATE`): `waiting` `Clock`; `preparing` `Preparing to land` `GitMerge`; `gating` `Running Checks before landing` `CircleDot`; `held` `Held, red on main` `Pause`; `merging` `Pushing onto main` `CircleDot`; `landed` `Check`; `red` `Checks failed` `X`; `conflict` `Unplug`; `stopped` `Power` (`PC/generated/vocabulary.ts:80-90`).
Waiting reasons: `Left out of this turn: clashes with another branch in it`, `Left out of this turn: clashes with main`, `Kept its place after a red, next up`, `Joined after the turn began` (`MergeLine.tsx:1298-1313`).
Notice: `<check> failed` or `<check> red on main`, `Open log`, `sent back`, `rerun failed`, `green on main` (`:513-559`).
Pull request rows: `Place in the merge queue` (`#<n>`), `Pull request`, `Order to merge in`, `Merge commit`, `ci passed`, `ci running`, `ci failed`, `ci red because main is`, `, waiting on the fix: <title>` (`:908-936`, `:1088-1113`, `:817`).
Main's light name is `main: <state>`, with `not read yet` before a read, and `, new checks are running` (`:628-632`, `:738`).
The panel's keys are tiles reached with the arrows: one tab stop for every repository's tiles (`R/merge-line.tsx:239-244`).

## 4. Write-clash marks

### 4.1 Where it lives

| What | File |
|---|---|
| The mark, its tooltip and card, the paths list | `PC/compositions/ClashMark/ClashMark.tsx` (`ClashMark` 69-133, `ClashPaths` 139-192), `ClashMark.css` |
| What the window holds: grouping a list by holder, the provider | `R/clashes.tsx` (`clashingOf` 16-35, `ClashesFrom` 44-67, `ClashesFromFleet` 70-79) |
| Reading the list from Fleet and when to read again | `R/fleet-clashes.ts` (`createFleetClashes` 37-100, `SETTLE_MS` 13) |
| Opening the other holder | `R/open-holder.ts` |
| The read, per Manifest | `M/clashes.ts` (`ClashesHost.get` 45-74, `file` 77-107, `fileChange` 110-136) |
| The API and channels | `AD/shared/api/clashes.ts` |
| The wire types | `PR/clashes.ts`, `IPC/clashes.rs` |
| Mounted once | `R/main.tsx:127` (`createFleetClashes`), `:171` (`ClashesFromFleet`) |
| Mock | `R/mock/scenarios/write-clash-mark.ts`, `write-clash-paused.ts`, `R/mock/slices/clashes.ts` |

### 4.2 Where the mark is drawn

| Where | How | Source |
|---|---|---|
| Jobs list row | `ClashMark kind id` in `.deck-row__clash`, left of the linked rails; not for a pull request row; only while the View menu's Conflicts is on (the default) | `DK/Deck.tsx:735-739`, `../native-jobs-inventory/list-needs-palette.md` |
| Bays tile | the row's wrong-state list gains `Write clash with <labels>` | `DK/SessionTiles.tsx:402-420` |
| The Job's view strip | the slot layer's view draws a `sv-view__clash` dot named `Write clash with <labels>` | `DK/JobViews.tsx:68-95` |
| Fleet map | a pressable hit named `Conflicts with <labels>` or `clashSaid`; a button when one clash with a Job, an image otherwise | `DK/pixi/Hits.tsx:261-300` |
| Job thread | a Fleet message with `notice: "write_clash"`, named `write clash` in the notice menu | `PC/compositions/SessionThread/FleetTurn.tsx:71`, `SessionThread.tsx:169` |

The mark sits on both rows of a pair (`ClashMark.tsx:9`).

### 4.3 The mark

| Part | Value | Where |
|---|---|---|
| Glyph | `Swords`, 12px (16px on request), stroke 2 | `ClashMark.tsx:96`, `:105`, `:69-77` |
| Colour | `--status-awaiting-review`; `--fg-default` on hover of a pressable mark | `ClashMark.css:4-19` |
| One clash with a Job | `button`, `aria-label` and tooltip `Conflicts with <label> · <path>, <path>`; press stops propagation and opens the other Job | `ClashMark.tsx:61-62`, `:114-128`, `R/open-holder.ts:9-15` |
| One clash with main | `img`, same name and tooltip; no press, since there is nothing of main to open | `:107-112` |
| Several clashes | one mark, `img`, name `Conflicts with <label>, <label>`; tooltip card: a `Conflicts with` heading then `<label> · <paths>` per holder; a press is the row's own | `:81-100` |
| Nothing | no mark and no space | `:80` |
| Paths list (Bays panel) | list named `Conflicting files with <label>`; three paths, then a chip `<n> more` that toggles to `Fewer`; a list of one more than three is drawn whole; with `onPath`, each path is a button `Open <path>` | `:135-192` |

### 4.4 What Electron reads and when

| Fact | Value | Where |
|---|---|---|
| Route | `GET /clashes?manifest_id=<id>` once for each served Manifest; bare it answers only the first served repository | `M/clashes.ts:1-3`, `:53-59` |
| Route per file | `GET /clashes/file?session_id&other_kind&other_id&path`; each side as `git diff` prints it | `M/clashes.ts:91-97`, `PR/clashes.ts:33-42` |
| Route per Job file | `GET /sessions/file_change?session_id&path[&seen]` | `M/clashes.ts:121-126` |
| A Fleet before 23.81 | a transport 404 reads as no clashes, not a refusal | `M/clashes.ts:27-33`, `:62` |
| When | on connect, 35,000ms after the board changes, and 35,000ms after any read that found a clash; at most one read pending; a lost connection clears the list; a failed read leaves what stands | `R/fleet-clashes.ts:1-6`, `:13`, `:49-74` |
| Why polled | Fleet pushes no event for a clash and finds one 30s after the writes that made it | `R/fleet-clashes.ts:1-4` |
| Wire shape | `{one: {kind, id, label}, other, paths (sorted), since}`; `kind` is `session` or `main` | `PR/clashes.ts:6-26` |
| Fleet-side filter | `GET /clashes?manifest_id=&holder=` lists those one holder is in | `IPC/clashes.rs:42-45` |

### 4.5 The paused state

A Job whose changes conflict with a paused Job's gets Fleet's notice in its thread: a message from `Fleet` with `notice: "write_clash"` that names the paused Job's branch and says it is paused, with no instruction.
The mock text is ``Git says your changes and branch mock/video-multitasking-boards (Job s-4eb87a78, paused) write the same lines of `apps/desktop/src/renderer/src/cockpit/deck/Deck.tsx`.`` (`R/mock/scenarios/write-clash-paused.ts:26-27`, `:73-80`).
Fleet makes that sentence: `branch <branch> (Job <address>, paused)`, `Job <address> (paused)`, and for main `Git says <base> and your changes write the same lines of <paths>[, landed in <ref>].` (`FL/clashing/telling.rs:30-57`); it names three paths, then `and <n> more` (`:13-22`).
The paused Job is drawn as any other: the mark does not change for a pause, and a press opens that Job.
The paused Job's own record carries `hosted.paused {at, resuming}` (`write-clash-paused.ts:42-52`).

| State | Drawn as |
|---|---|
| A Job pair | the mark on both rows |
| A Job against main | the mark on the Job's row only |
| Two Jobs on one file with no conflict | not marked (`R/mock/scenarios/write-clash-mark.ts:1-5`) |
| Paused other side | the same mark; the notice in the thread names it paused |
| Not connected | the list clears, so no mark (`R/fleet-clashes.ts:68-72`) |
| Provider with no list | nothing marked (`R/clashes.tsx:60-62`) |

## 5. Worked out in TypeScript

Each row is a value Electron works out in TypeScript rather than drawing from Fleet.
"Fleet already sends it" says whether the value is already on the wire, with the path.

### 5.1 Dispatch sheet

| Piece | TS path:line | Fleet already sends it? |
|---|---|---|
| Kinds offered and the default kind | `DP/dispatching.tsx:198-213` | missing; a window choice that depends on whether the window can start an Autopilot |
| Which repository the words name | `DP/reading.ts:109-168` | yes: `manifest_id` and the best-first `likely` list, `IPC/repositories.rs:69-79`; the 250ms rest and stale-answer rule are TS (`reading.ts:13`, `:121-132`) |
| Repository order within the ranking | `DP/reading.ts:27-46` | partly: Fleet ranks every served repository (`repositories.rs:75-78`); TS adds an alphabetical tiebreak for unranked ones and marks only the head `likely` |
| Chip order: rail's repository, the one the words name, the last picked, the rest | `DP/reading.ts:52-91` | missing; the rail pick is window state, the recent list is `localStorage` (`SH/recent-repositories.ts:7`, `:10`) |
| Which chip carries a key (first nine) | `DP/DispatchPanel.tsx:93`, `:391-392` | missing |
| Chosen repository: picked, else the rail's, else the one read | `DP/dispatching.tsx:582` | missing |
| The six reading states and their words | `DP/dispatching.tsx:587-607`, `DP/DispatchPanel.tsx:95-126` | missing; Fleet sends only presence of `manifest_id` |
| A repository's label | `SH/repository-label.ts:21-30` | missing for a repository list: `RepositorySummary` carries no label (`repositories.rs:44-57`); present for the merge line (`merge_line_view.rs:26-28`) |
| Which Job a brief tags | `DP/waits.ts:247-291` (longest title first, regular expression on `@<title>`) | missing; `read_session_wait` takes the Job id already chosen (`session_waits.rs:110-116`) |
| The wait's condition, base and needs | `R/sessions-wired.tsx:740-775` | yes: `condition`, `base`, `needs`, `plain` (`session_waits.rs:120-131`) |
| Condition to the sentence's `when` and plan part | `PS/session-waits.ts:226-257` | missing; Fleet sends a `StartCondition`, TS maps it using the tagged Job's plan |
| Default base when no read came | `PS/session-waits.ts:44`, `R/sessions-wired.tsx:694-705` | partly: Fleet's read carries `base`; the fallback default is TS |
| The sentence's cycles and the person's hand over it | `DP/waits.ts:132-244` | missing; pre-start choices are client state (`change_waiting` is for a Job already waiting, `session_waits.rs:150+`) |
| `needs` split on `and` | `DP/waits.ts:105-114` | missing |
| The sentence's words | `DP/waits.ts:72-102`, `DP/WaitChip.tsx:12-17` | missing |
| The wire `after` built from the chosen sentence | `R/sessions-wired.tsx:787-798`, `PS/session-waits.ts:190-224` | missing; Fleet takes `StartAfter` (`session_waits.rs:52-65`) and TS builds it |
| `@` and `#` rows, filtering, ordering and caps | `DP/SessionMenu.tsx:52-123`, `:321-352` | partly: files come from Fleet's `search_files` (`M/command.ts:84`); filtering, order and the caps of 5 and 6 are TS |
| Model list | `R/sessions-wired.tsx:1617-1619` | yes: held `models` |
| Effort list | `R/sessions-wired.tsx:1623-1629` (`low medium high xhigh max`) | yes per model: `OfferedModel.efforts`, `IPC/setup.rs:443-445`; Electron does not use it |
| Large paste becomes a file | `PC/compositions/SessionComposer/large-paste.ts:5-10` | missing |
| Queued ideas | `R/queued-ideas.ts:12-26`, `R/sessions-wired.tsx:1359-1366` | missing; no queued-idea type on the wire |
| Held on its start card for a Job with no wait | `DP/dispatching.tsx:421-425` | yes: `StartSession.held`, `IPC/hosted_sessions.rs:68-97` |
| Autopilot read to fields and fields to settings | `R/cockpit/autopilot.ts:115`, `:173`, `:245` | yes: `AutopilotProposal` with `quoted`, `IPC/autopilot.rs:158-195`; mapping to the form fields is TS |
| New workflow brief | `DP/dispatching.tsx:65-74` | missing |
| Act word and which acts are disabled | `DP/dispatching.tsx:694`, `DP/DispatchPanel.tsx:654-666` | missing |
| Strip title | `DP/dispatching.tsx:507-510` | missing |

### 5.2 Annotation layer

| Piece | TS path:line | Fleet already sends it? |
|---|---|---|
| Element reading: component, owners, selector, source, nearby, styles | `R/annotate/capture.ts`, `fiber.ts`, `selector.ts` | not applicable to a native tree; Fleet only receives the note's `place` (`shared/pinned-note.ts:57-80`) |
| Note numbering and which notes a send takes | `Layer.tsx:1062-1072`, `:967-972` | missing |
| Which Jobs the picker lists | `Layer.tsx:427-445` | partly: sessions are on the wire; the live and listening filter is TS |
| New Job's words | `send.ts:173-176` | missing; Fleet's proposer names the Job from the words (`send.ts:177`) |
| Ring box on the picture | `AD/shared/ring.ts` (`ringOn`, `pngSize`), `send.ts:63-66` | missing |
| Open Job's facts as background | `R/job-facts.ts`, `send.ts:124-125`, `:204` | missing |
| Which repository | `M/annotations.ts:77-101` | missing; git is asked in main |

### 5.3 Merge queue strip

| Piece | TS path:line | Fleet already sends it? |
|---|---|---|
| Pull request order: the queue by position, then waiting for ci, then the rest | `PS/merge-line.ts:353-376` (`queueFirst`) | yes: `MergeLineView.pulls`, `IPC/merge_line_view.rs:34-40`; Electron does not read it |
| Another author's pull request: listed only in the queue, quiet, state `in_queue` | `merge-line.ts:230-232`, `:332-351` | yes: `merge_line_view.rs:34-40`, `the-merge-line-fleet-reads.md:26-30` |
| Main: green, red with the failed job, or no mark | `merge-line.ts:378-426` | yes: `MainView`, `merge_line_view.rs:60-95` |
| A failed job is its Manifest Check, else its own name with `unmapped` | `merge-line.ts:429-466` | yes: `MainRedJob`, `merge_line_view.rs:98-106` |
| The Job that merged the red and the Job taking it, with titles | `merge-line.ts:407-414`, `:482-488` | yes: `merge`, `taken_by` always titled, `merge_line_view.rs:76-90` |
| Repository name widened on a clash | `merge-line.ts:495-499`, `SH/repository-label.ts:21-30` | yes: `name`, `merge_line_view.rs:26-28` |
| Which lines to draw for the pick, and naming only when more than one | `merge-line.ts:103-112` | missing by design: "The person's repository pick stays a client filter" (`the-merge-line-fleet-reads.md:41`) |
| Merged commit cut to 10 characters | `merge-line.ts:88`, `:193` | no: Fleet sends it whole and a screen cuts it (`merge_line_view.rs:6-7`) |
| Landed list from the forge's merged pulls | `merge-line.ts:118-121`, `:182-215` | yes: `landed`, `merge_line_view.rs:41-43` |
| Land runner lists (`line`, `sentBack`) and their `preparing` and `held` states | `merge-line.ts:73-85`, `:507-511` | gone: always empty since `armada land` was retired (`merge_line_view.rs:11-12`) |
| Strip dots: state, queued, tone, tip, card | `R/cockpit/horizon.ts:216-271`, `DK/Deck.tsx:1271` | missing; the states and the words are TS |
| Queue wording and ci wording | `horizon.ts:29-85`, `PC/compositions/MergeLine/MergeLine.tsx:908-986` | missing by design: "States, not words" (`merge_line_view.rs:6`) |
| Review, draft, conflicts, times on the card | `horizon.ts:110-152` | yes, raw: `review`, `draft`, `mergeable`, `opened_at`, `updated_at`, `queue.enqueued_at` on `HubPullRequest` (`IPC/merge_hub.rs`); the mapping to words is TS |
| By-hand pull request, `merge_by_hand` | `R/merge-by-hand.tsx:57-60`, `:14` | yes: `HubPullRequest.merge_by_hand`, `IPC/merge_hub.rs:348`; the ci mapping `checksOf` is TS |
| Chip order: queue by place, then the rest with the open Job's first; drafts hidden | `DK/Deck.tsx:1336-1365` | missing |
| Which Job owns a pull request | `R/cockpit/owner.ts` (`pullOwner`), `horizon.ts:231` | missing |
| Main light sentence | `DK/Deck.tsx:1211` | missing |
| Oldest read and quiet repositories | `DK/forge.tsx:19-48` | partly: `read_at` and `quiet_since` per repository are sent (`merge_line_view.rs:44-48`); the minimum and the sentence are TS |
| Held checkout per repository | `DK/YourCheckout.tsx:32` | yes: `checkout`, `merge_line_view.rs:44-45` |
| Calls that pulse chips (`main:`, `checkout:`, `pull:`) | `DK/homes.ts:19`, `R/cockpit/answers.ts:132-140` | yes: `get_calls` and `calls.changed`, `IPC/standing_calls.rs:59-110`; Electron does not read them |
| Rotation of the stack | `DK/Deck.tsx:1434-1440` | not applicable; a presentation rule |
| Sessions main's red can go back to | `PS/merge-line.ts:142-179` | missing |
| The fix brief | `PC/compositions/MainHead/MainHead.tsx:679-693` | missing |

### 5.4 Write-clash marks

| Piece | TS path:line | Fleet already sends it? |
|---|---|---|
| Detecting a clash | not in TS; Fleet detects and tells both sides | yes: `FL/clashing.rs`, `FL/clashing/telling.rs` |
| The list of clashes across repositories | `M/clashes.ts:53-69` (one read for each Manifest) | missing: `GET /clashes` needs a Manifest and bare answers one repository (`M/clashes.ts:1-3`) |
| When to read again | `R/fleet-clashes.ts:13`, `:49-74` | missing: no event for a clash (`fleet-clashes.ts:1-4`) |
| What each holder is in: the other side and the paths | `R/clashes.tsx:16-35` | partly: `GET /clashes?manifest_id=&holder=` filters by holder (`IPC/clashes.rs:42-45`); Electron reads all and groups |
| Whether a mark is drawn on a row | `DK/Deck.tsx:735`, `ClashMark.tsx:80` | missing; also gated by the View menu's Conflicts |
| One mark for several clashes, the card | `ClashMark.tsx:81-100` | missing |
| The tooltip words `Conflicts with <label> · <paths>` | `ClashMark.tsx:61-62` | missing; the thread notice words are Fleet's (`telling.rs:30-56`) |
| `Write clash with <labels>` on tiles and the view strip | `DK/SessionTiles.tsx:402-405`, `DK/JobViews.tsx:67-69` | missing |
| Path chip folding | `ClashMark.tsx:136-190` | missing; paths are sent sorted |
| Opening the other holder | `R/open-holder.ts:9-15` | not applicable; `main` has nothing to open |
