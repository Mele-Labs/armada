# Bridge

**What it is:** The Electron command-center shell, and the only engineer-facing surface in Armada — a client of Fleet rather than the daemon itself, so closing it stops nothing.

---

**Kind:** Surface.

This document exists so "Bridge" has one stable, correct link target, not to duplicate journey content.
Bridge is a shell rather than a domain concept: it owns the chrome, and the work happens in the journeys that mount inside it.

## What it is

The Electron frontend — the only engineer-facing surface in Armada.
Single-user, no auth layer (personal dashboard, not a shared team view).
Chosen over a TUI for richer visualization: monitoring many Jobs, diffs, real-time UI (see `../contracts/system-architecture.md`, section 4).

## Relationship to Fleet

**Bridge is a client of the Fleet daemon, not the daemon itself.** It connects to the Armada API over **one axum listener** — WebSocket for the real-time event stream, HTTP for request-response commands, same port, no second port and no gRPC.
`../contracts/system-architecture.md` owns that decision and the measured `tonic` cost behind it, recorded against the decision on which web framework the api crate uses.

Closing Bridge does not stop Fleet; reopening it reconnects rather than respawning.
**What Bridge shows while it cannot reach Fleet, and what it trusts on reconnect, is not yet designed** (see Open questions).

**Bridge authors no vocabulary of its own.** Statuses, their reasons and verdicts all reach the renderer as one generated TypeScript module, emitted by the same codegen that produces the `ipc` types — names from `core-model`'s enums joined to a checked-in file carrying each variant's verb, icon and status token.
Bridge imports it, and never holds a status list for a filter, a reason list for a badge, or a copy of the enum→verb map.

The build fails where a variant lacks a verb, an icon or a token. `../contracts/system-architecture.md`, section 6 owns the mechanism.

**A person's own way of looking is kept by Fleet, in settings.json** — theme, layout, key bindings, and the editor and terminal a file or a worktree opens in.
Bridge's Settings page is drawn from `get_settings`, so a key Fleet adds appears there without a change to Bridge, and a hand edit reaches an open Bridge on `settings.changed`. [Machine](machine.md), *settings.json*.

**Bridge also calls `launchctl` directly**, outside the protocol: it bootstraps Fleet's launchd job at login and restarts it via `kickstart -k`.
Why that is Bridge's job rather than an API operation, and what "Restart Fleet" actually means under launchd, are on [Fleet](fleet.md) — Daemon Lifecycle.

## Where Bridge's behavior is actually documented

Bridge has no behaviour section of its own. Its behaviour is specified across the journeys below, each covering a distinct trigger rather than a slice of shared UI machinery.

| Journey | Trigger |
| --- | --- |
| Check System Health | "Is everything okay right now?" — Doctor's module grid |
| Run and edit a Manifest | Run this project's checks on demand, and edit the file |

The module grid is [Doctor](doctor.md)'s. **The count is not a contract** — a module earns a row where Armada depends on it and it can be up or down, and Doctor carries the list.

Run and edit a Manifest reads a project's Checks and Commands, runs any one of them on demand, and edits the file.

## Top-level shell

Bridge's shell is a **title row** across the top and, beneath it, a **full-width window** where the journeys mount.
There is no left column and no rail: the places a person moves between are the title row's tabs and, under System, a strip of pages.
The Cockpit is the window's first tab, and its own left column holds the repository picker and the Work list of Jobs.
See [Cockpit](cockpit.md).
Finer layout treatment within each journey remains UI/UX design phase work.

**The title row** carries a wordmark, the tabs, the one bar that searches and dispatches (an icon and its key while the tabs are drawn), the plan's limits, once Helm's dock is closed its own reopen button, Fleet's status dot, and one menu holding the account, Sleep, Settings and Doctor.
The account entry reads Sign in with GitHub while signed out and opens the [account](account.md) once signed in; Settings carries its key.
The menu button carries a moon while Sleep is on and an amber dot while signed out.
Below the row's narrow width the other controls are listed in the menu, and the tabs stay.

| Tab | Opens | Key |
|---|---|---|
| Cockpit | The flight deck | `⌘1` |
| Jobs | Every Job | `⌘2` |
| Studios | One repository's Studios, and a Studio opened from the list | `⌘3` |
| Logbook | What agents know, and on its Retros tab the items every Job's retro raised | `⌘4` |
| Reviews | The pull requests waiting on him | `⌘5` |
| Issues | A repository's tracker | `⌘6` |
| Projects | His own projects and their tasks, across repositories | `⌘7` |
| System | The System page last used, Kit where none has been | `⌘8` |

The keys above are the tabs' places when every tab is drawn.
A tab that is not drawn takes no key, and the keys behind it move up.

**System is a strip of pages under the tab bar**, drawn while System is the place: Kit, Mods, Settings, Guides, Workflows.
Settings answers `⌘,` and the title menu's entry.
**Doctor is a row of the title menu** and not a page of the strip (the owner, 15 Oct 2026: *"Doctor should live in the menu in the top right"*); `⌘K` reaches it by name.
**Studios is a tab** (the owner, 15 Oct 2026: *"Studios and retros should live in the main bar at the top of the app"*), and every page of an open Studio counts as the Studios tab being current.
**Retros is not a place since 22 Oct 2026**: its items are a tab of the Logbook, and every way to Retros opens the Logbook on it.
A page with no tab and no strip place stays reachable: Worktree Slots, Merge line, Manifest and Checks open from the palette (`⌘K`) by name.
The rule is that no page is reachable only by a place the window does not draw.
`packages/shell/src/surfaces.ts` is the one roster the tabs, the strip and the palette read.

