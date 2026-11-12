# Native Bridge

**What it is:** The core Bridge rebuilt natively in Rust on gpui, in `crates/bridge-native/`, beside the Electron Bridge, which stays until the owner picks the switch day.

---

**Kind:** Surface.

The Electron Bridge is described in [Bridge](bridge.md).
How a Job works on the native one is in [the native Bridge practice](../practices/native-bridge.md).

## Why native

Decided 8 Nov 2026.
The comparison page "Should Bridge leave Electron" said "not yet": opening a thread was mostly React drawing, and #3356 cut per-update redraws from 2,211 to 561.
The owner chose native anyway, for three goals at once.

| Goal | Meaning |
|---|---|
| Feel | The UI answers instantly |
| Load | A lighter weight on the Mac |
| Ownership | The stack is Armada's own |

## The seven decisions

### 1. gpui

**The native Bridge is built on gpui-component and its pinned weekly gpui snapshot, gpui-pre.**
If gpui-component's one maintainer stops, the build pins Zed's own gpui.

### 2. Electron is frozen

**Electron takes fixes and small features that help the owner daily.**
Bigger features go native.
Each Electron feature that lands during the overlap gets a task in the matching step of the build order below.

### 3. A native mod kit, after switch day

**Code mods, such as tabs and instruments, are sandboxed WebAssembly written in Rust.**
Themes do not wait for the kit.
Every theme in the app today works from the foundation: the catalogue in `packages/tokens/themes`, the built-in light and dark, and theme mods such as Dusk.
Armada's own light and dark follow macOS, whichever of the two is picked.
A catalogue theme or a mod stays as picked.
Job track, the owner's tab mod, waits for the kit.
The custom tab in a Job's + menu arrives with the kit too, since Armada Classic's unsandboxed React module cannot run in gpui.

### 4. Walks and annotations

**A note aims at a picture of the app plus labelled elements, each a role and a name.**
A walk steps through labelled elements.
The same labels are what VoiceOver reads and what tests find.
A native screen is walked in its own native window on mock data, with a walk bar.

### 5. Chrome for all web content

**Chrome (CEF) is embedded for all web content: documents, sketch mocks, walks of web pages and the Browser tool.**
One Armada Chrome profile keeps sign-ins, and the owner's own Chrome is never touched.
Chrome draws inside the gpui window: it renders windowless, and gpui shows each frame in a `surface`.
Bridge carries a small patch to gpui so `surface` takes Chrome's frames as they are, in exact colour and with alpha.
A page has no view of its own, so its accessibility tree is mapped into gpui's for VoiceOver.
What was measured is in [the Chrome spike](../spikes/native-bridge-chrome.md).

### 6. Order

**Jobs come first, then Operator, Projects and the Job tools.**
Each step is its own Job, and at most two run at once.

