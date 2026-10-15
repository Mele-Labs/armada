# Cockpit

**What it is:** Bridge's first tab and the screen it opens on — the flight deck: every Job and Session running, every call waiting on a person, the worktree slots and the merge queue, on one screen.

---

**Kind:** Surface.

You open Bridge. The list on the left holds a Session that is waiting and a Job that is running. Incoming calls on the right holds the Session's question. Press `c`, pick the call and answer it, and it leaves Incoming calls.

The Cockpit is the one body behind the Cockpit tab. The grid it replaced, the call deck and tray, the Bays metro view and the left-column rail are gone (#2301). [Bridge](bridge.md) owns the shell around it.

## Panels

```
+-----------------+----------------------------------+------------------+
| repository      |                                  | Bays             |
| Jobs and        |  Stage                           |                  |
| Sessions        |                                  +------------------+
|                 +----------------------------------+ Incoming calls   |
+-----------------+ Merge queue                      |                  |
| Activity        +------------------+---------------+                  |
|                 | Flight control   | Autopilot     |                  |
+-----------------+------------------+---------------+------------------+
```

| Panel | Holds | Key |
|---|---|---|
| Repository picker | Top of the left column. Narrows the list, the feed, the bays and the queue to one repository, or All repositories | |
| Jobs and Sessions | One row for each: its kind, title, what it says and its state's mark | `l` |
| Activity | What Fleet has been doing, newest first. A press picks the event and the map shows what moved. While a Job or Session is open on the stage it tells of that one alone, a Session's subagents included, and tells of all again when it closes | |
| Stage | The timeline or the map at rest. When something is picked, that thing: a Job's canvas, a Session as the panel the Sessions page draws (its head with the context figure and the rename, its thread and its box), a bay, a pull request, a call, Autopilot's lane | |
| Merge queue | Main as one light, then a chip for each pull request. A press opens the pull request on the stage | `q` |
| Bays | The worktree slots and who holds each. While a Session is open on the stage, that Session's ledger is drawn here instead, and the Bays return when it is closed | `b` |
| Incoming calls | Every call waiting on a person | `c` |
| Flight control | Jobs only, Sessions only, Checks, Calls, Map and Timeline, as tiles. While a Session is open on the stage, that Session's own (*The Session's flight controls*) | `f` |
| Autopilot | The switch. On, it gives the stage to the lane, with its sidings, Next up and Set aside | `a` |

`Esc` returns the stage to what it was. The state is held in `apps/desktop/src/renderer/src/cockpit/deck/store.ts`, so the title row, the body and the keys are one thing, and leaving the tab and coming back returns the same view.

## The Session's flight controls

While a Session is open on the stage the tiles are that Session's, in the same block under the same keys, `1` to `5`. When it closes the six return.

| Key | Tile | Does | Through |
|---|---|---|---|
| `1` | Ask for update | Sends the Session `askForUpdate`'s text | `askForUpdate(draft, sessionId)` in `deck/acts.ts`, over `draft.send`, the route the Sessions page composer sends by |
| `2` | Auto land | Turns the instruction on or off. On sends "Auto land is on. When your work is done, land it: open the pull request, merge main in, turn on auto-merge, watch ci, and fix failures and conflicts until it is in main." Off sends "Auto land is off. Do not land on your own." | `draft.send`; the state is kept for each Session in the browser's storage (`deck/auto-land.ts`), so the tile reads on after a restart. Fleet has no operation for it |
| `3` | Compact | Runs the Session's `/compact` | A hosted Session is sent `/compact` as a message, as picking it from `/` does; a terminal Session is typed it through `draft.command`. Drawn off unless the agent or the terminal listed `compact` |
| `4` | Report | Drawn off | `file_report` is keyed by a Job and renders that Job's record, so a Session has no route to it |
| `5` | Map | Closes the Session and shows the map | The stage's view switch, as tile 5 is |

A tile is off with its reason as the tooltip when the Session ended or its terminal is not listening. A press that sends is lit until Fleet answers, and a refusal is the toast the composer shows.

## The map