**The repository picker is the top of the Cockpit's left column.** At the title row's narrow width the picker's entries are listed in the title menu, with a control that opens the picked repository's [Manifest](manifest.md).
**Off the Cockpit the title row names the picked repository**, or "All repositories", as text at its start; it cannot be changed there.

**The picker names a repository, never a Manifest's id.** A set-up repository read as its `ManifestId` — a ULID — until the owner found one in the trigger on 28 Sep 2026. [Manifest](manifest.md), Deletion, already bound it: a surface renders the name, and nothing renders a bare id.
The name is what Fleet read the Manifest under, widening to `parent/folder` and then the whole root only where two repositories would otherwise read alike.

**The window's places are declared in `surfaces.ts`, not derived from what a screen draws.** Helm is not a place: it is a dock beside the content on every surface, toggled by `⌘J`, and above the layout breakpoint a closed dock draws nothing at all, so the title row's own Helm button is the one way back.
See [Helm](helm.md).

**Studios is one repository's Studios, and one open on its whiteboard.** The list names each Studio and when it was last touched; a Studio opened from it is read-only until Continue, and accepting or rejecting a proposed relation, and deleting a node, happen there and nowhere else.
See [Studio](studio.md).

**Settings is Fleet's limits and this machine's own settings, on one screen** — Helm's action authority first.
It replaced a sheet reached from the status bar (#1088 removed the bar), and it is where a limit changed still takes the same way it always has.
It is a System page, `⌘,`, and the title menu's entry.

**A digit is a tab's place in the bar, and nothing else takes one.** `⌘1` to `⌘8` are the tabs in the order the bar draws them, each drawn tab taking the next, Settings is `⌘,`, and every other page is reached from the strip, the palette or a link.
The rail's `⌘1`–`⌘9` and their history are in `../contracts/design-system.md`, Two tiers.

**Workflows is a System page and takes no digit.** It lists the workflows Fleet holds for the picked repository in three groups, and a workflow opens as its steps.
Fleet is read for the list, for each file's text and for the save; a Kit file and a repository file are written through `save_workflow`, so Fleet's loader checks what is saved and its sentence is what a refused save shows.
The page's glyph is `workflow`, and its strip tab carries a mark while Fleet has left a file out, with no count.
See [Workflow](workflow.md).

**Checks is reached by name from the palette and takes no digit.** It lists every Check this repository's Manifest has had requested or run, one row each with a mark for where it stands and who asked for it: a Job, the merge line, or a person.
A row opens a panel with the Check's facts over its log, followed live while the Check runs.
Who asked is a link where a route to it exists.
The glyph is `shield-check`, the Check family's.
See [Manifest](manifest.md).

What the window places is what is built, which is not yet the whole roster. A surface with nothing behind it would be a promise Armada does not keep, so it is not listed and takes no digit.

**The Fleet panel is a popover on the title row's status dot.** The dot is drawn at every width, so a press reaches the panel from every tab, and the panel reports Fleet and Doctor health when it is open.
The Manifest surface's Doctor strip can therefore disappear when every module passes, without its absence being ambiguous.

**The panel names Fleet's state rather than only reporting health, and it draws even when Fleet is down.** Running, not running and unreachable each get a string — see `../contracts/design-system.md`, Component → token mapping, Fleet panel.
Not running and unreachable are separate because a missing runtime file and a live pid that does not answer call for different things.

Whether the panel reads the same during onboarding, before Fleet is reachable, is tracked in `../contracts/design-system.md` (`status-bar-onboarding` — the slug predates the panel and is unchanged, since a citation resolves by name).

## Still open

Almost nothing is scoped to Bridge as a config target: settings a person adjusts *in* Bridge are tagged to the concept each one affects — landing Manifest to Manifest — rather than to Bridge as a catch-all.
**Notification routing is the exception**: no dependency path carries config to Bridge, so the Electron side reads its own copy or a hardcoded default.
See `../contracts/configuration.md` for the tiering rule.

**A person's Bridge preferences are a different path from that one, kept by Fleet rather than resolved from `armada.yml`.** `#927`.
Fleet keeps a preferences table the way it keeps a person's admission limits — one row per name, an absent row reading as the shipped default — and serves it over `get_preferences`/`save_preferences`.
Bridge's main process loads it when Fleet connects, saves on change, and publishes it in `BridgeState`; while Fleet is unreachable Bridge draws the shipped default and queues no save.

**Whether Bridge notifies is one switch in settings.json, `features.notifications`**, read off `BridgeState.settings` like every other key.
It covers every job that needs the owner; there is no per-event routing.

## Open questions

- **[bridge-reconnect-trust]** What does Bridge show while it cannot reach Fleet, and what does it trust on reconnect?
  Closing Bridge does not stop Fleet, and reopening it reconnects rather than respawning, but the reconnect behavior itself is not yet designed.
