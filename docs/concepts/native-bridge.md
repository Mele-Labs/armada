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
| Job panes | Plan, Changes, Files, Checks, Subagents, Log (the Check panel's Log tab), Ledger, Judge, the pull request panes and the native diff. Still owed in the Ledger: the Merged, Open and Checks chips on pull request and linked Job rows, and note thumbnails. A pull request's judge notes stand in its panel's Files tab, under the head of the file they name, since the native diff has no line numbers yet; a doc's stand under its Ledger row until the doc reader lands. Still owed in that Files tab: his own line notes, the notes bar and Send to job, which wait on line numbers in the native diff |
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
| A judge note on a pull request with no line | Has nowhere to show | Stands at the top of Files, above the tree, in the line notes' look |
| A judge note or review thread on a pull request's line | Between the diff's lines, at that line | Under the head of its file in Files, until the native diff draws line numbers |

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
- The Job panes Plan, Changes, Files, Checks, Subagents, Log (the Check panel's Log tab), Ledger, Judge and the pull request panes.
- A native diff viewer with comments.
- The Operator's page, Your turn beside its digest, with its chat on Ask and as a pane from a Job, and toasts.
- Projects, every Settings section, Kit, Reviews, Issues, Usage with Time, Account with phone pairing.
- Terminal, several per Job, the Request pane and the Browser tool.

## What waits for after switch day

- The mod kit.
- The Cockpit's map, activity, bays, cleanup grid and flight tiles, and the battlefield, as mods.
- The Job view's Field layer, with its plan bar, replay slider and lens card, which comes with the Cockpit map.
- Autopilot, the timeline HUD, Doctor, Lessons, Studios, Workflows, Logbook, Guides and the Mods page.
- The seldom-opened Job panes: Spine, Workflow, Now, Done, Open and Deferred.
