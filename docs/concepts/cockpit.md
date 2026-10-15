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
| Activity | What Fleet has been doing, newest first. A press picks the event and the map shows what moved | |
| Stage | The timeline or the map at rest. When something is picked, that thing: a Job's canvas, a Session's thread, a bay, a pull request, a call, Autopilot's lane | |
| Merge queue | Main as one light, then a chip for each pull request. A press opens the pull request on the stage | `q` |
| Bays | The worktree slots and who holds each | `b` |
| Incoming calls | Every call waiting on a person | `c` |
| Flight control | Jobs only, Sessions only, Checks, Calls, Map and Timeline, as tiles | `f` |
| Autopilot | The switch. On, it gives the stage to the lane, with its sidings, Next up and Set aside | `a` |

`Esc` returns the stage to what it was. The state is held in `apps/desktop/src/renderer/src/cockpit/deck/store.ts`, so the title row, the body and the keys are one thing, and leaving the tab and coming back returns the same view.

## The map

The stage at rest is the map or the timeline, one switch (`m`, `t`, or tiles 5 and 6). The map draws each repository as directories with a drone over what it is changing, from the tool events Fleet writes when a burst closes. A write clash is an amber hatched slab on the directory both write in, from `list_write_clashes` ([Fleet](fleet.md), Write clashes). A replay plays back the last two hours of events Bridge holds; before a bay's first event in the window it reads as unknown, not free. `v` swaps a Job's mission and its canvas.

**Nothing is drawn that Fleet does not hold.** A drone whose path is not known rests by its bay pad, and a stale tool burst reads as no action.

## Calls

A call is something waiting on a person: a Job's gate, a question an agent asked, a Session that replied and went idle. It is raised on the deck as an Incoming call, and the deck is the only Cockpit, so there is no second place one is drawn. A press opens it on the stage, and it leaves Incoming when it is answered or seen.

While the window is on another tab a toast says a call has come and a press brings it forward on the stage. On the Cockpit tab no toast is drawn, and the Cockpit tab carries a pip while a call waits and the window is elsewhere. A call in front takes the bare digits and letters the `call` scope names, and the deck's handlers stand down for it.

## Moving about

| To | Do |
|---|---|
| Another tab | `⌘1` Cockpit, `⌘2` Jobs, `⌘3` Sessions, `⌘4` System, or `g` then `c`, `j`, `s` or `y` |
| A System page | The strip under the tabs: Kit, Mods, Settings, Guides, Workflows, Doctor, Studios, Retros. System opens the page last used. `⌘5` is Settings |
| A page with no tab or strip place | A link on the deck, or the palette, `⌘K`, by name: Worktree Slots, Merge line, Manifest, Checks |
| A panel | `l b c q f a` |
| A flight control | `1` to `6` |
| Walk the list | `j`, `k`, `Enter` in the list; `w` brings a waiting call back; `?` lists the keys |
| Fleet's panel | The status dot on the title row |

The keys are rows in `crates/core-model/domain/actions.toml`, rebound as [Key bindings](key-bindings.md) says. The tab bar's order and the places a layout may hide are in [Layout mods](layout-mods.md). No page is reachable only by a place the window does not draw.

The deck links to three of them: the Bays band to Worktree Slots, the merge queue's mark to Merge line, and a control beside the repository picker to the Manifest. Checks has no link and is reached by name.

## Not yet, shown empty

The deck draws what Fleet holds and leaves the rest empty. Additions to Fleet, F1 to F4, each additive and each with its own claim, switch a part of the deck from empty to data. They land after #2301, in any order.

| | Fleet gives | Until then the deck shows |
|---|---|---|
| F1 | Repository areas: file counts by directory, to depth 2 | Pads and the main flag in each territory, no plots |
| F2 | Changed paths for every held slot | A drone resting by its bay pad, the slab flat; never a guessed directory. A Job's footprint is read for the open Job only |
| F3 | Why a lease ended, and its branch, on `slot.released` | A bay's history with no end glyph and no branch on past rows; a bay with no events draws no History |
| F4, optional | A burst-open event, so what a drone is doing is live | What a drone is doing from closed bursts only |