The stage at rest is the map or the timeline, one switch (`m`, `t`, or tiles 5 and 6). The map draws each repository as its directories, with a drone over what it is changing. The directories are the 40 largest of the base branch's tree, counted by git, and a drone is placed from the tool events Fleet writes when a burst closes, and where it has none, from what its slot holds against the base ([Fleet](fleet.md), *The territory*). A write clash is an amber hatched slab on the directory both write in, from `list_write_clashes` ([Fleet](fleet.md), Write clashes). Both parties also carry the swords mark wherever they are drawn: their list row, their pane head and their drone, which needs no directory. The mark names the other party and a press opens it on the stage; a clash against main is a picture with nothing to open. A replay plays back the last two hours of events Bridge holds; before a bay's first event in the window it reads as unknown, not free. `v` swaps a Job's mission and its canvas.

**Nothing is drawn that Fleet does not hold.** A drone whose path is not known rests by its bay pad, and a stale tool burst reads as no action.

## Calls

A call is something waiting on a person: a Job's gate, a question an agent asked, a Session that finished its turn and is done. A reply asks nothing of him and raises none. It is raised on the deck as an Incoming call, and the deck is the only Cockpit, so there is no second place one is drawn. A press opens it on the stage, and it leaves Incoming when it is answered or seen.

While the window is on another tab a toast says a call has come and a press brings it forward on the stage. On the Cockpit tab no toast is drawn, and the Cockpit tab carries a pip while a call waits and the window is elsewhere. A call in front takes the bare digits and letters the `call` scope names, and the deck's handlers stand down for it.

## Moving about

| To | Do |
|---|---|
| Another tab | `⌘1` Cockpit, `⌘2` Jobs, `⌘3` Sessions, `⌘4` Studios, `⌘5` Retros, `⌘6` System, or `g` then `c`, `j`, `s` or `y` for the first three and System |
| A System page | The strip under the tabs: Kit, Mods, Settings, Guides, Workflows. System opens the page last used. `⌘,` is Settings |
| Doctor | The title bar's menu, top right, or `⌘K` by name |
| A page with no tab or strip place | A link on the deck, or the palette, `⌘K`, by name: Worktree Slots, Merge line, Manifest, Checks |
| A panel | `l b c q f a` |
| A flight control | `1` to `6`, or `1` to `5` while a Session is open |
| Walk the list | `j`, `k` and `Enter` or `o` to open, in a list: Jobs and Sessions, Activity, Bays, Incoming calls and the merge queue; `w` brings a waiting call back; `?` lists the keys |
| Fleet's panel | The status dot on the title row |

The keys are rows in `crates/core-model/domain/actions.toml`, rebound as [Key bindings](key-bindings.md) says. The tab bar's order and the places a layout may hide are in [Layout mods](layout-mods.md). No page is reachable only by a place the window does not draw.

The deck links to three of them: the Bays band to Worktree Slots, the merge queue's mark to Merge line, and a control beside the repository picker to the Manifest. Checks has no link and is reached by name.

## What Fleet serves the map, and what it does not

The deck draws what Fleet holds and leaves the rest empty. F1 and F2 of #2301 are served; F3 and F4 are not.

| | Fleet gives | Where it gives none the deck shows |
|---|---|---|
| F1 | `list_repository_areas`: the directories of each repository's base to two names deep, the files of each, the 40 largest | Pads and the main flag in each territory, no plots |
| F2 | `list_slot_touches`: for each Job or Session holding a slot, the paths it changed since the slot met the base, uncommitted and untracked included | A drone resting by its bay pad, the slab flat; never a guessed directory. A Job's footprint is read for the open Job only |
| F3, not yet | Why a lease ended, and its branch, on `slot.released` | A bay's history with no end glyph and no branch on past rows; a bay with no events draws no History |
| F4, not yet, optional | A burst-open event, so what a drone is doing is live | What a drone is doing from closed bursts only |

**F2 is the last resort for a drone.** A tool burst places a drone first, then what its slot holds against the base, and the open Job's footprint only where neither does. A holder a repository's slots do not name, and a Session in a terminal Fleet does not host, has no such read and rests at its pad. A repository with more than 40 directories is drawn from its 40 largest, and a holder with more than 200 changed paths from the first 200 in the alphabet.

Both reads are polled, since Fleet pushes no event for either: when Fleet connects, and every 35 seconds while the board changes or a holder has changes. A Fleet before them answers 404 and the deck draws none.