| Step | What it gives |
|---|---|
| Foundation | A native window on Fleet listing Jobs in his theme, the native walk window on mock data, the Chrome test and the speed check |
| Fleet views | Thread rows, lanes and the merge line worked out in Fleet, beside the foundation |
| Jobs screens | The Jobs list, thread, composer, approvals and Needs you |
| Dispatch and the title bar | The dispatch sheet, the merge queue strip, usage and context rings, the build switcher, clash marks |
| Web pages in Chrome | Documents, sketches, walks of web pages and notes |
| Job panes | Plan, Changes, Files, Checks, Subagents, Log (the Check panel's Log tab), Ledger, Judge, the pull request panes, Issues with the issue panel, Jobs, System, Bays with the bay panel, Spine, and the native diff. Issues lists issues alone, as in Armada Classic: a Job's reported bugs show in the thread's Issues block. Still owed in the issue panel: Back to the issue from a pull request opened in it, and the panel of a linked pull request the Job does not hold, which opens in the browser until its plates can be read without the Job's record. The ticket menu reads the Ticket lifecycle as the tab shows, and not again as it opens. A Job it started carries its pull request in the Ledger, Merged or Open, and its Checks, as Classic's `LinkedSession` does. Still owed in the Ledger: note thumbnails, which wait on Fleet serving a note's picture: Fleet names it by its path and serves no bytes for it. A pull request's judge notes stand in its panel's Files tab, under the head of the file they name, until the native diff places a note at its line; a doc's stand under its Ledger row until the doc reader lands. Still owed in that Files tab: his own line notes on the diff's gutter, the notes bar and Send to job; Classic's diff has no line numbers either. Still owed in the pull request panel: the merge queue's place on its plate, Fix, which needs a terminal, the tests that reach the change, and the owner's chip. Still owed in System: CPU, memory and I/O for each process, which Fleet does not report, and a Job's logs, which Fleet does not serve, so a log row opens only on mock data. Still owed in Bays: the Jobs list row lit while the pointer is on a clash's Job, the colliding lines marked in a clash's file, Open Job beside a refused release, and the hatched grounds of a stranded or closed bay, drawn as tints. Beside the thread, a draft, a page owed alone, a walk of a native screen and a subagent's thread open as tabs. Still owed there: a web page's walk, which waits on Chrome. Done lists what the Job made, By step and By kind, as Classic's `madeOf` does. The mods a Mod Job made stand in Done as Classic's `ModsMade` does, each with its kind, a switch and a press that opens its panel with Change it; pull requests stacked on one another draw beside their lane with the base and the "on #N" chip; and a workflow that lands in a report draws the report as its card. Still owed in Done: the first lines of the report's conclusion, which waits on Fleet's file read (Fleet serves native's thread fields), and Promote on a mod, which comes with the Mods page. Now says what the Job is doing as Classic's bare Now does: the lamp, Ask for an update and Plan (⌥N), then what is owed him, the walks shown to him, the agent's row, Fleet at work, what the Job waits on, the route under way and the Checks, then its open tasks, its subagents, the groups being worked with their drones, the Jobs it started that still run, a plan he has not approved, and Ahead. A drone opens its subagent's thread as a tab. A Mod Job's route is Mod, Check and On, read from the mods Fleet serves and the Job's Checks; a workflow that lands in a spike routes through Findings, and one that lands in a document or report through Document and Read. Still owed in Now: the live page, which Fleet serves no frame for. Watching stands at Now's foot as Classic's `WatchingBlock` draws it: the live watches and one that ended not ours while Needs you asks about it, Show ended for the rest with Delete beside each, and a row opens the watch's panel with Readings, Parts, Command, Output for a broken run, Log, its link out and Stop while it is live. Open lists what is still to come as Classic's bare Open does: the plan's groups still to come, shut until pressed, a group going to a Job not started yet as that Job's placeholder card, then what has not landed and no group still to finish holds, then what Fleet is to do for the Job. Deferred lists each follow-up the agent kept and he has not confirmed, as Classic's flat `LaterBlock` does: a card in Deferred's hue with Remind and two thumbs in columns of their own, and a right press or Shift+F10 offering Deferred and Not deferred. In Done, Now, Open and Deferred, Up and Down step between the rows and Home and End go to the first and last, as Classic's `arrowsIn` does. Spine draws Brief, Done, Now, Open and Deferred down one line as Classic's `Spine` does: a dot and a lamp band for each zone, the band held at the top while its zone scrolls under it, a press, Enter or Space folding the zone, and the folds kept for each Job, with Done shut at first. Brief shows only when the Job has a brief, with How it started and the Tasks the Job holds; Deferred shows only when the Job kept a follow-up, at the foot, shut until pressed. Open carries Ahead there, and Now does not. Spine opens with Now at the top. ⌥1 to ⌥4 fold a zone and bring its band to the top while Spine is in front; otherwise they bring the zone's tab forward, or open Spine at that zone. Shift+F10 or the menu key opens a band's menu. A Job on a workflow draws its steps as the zones, as Classic's `StoryColumn` does: the head names the workflow with its picker, and the bulb where the proposer picked it; a finished step has a tick and folds as Done does; the current step has the lamp, its acts and Skip; a step to come has Jump here and Skip; a skipped step is struck through with Undo alone; and the last step carries the landing. A plan that ends on gates draws a zone for each closed stretch with the gate that closed it, then Now with the plan's chunks. The Open group follows the last zone, and Hide Open group moves what it holds into the current step. The bare Done, Now and Open tabs draw the same zones. Still owed in Spine: a task chip's press, which opens Projects, a page native lacks. Workflow, from the + menu or ⌥W, draws a Job's lanes as Fleet works them out (`SessionRecord::lanes`), as Classic's `LanesBody` does: the workflow's lane on top, then a lane for each ticket, pull request, watch and Job it holds, each named in a head at the left; a rule is a chip on its edge with what it did and when, or what it waits for; −, + and Fit stand in the rail, the wheel and a drag move the canvas, arrows go to the nearest node, ] and [ go round the rules, Enter picks and Esc lets go. A Job with no lanes draws nothing there. Add to this Job above the lanes adds a step, a lifecycle or a rule to this Job's copy, with Fleet's refusal kept in the form; Changed on this Job under them lists what the copy changed, with Keep for every Job on the workflow and on each changed piece. A rule's chip opens its acts at the side, a section for each time it fired. A started Job's head opens that Job's own lanes, on a double press or Enter, with Back and Esc to return. A ticket's head offers Teach on a right press |
| Operator and Projects | The Operator's page (Your turn and its digest), its chat and toasts, and Projects |
| Settings, Kit, Reviews, Issues | Those four screens, Usage with Time, and Account. The pull request side panel's Review tab arrives with Reviews, with the Reviews page's keys; until then the panel has four tabs |
| Job tools | Terminal, the Request pane and the Browser tool |
| Switch-over | VoiceOver pass, the signed app on both Macs, the Electron fallback |

### 7. Switch day

**The owner picks the switch day once the native app does everything he uses daily.**
Both his Macs switch together, the work Mac included.
Electron is Armada Classic, with a grey icon, from the foundation on, so the Dock tells the two apps apart.
`armada://` links stay with Armada Classic until switch day, and move to the native app on that day.
Armada Classic stays installed for two weeks after it.
Then the Electron app is deleted from the repo, and the packages Pocket uses stay.

## How the build looks and behaves

| Area | Rule |
|---|---|
| Look | Today's Electron layout and features, on the same tokens. Only the spacing and font tweaks he names change |
| Each screen | Built to match its Electron screen, then walked natively, on its own, before it lands |
| Window | One window, as today. One Job shows at a time |
| Speed | A long thread opens, and a Job switch lands, in under 50 ms |
| Jobs list | Grouped by project, Needs you first in each group |
| Ended Jobs | Leave the list. The palette finds them |
| Thread | A run of tool calls folds into one line that opens on a press |
| Start | The app opens where he left off |
| Accessibility | Full VoiceOver |

**The speed figures tune the build and gate nothing.**
They are measured against Electron, where a thread's first paint takes 132 to 239 ms today.

**He usually has more than ten Jobs live.**
He switches from the list, from palette search and from Needs you.

## Where native departs from Armada Classic

The parity check takes each row here as chosen.

| Where | Armada Classic | Native |
|---|---|---|
| A pull request's Files tab | "N files" beside the tree, which draws every file | No count, under the no-aggregate-counts rule of 29 Sep |
| A Job's Issues tab | Two repositories' #7 show as one row, a fault | Each repository's #7 is a row of its own; a row from another repository names it before the number, as sample-web#7 |
| A Job's System tab: Columns | Hides Port, CPU, Memory and I/O | Hides Port alone: Fleet reports nothing else for each process |
| A Job's System tab: a refused read | Placeholder shapes forever | A toast in Fleet's words; the shapes stop and the tab draws nothing until a later read succeeds |
| A bay's panel: a confirm, a refusal or a receipt | Drawn where it falls, often out of sight in a short panel | Scrolled into view as it appears |
| A walk the Job never showed | A tab with an empty heading and a live Open | No tab is made for it |
| A draft whose row has gone from the thread | Its tab left standing as a blank frame | Its tab goes away and the next tab comes forward, as on a close; a press on it opens no tab |
| A subagent's tab once the Job no longer holds the subagent | Its tab left standing as a blank frame | Its tab goes away and the next tab comes forward, as on a close |
| What Now lists as owed him | Fleet's `waiting_for` with the asked question mixed in, so the question shows twice | Fleet's `waiting_for` alone, one row for each item; the asked question lights the lamp |
| A finished plan group in Done | Its tasks listed under every group at once by Whole plan | A press unfolds that group's tasks, as Open's groups do |
| A document in Done | Opens its page in the reading layer | Opens its card as a tab beside the thread; the page waits on Chrome |
| A row in a Job's Checks pane | A press opens nothing, a fault | Opens the Check's panel, with its runs in this Job |
| A judge note on a pull request with no line | Has nowhere to show | Stands at the top of Files, above the tree, in the line notes' look |
| A judge note or review thread on a pull request's line | Between the diff's lines, at that line | Under the head of its file in Files, until the native diff places a note at its line |
| A thumb pressed on a Deferred row | The row stays until Fleet's record says it is settled; a refusal says nothing | The row goes at once and the keys go to the message box; a refusal brings it back and is said in a toast |
| Stop or Delete on a watch | The watch stays as it was until Fleet's `watches.changed` says it stopped or went | It shows stopped, or goes, at once; a refusal puts it back, said in a toast in Fleet's words |
| The watch panel's Stop | Named Stop to VoiceOver | Named Stop watch and the watch's name, so it is told apart from the thread's Stop |
| A zone's band in Spine | A right press, Shift+F10 or the menu key opens Teach, where the window offers it | The same menu also holds Mod and Report |
| Deferred's toggle in Spine | Named Deferred to VoiceOver, as its band is | Named Deferred list, so it is told apart from Deferred's band |
| A band's height in Spine | The legend's height | The height of a control, as every other band in native |
| The Open group's menu in Spine | A right press alone | A right press, Shift+F10 or the menu key, as every band's |
| Triage comments and the rule's draft answered in place, in the Workflow tab, and triage in a pull request's panel (`owner.lanes.job.triage`, `shared-panel/pull-request.tsx:195`) | Classic draws mock-only rows | Native draws what Fleet serves |
| The Workflow tab's frame in a short pane | Keeps a floor of seven space-12s and scrolls the pane under it, so a key can pan to a node below the pane's edge | Fits the pane while nothing stands under it. With a form or the changes open, the body scrolls as Classic's does, but the canvas is never taller than the body less Add to this Job, and a key or press on it scrolls it into view, so the node the keys reach is in sight |
| Esc in a started Job's lanes | Goes Back from anywhere in the window | Goes Back while the keys are in the Workflow tab, so Esc in the thread or a menu keeps its own meaning |

## Notifying

**Needs you lives inside the Jobs list, and the Operator raises what needs him through toasts.**
A banner and sound fire only when a Job needs him or stops.
From the day the native Jobs screens land, only the native app notifies, sounds and badges the dock.

## Logic lives in Fleet

**Logic that Bridge works out in TypeScript today moves into Fleet as views.**
The native app and Pocket, which stays web, share those views.

## Using it while it is built

**He uses the native app for real work as each screen lands.**
Refresh preview rebuilds it with Fleet and Electron.

## What ships before switch day

Everything he uses daily:

- Jobs list, thread, and the full composer: drafts, attachments, tags, voice, fork, slash commands.
- Plan and draft approval, Needs you, the command palette and keys.
- Dispatch sheet, merge queue strip, usage rings, Fleet build switcher, context ring, write-clash marks.
- Documents, sketches and walks.
- The Job panes Plan, Changes, Files, Checks, Subagents, Log (the Check panel's Log tab), Ledger, Judge, the pull request panes, Issues, Jobs, System, Bays and Spine.
- A native diff viewer with comments.
- The Operator's page, Your turn beside its digest, with its chat on Ask and as a pane from a Job, and toasts.
- Projects, every Settings section, Kit, Reviews, Issues, Usage with Time, Account with phone pairing.
- Terminal, several per Job, the Request pane and the Browser tool.

## What waits for after switch day

- The mod kit.
- The Cockpit's map, activity, bays, cleanup grid and flight tiles, and the battlefield, as mods.
- The Job view's Field layer, with its plan bar, replay slider and lens card, which comes with the Cockpit map.
- Autopilot, the timeline HUD, Doctor, Lessons, Studios, Workflows, Logbook, Guides and the Mods page.
