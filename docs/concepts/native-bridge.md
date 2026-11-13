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
| Job panes | Plan, Changes, Files, Checks, Subagents, Log (the Check panel's Log tab), Ledger, Judge, the pull request panes, Issues with the issue panel, Jobs, System, Bays with the bay panel, Spine, and the native diff. Issues lists issues alone, as in Armada Classic: a Job's reported bugs show in the thread's Issues block. A pull request opened from the issue panel has Back to the issue at its head, as Classic's `useBack` draws it. A pull request no Job holds opens in the panel as the forge reads it, with no Notify Job and no Evaluate. One closed without merging says Closed and offers no act; Armada Classic draws it as Open. Its branch is left out, because the forge's page does not name it. The ticket menu reads the Ticket lifecycle as the tab shows, and not again as it opens. A Job it started carries its pull request in the Ledger, Merged or Open, and its Checks, as Classic's `LinkedSession` does. Still owed in the Ledger: note thumbnails, which wait on Fleet serving a note's picture: Fleet names it by its path and serves no bytes for it. A pull request's judge notes stand in its panel's Files tab under the line they name, after his own notes there, as in Classic; one on a line the diff does not draw stands under its file's head. A doc's stand under its Ledger row until the doc reader lands. His own line notes start from a changed line's gutter, stay on this Mac, and go from the notes bar to the Job that holds the pull request or, by its caret, to another; a resolved one waits behind Show resolved. Review threads always show, each with its Resolved mark, where native had hidden resolved ones behind a toggle Classic lacks. A pull request in the merge queue says its place, as in "3rd in the merge queue", and a Merge line plate draws the queue as Classic's `MergeLinePlace` does. Classic falls back to `armada land`'s own line; native leaves that out, because the merge line view it reads does not carry that line. The owner's chip stands in the panel's meta row, read from the page, else from the merge line. A file in the Files list carries the marks of the tests that reach it, from Fleet's test map, and a press or → opens them; Armada Classic wires this only in its mock, and native reads the route Fleet serves. The Files list keeps Classic's keys: Up and Down move one focus between its rows, → opens a folder or a file's tests and ← shuts them, and Enter opens a file, or a test that is a file in the file panel. Still owed in the pull request panel: Fix, which needs a terminal. Still owed in System: CPU, memory and I/O for each process, which Fleet does not report, and a Job's logs, which Fleet does not serve, so a log row opens only on mock data. In Bays, a clash's link to another Job lights that Job's row in the Jobs list while the pointer is on it; the link takes no keys, so focus lights nothing, where Armada Classic lights it on focus too. A Release Fleet refuses because the holding Job is working offers Open Job beside the refusal. A stranded bay is hatched on its amber ground and a closed one shuttered, without the stranded bay's glow. Beside the thread, a draft, a page owed alone, a walk of a native screen and a subagent's thread open as tabs. Still owed there: a web page's walk, which waits on Chrome. Done lists what the Job made, By step and By kind, as Classic's `madeOf` does. The mods a Mod Job made stand in Done as Classic's `ModsMade` does, each with its kind, a switch and a press that opens its panel with Change it; pull requests stacked on one another draw beside their lane with the base and the "on #N" chip; and a workflow that lands in a report draws the report as its card. Still owed in Done: the first lines of the report's conclusion, which waits on Fleet's file read (Fleet serves native's thread fields), and Promote on a mod, which comes with the Mods page. Now says only what runs now: Ask for an update and Plan (⌥N), then the agent's row, Fleet at work, the route step under way and the running Checks, then the tasks in progress, the groups being worked with their subagents at work, the Jobs it started that still run, and a live watch. A Job with nothing running draws nothing there. A drone opens its subagent's thread as a tab. A Mod Job's route is Mod, Check and On, read from the mods Fleet serves and the Job's Checks; a workflow that lands in a spike routes through Findings, and one that lands in a document or report through Document and Read. Still owed in Now: the live page, which Fleet serves no frame for. Watching stands at Now's foot, in Now alone only while a watch is live, as Classic's `WatchingBlock` draws it: the live watches and one that ended not ours while Needs you asks about it, Show ended for the rest with Delete beside each, and a row opens the watch's panel with Readings, Parts, Command, Output for a broken run, Log, its link out and Stop while it is live. Open lists what is still to come: the plan's groups still to come, a drafted plan's included, shut until pressed, a group going to a Job not started yet as that Job's placeholder card, then what has not landed and no group still to finish holds, then Ahead, then what Fleet is to do for the Job. Classic's bare Open leaves out a drafted plan and Ahead, which its Now holds. Deferred lists each follow-up the agent kept and he has not confirmed, as Classic's flat `LaterBlock` does: a card in Deferred's hue with Remind and two thumbs in columns of their own, and a right press or Shift+F10 offering Deferred and Not deferred. In Done, Now, Open and Deferred, Up and Down step between the rows and Home and End go to the first and last, as Classic's `arrowsIn` does. Spine draws Brief, Done, Now, Open and Deferred down one line as Classic's `Spine` does: a dot and a lamp band for each zone, the band held at the top while its zone scrolls under it, a press, Enter or Space folding the zone, and the folds kept for each Job, with Done shut at first. Brief shows only when the Job has a brief, with How it started and the Tasks the Job holds; Deferred shows only when the Job kept a follow-up, at the foot, shut until pressed. Open carries Ahead there, and Now does not. Spine opens with Now at the top. ⌥1 to ⌥4 fold a zone and bring its band to the top while Spine is in front; otherwise they bring the zone's tab forward, or open Spine at that zone. Shift+F10 or the menu key opens a band's menu. A Job on a workflow draws its steps as the zones, as Classic's `StoryColumn` does: the head names the workflow with its picker, and the bulb where the proposer picked it; a finished step has a tick and folds as Done does; the current step has the lamp, its acts and Skip; a step to come has Jump here and Skip; a skipped step is struck through with Undo alone; and the last step carries the landing. A plan that ends on gates draws a zone for each closed stretch with the gate that closed it, then Now with the plan's chunks. The Open group follows the last zone, and Hide Open group moves what it holds into the current step. The bare Done, Now and Open tabs draw the same zones. Still owed in Spine: a task chip's press, which opens Projects, a page native lacks. Workflow, from the + menu or ⌥W, draws a Job's lanes as Fleet works them out (`SessionRecord::lanes`), as Classic's `LanesBody` does: the workflow's lane on top, then a lane for each ticket, pull request, watch and Job it holds, each named in a head at the left; a rule is a chip on its edge with what it did and when, or what it waits for; −, + and Fit stand in the rail, the wheel and a drag move the canvas, arrows go to the nearest node, ] and [ go round the rules, Enter picks and Esc lets go. A Job with no lanes draws nothing there. Add to this Job above the lanes adds a step, a lifecycle or a rule to this Job's copy, with Fleet's refusal kept in the form; Changed on this Job under them lists what the copy changed, with Keep for every Job on the workflow and on each changed piece. A rule's chip opens its acts at the side, a section for each time it fired. A started Job's head opens that Job's own lanes, on a double press or Enter, with Back and Esc to return. A ticket's head offers Teach on a right press |
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
| A setup screen (owner, 12 Nov) | Opened cold, with no walk: each state shows the one press that moves it on and the reason when there is one. Nothing tells him to go and do something outside Bridge; a state with nothing to press says what stands, in plain facts. If he needs a walk through it, the screen has failed |

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
| A subagent in the thread and in the running card at its foot (owner, 12 Nov) | `SubagentRow`: a bordered box with a state mark, the brief, the time and a chevron; the card lists each with a dot | The call row's take A, as every tool call: the Agent kind's rail and moving icon, the brief, a counting time and its latest call on a second line, cut short; done, one grey line with the time it took, and a cut-off one ends in the stopped state's mark. The card keeps its frame, fold and Stop, and lists only the subagents Fleet says run now |
| What a Job waits on that is not him (owner, 12 Nov) | `WaitingBand`: a teal band over the box whose panel lists each wait with its own × | A card at the thread's end, held in sight at its foot as the subagents card is: a row for each wait with its kind's glyph in its state's hue. It has no close and leaves when nothing holds the Job open. A wait's stop is behind a right press on its row. Needs you stays the band for what waits on him |
| Now alone in its pane | What is owed him, the walks shown to him, what the Job waits on, a failed route step or Check, the open tasks, a plan he has not approved, Ahead, and a stopped Job's last tasks | Only what runs now, with more room between rows; the rest stays in Needs you, Open and Done, and a Job with nothing running draws nothing (his ruling of 12 Nov). Its band holds Ask for an update and Plan with no lamp before them, where Classic's bare band leads with a lone dot. Spine's Now keeps Classic's beats |
| A Job another started, in the Jobs list (owner, 12 Nov) | Steps in under its starter with a short elbow | Pocket's tree: each level steps in by space-6, a starter's line runs down the gutter to the last Job it started and turns into each at its title, and a line further up runs on past rows that are not its own |
| The Jobs list's scopes (owner, 12 Nov) | Active, Paused, Quiet, Ended and All | Needs you first: the Jobs whose lamp says they need him, each under its starters, drawn muted for their place and passed over by a press or a key. Nothing needing him draws an empty list |
| What Spine's Now lists as owed him | Fleet's `waiting_for` with the asked question mixed in, so the question shows twice | Fleet's `waiting_for` alone, one row for each item; the asked question lights the lamp |
| A finished plan group in Done | Its tasks listed under every group at once by Whole plan | A press unfolds that group's tasks, as Open's groups do |
| A document in Done | Opens its page in the reading layer | Opens its card as a tab beside the thread; the page waits on Chrome |
| A row in a Job's Checks pane | A press opens nothing, a fault | Opens the Check's panel, with its runs in this Job |
| A judge note on a pull request with no line | Has nowhere to show | Stands at the top of Files, above the tree, in the line notes' look |
| A description a Job proposed for its pull request | Waits as a draft in the Job's thread; the pull request panel shows the body the forge has | Stands in the panel in the description's place, with the lines it adds marked and those it takes out struck through, Current for the body it replaces, and Approve, Edit and Note. The pull request's row says a description waits |
| A pull request panel's plates (status, reviewers, failing checks) | Off the panel's leading edge, over the thread, when the window has room | Inside the panel above its tabs at every width, so nothing of the thread is covered |
| What Settings › Layout switches off | Every Cockpit panel and place it lists, with each layout mod that is on laid under his own choice | His own choice, on what native draws: a Job's Done and Open tabs (out of + too) and the merge queue. Native has no Cockpit panels or rail of places, so those switches have nothing to hide, and a layout mod is listed in Settings but not yet laid under it |
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
| A draft card's head in a narrow thread | The kind, title and time share one row, and the title's words wrap inside it | The time and its page link wrap under the title, so they stay inside the card |
| A sketch or document card's answers in a narrow thread | Approve, Send back and Withdraw stay on one row, small enough to fit | The same buttons at native's control height wrap to a second row, so Withdraw stays inside the card |
| A judge note's answers in a narrow pane | Send to Job and Dismiss stay on one row at Classic's small button size | The same buttons at native's control height wrap to a second row, so Dismiss stays inside the pane |
| A System section's note in a narrow pane | The note beside a heading such as Processes wraps a word to a line, and Columns stays at the row's end | The note is one line, cut short, and Columns stays at the row's end |
| A pinned sketch's head in a narrow thread | The band shrinks and its title wraps beside Revision, open and Fold | The title is one line, cut short, so Fold stays inside the card |
| The message box's menus in a box under 400px wide | The menus lose their floor and their words truncate to nothing beside the caret | The words are not drawn, and each menu keeps its glyph and caret, so Send stays on the row at native's control sizes |
| Close pane | The pane leaves the saved arrangement, and the Layout menu brings it back | The pane stays closed on that Job until Bridge restarts; the Layout menu, splits and the saved arrangement are held |
| Zoom from a tab in the right column | Each tab's own menu has Zoom or Restore | The column's tabs have no menu yet: ⌥⌘Enter, a double press on the strip and Restore zoom them. The thread tab's menu has Zoom |
| A terminal's tab glyph | `square-terminal`, which the icon registry reserves for a Session | `file-terminal`, the glyph the registry gives the Job's shell, as the dispatch line's Terminal kind and ⌘T's palette row draw it |
| A terminal tab's menu | Send to agent, Copy terminal ID, Rename, Move to window, Zoom, Split right, Split down and Close | Send to agent, Copy terminal ID, Rename and Close; Move to window, Zoom and the splits wait for native panes to have them |
| A note just saved (owner, 12 Nov) | The card closes | The card stays open with Send to job in hand: Enter opens the live Jobs beside it, ⌥1–9 or Enter sends, Esc closes it |
| Send to job on a note, the notes bar and the selection in All notes (owner, 12 Nov) | Behind Start job's caret, in a split button | Its own button, first in the row, with Start job beside it; the picker opens beside it and finds a Job as it is typed |
| The page a sent note names | The page's web address, which Fleet reads as a checkout of the page | `armada://native/<screen>`, since a native screen has no web address |
| A hold on the Operator page | Electron is frozen and draws no paths, Job or until | Fleet's words, then a line under them with each held path in mono and "until it pushes" or "until it ends"; a press opens the Job that alone writes there. A hold Fleet sent before it named them keeps its words alone |
| A picture a call read, and a message a call sent | Electron is frozen and draws a `Read` of a picture and a `SendMessage` as their bare lines | A `Read` of a picture draws it under its line, read from the Job's own files, and a press opens it full size. A `SendMessage` reads "Message to" and the Job's title, a link where Fleet placed it, else the name as written; its summary is the second line, and a press on the row shows the words it sent |
| A switch in Settings (owner, 12 Nov) | The default `Switch`, two space-units taller than a badge, near a row's height, so the rows read as one column | Classic's small switch (`data-size="sm"`): one badge tall, with each row's height kept, so a row has air above and below its switch |
| A workflow's lanes in Settings › Workflows | The lanes stand in place of the step cards, and an inspector shows a picked step | The lanes stand under the step cards, until the inspector is ported |
| Add lifecycle over a workflow's lanes in Settings | Drawn where the lanes can be edited | Not drawn: native's lanes are read-only for now |
| The clash shown in the Settings walk | api's own copy of feature, where Platform and Mobile differ | armada's own copy of feature, with the same words, since the mock serves no api repository |

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
