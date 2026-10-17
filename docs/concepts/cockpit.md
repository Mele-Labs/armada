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
| Activity        +-----------+----------+-----------+                  |
|                 | Flight    | Dispatch | Autopilot |                  |
+-----------------+-----------+----------+-----------+------------------+
```

| Panel | Holds | Key |
|---|---|---|
| Repository picker | Top of the left column. Narrows the list, the feed, the bays and the queue to one repository, or All repositories | |
| Work | One tile for each Job and Session: its kind, title, what it says, one glyph for the phase of its build and its state's mark. Filter in its band holds a search and the scopes Active, Quiet, Ended and All, and the band names what narrows it; `/` opens it on the search. An ended Session is a row here and never a drone on the Field. **A Session started from another stands directly under its parent** with a connector, led by `link` (tooltip `Linked Session`, and `Linked Session, handed from <group>` where it was handed a plan group); a press on it opens it from the parent, and Esc returns to the parent's thread as it was. `docs/concepts/session.md`, *Linked Sessions* | `l` |
| Activity | What Fleet has done in the last hour, as a beeswarm with no words: a dot for each event at its time, the newest at the right, in the colour of its tone and smaller and fainter for the coalesced reads, edits and commands. Hovering a dot opens a card of the events near it, the `Hour` card, which keeps its own look by the owner's choice (16 Oct 2026) and is not an inspect card, and a press picks the event and the map shows what moved. While a Job or Session is open on the stage it tells of that one alone, a Session's subagents included, and tells of all again when it closes | |
| Stage | The timeline or the map at rest. When something is picked, that thing: a Job's canvas, a Session as the panel the Sessions page draws (its head with the context figure and the rename, its thread and its box), a bay, a pull request, a call, Autopilot's lane | |
| Merge queue | Main as one light, then each repository's pull requests on one line that scrolls across and never wraps: those in the forge's merge queue first, in its order, framed together with each one's place, then the rest past a rule. A press on a chip opens the pull request on the stage. A pull request the hub marks `merge_by_hand`, the one a Manifest form's save opened where the forge would not turn auto-merge on, carries a person's glyph after its number (`Needs merging by hand`), its pane has Merge and Review, and it is also a call in Incoming calls; the Manifest form shows the same line under Save. While a main is red, hovering or focusing the light opens an inspect card holding the Merge line's red panel for each red repository: what failed, the merge that broke it, who is fixing it or holds the claim, and, where nobody has it, the acts to dispatch a Job or a Session on it, the same acts and routes the Merge line sends. A press on the light pins the card; Esc or a press outside closes it. Main's light also carries a small amber mark while a served repository's checkout is behind main and Fleet would not catch it up; its dot keeps meaning only main's CI. The card then opens on a green main too, and under main's state a section, Your checkout, names what holds it back and, where one press is safe, the command that brings it level, to run or copy. Run asks Fleet to do it, which reads the hold again and refuses if it is no longer that one; Copy gives the command for a terminal, and both show without the pointer over the block. Beside them a Pull request button sends the uncommitted files up as a pull request with auto-merge on, committed by Fleet without touching the checkout; the card then shows the pull request's chip with an auto-merge mark in place of the command, drawn from the hold Fleet publishes, and the files stay listed. Once the pull request is on main Fleet catches the checkout up by itself and the mark goes. Each uncommitted path is a button: a press opens that file's change under it, read by Fleet, and a second press folds it. A change past Fleet's bound is drawn as far as Fleet sent it, with one line after it, `First N lines`. A long list of uncommitted files scrolls inside the card, the files main also changed first. A green main with a level checkout has no card | `q` |
| Bays | The worktree slots and who holds each. While a Session is open on the stage, that Session's ledger is drawn here instead, with its ring above it where it has one, and the Bays return when it is closed | `b` |
| Incoming calls | Every call waiting on a person | `c` |
| Flight control | Jobs only, Sessions only, Checks, Calls, Map and Timeline, as tiles. While a Session is open on the stage, that Session's own (*The Session's flight controls*) | `f` |
| Dispatch | One tile, between Flight control and Autopilot, carrying `n`. It opens the Dispatch sheet, the one `n` and New Job open | `d` |
| Autopilot | The switch. On, it gives the stage to the lane, with its sidings, Next up and Set aside | `a` |

**Hovering or focusing a row of Jobs and Sessions, a bay, an incoming call, a merge queue chip or a drone on the map opens the inspect card** (`docs/contracts/design-system.md`, Inspect card): the kind, the name, the state, the facts the row already holds and the key that opens it. A press on each of these is already an act, so none of these cards pins.

The two side columns resize. A handle sits in the gap on each inner edge, drawn only while hovered, focused or dragged, and it is `DockHandle` (`packages/components/src/dock-handle.tsx`), the one Helm's dock and every sheet draw. A drag or `←` `→` moves it, and a double click or `Home` puts the column back to its resting width. The left column stops at 4.5 `--space-12` and the right at 4.75, and neither goes wider than leaves the stage 10 (480px), measured against the window and taken again when it resizes. The widths are kept in this browser as dragged (`armada.bridge.deck-columns`, `deck/columns.ts`), so a column dragged wide in a big window returns to that width when the window does. The deck has no narrow layout, so there is none to leave the handles out of.

`Esc` returns the stage to what it was, and with nothing open it lets the cursor go, so no row is picked and the map holds no drone's card. A press on the list or the map away from a row or a drone lets go the same way. The state is held in `apps/desktop/src/renderer/src/cockpit/deck/store.ts`, so the title row, the body and the keys are one thing, and leaving the tab and coming back returns the same view.

## The Session's flight controls

While a Session is open on the stage the tiles are that Session's, in the same block under the same keys, `1` to `6`. When it closes the six return.

| Key | Tile | Does | Through |
|---|---|---|---|
| `1` | Ask for update | Sends the Session `askForUpdate`'s text | `askForUpdate(draft, sessionId)` in `deck/acts.ts`, over `draft.send`, the route the Sessions page composer sends by |
| `2` | Auto land | Turns the instruction on or off. On sends "Auto land is on. When your work is done, land it: open the pull request, merge main in, turn on auto-merge, watch ci, and fix failures and conflicts until it is in main." Off sends "Auto land is off. Do not land on your own." | `draft.send`; the state is kept for each Session in the browser's storage (`deck/auto-land.ts`), so the tile reads on after a restart. Fleet has no operation for it |
| `3` | Compact | Runs the Session's `/compact` | A hosted Session is sent `/compact` as a message, as picking it from `/` does; a terminal Session is typed it through `draft.command`. Drawn off unless the agent or the terminal listed `compact` |
| `4` | Report | Swaps the tiles for a box. What he types and sends starts a new Session in Armada's repository where Fleet serves it, else the reported Session's own, and tells it his words, verbatim, then plain labelled lines of what Bridge holds about the reported Session: its ids, transcript path, repository, branch and slot, state, model, context, Jobs and pull requests, what it waits on him for, its last messages, tool calls and timeline entries, and when and where Report was pressed. The new Session is opened on the stage. Esc closes the box and keeps the words. Empty words cannot be sent | `draft.start(undefined, reportRoot(...))` then `draft.send` (`deck/SessionTiles.tsx`, `deck/ReportBox.tsx`); the message is built by `reportBrief` in `deck/report-brief.ts` from the window's state, never by asking the agent. No Fleet operation. A Session that ended or went quiet can be reported |
| `5` | Map | Closes the Session and shows the map | The stage's view switch, as tile 5 is |
| `6` | Release | Opens a confirmation that names each slot, branch, open pull request, Job and need the Session holds, each a checkbox ticked when it opens, with a picker: Release, the default, or a live Session of the same repository to hand them to. The confirm key names the act, Release all, Release while some are unticked, or Hand to and the Session's title, is off while nothing is ticked, and sends only what is ticked. Cancel holds focus and changes nothing. Once Fleet answers, what moved stands in place of the tiles, then what is still held with why, the unticked among it, until Close | `draft.releaseHolds`, over `release_session_holds` ([Session](session.md), *Releasing what a Session holds*); `deck/ReleaseHolds.tsx`. Off where the Session holds nothing |

A tile is off with its reason as the tooltip when the Session ended or its terminal is not listening, except Report, which is off only where the Session's repository is not known. A press that sends is lit until Fleet answers, and a refusal is the toast the composer shows.

## The map

The stage at rest is the map or the timeline, one switch (`m`, `t`, or tiles 5 and 6). The map draws each repository as its directories, with a drone over what it is changing. The directories are the 40 largest of the base branch's tree, counted by git, and a drone is placed from the tool events Fleet writes when a burst closes, and where it has none, from what its slot holds against the base ([Fleet](fleet.md), *The territory*). A write clash is an amber hatched slab on the directory both write in, from `list_write_clashes` ([Fleet](fleet.md), Write clashes). Both parties also carry the swords mark wherever they are drawn: their list row, their pane head and their drone, which needs no directory. The mark names the other party and a press opens it on the stage; a clash against main is a picture with nothing to open. A replay plays back the last two hours of events Bridge holds; before a bay's first event in the window it reads as unknown, not free. `v` swaps a Job's mission and its canvas.

**Nothing is drawn that Fleet does not hold.** A drone whose path is not known rests by its bay pad, and a stale tool burst reads as no action.

## The ring

A Session that has built something has a ring lying on the ground under its drone. It is Fleet's reading of the Session, not the agent's word: `SessionRecord.build` ([Session](session.md)), derived from the edits, commits, pushes, walks and pull request Fleet observed, with the agent's task list adding detail. Bridge draws what the record says and holds nothing beside it. A Session with no `build` has no ring.

| Part | What it shows |
|---|---|
| Inner ring | One segment for each task. Solid: done, and an edit was seen. Hollow, outlined: done, and no edit was seen. Scaffold: the first task not done, with edits since. Ghost, dashed: any other task not done. The ring counts the whole Session's work: the tasks a subagent writes are segments too, since subagents share the Session's task list, and one carries a dot at its middle and its hover adds what that subagent was asked (`BuildPiece.by`, kept after the subagent ends). Each subagent's tasks are drawn together as one stretch, in the order the subagents started and each where its first task stood, with a wider gap at each end and a thin arc inside it; the Session's own tasks keep list order around them |
| Outer ring | The road to main, a thinner ring of arcs: a walk shown, a push, the pull request opened, its checks, auto-merge, and the landing that closes the circle. A reached stop is lit with when it happened. A stop not reached is faint and is what the repository's recent Sessions usually did next |
| Survey | While the Session is scoping, a faint grid under the drone with a stake at each corner, and a small marker for each walk shown and each question asked |
| Workers | A small drone for each subagent running. On the map it circles the ring and takes the loops the drones leave, so the field never loops more than the existing cap; one that has written pieces stands by its stretch and does not lap. In the column it sits inside the ring at the middle of its own stretch and is still, a step inward where neighbouring stretches are too close for the drones to clear each other. Its hover says what it was asked. It leaves when the subagent ends, and its pieces stay |
| Stalled | Fleet's `stalled` draws the drone still under an amber ring. The unfinished segments dim from 0.3 at ten quiet minutes to 0.65 at forty, read from `quiet_since` |
| Landed | The last arc closes, the capture flash plays at the flag, and the ring is gone a quarter hour later. What landed is in Activity |

A hover or focus on a segment or an arc says its task or stop and where it stands, and for a subagent's piece who wrote it. No count is drawn beside the items it counts. Pieces are static once placed; only a placed piece's one rise, the map's workers' laps and the scaffold's brace in the column move, and reduced motion holds all of them still.

**Where it is drawn.** On the map, under the drone. While a Session is open, the same ring is drawn tall above its ledger in the right column, and Incoming calls stay. A Job keeps its Bays and has no band: its ring stands in its mission beside the waypoint of the step that works the plan, and its segments are the plan's tasks (done solid, handed in hollow, working scaffold, open or failed ghost, dropped none). It has no outer ring, since its route is the road. A Job's row shows Implement while its plan has tasks.

**Replay shows none.** A record of now is not the field at the playhead, so a replay draws no ring.

Why a ring, and what it cost the owner: [a Session builds a ring](../../.claude/decisions/2026-10-14-a-session-builds-a-ring.md). The owner chose on 15 Oct 2026 that subagents' work fills the ring.

## Calls

A call is something waiting on a person: a Job's gate, a question an agent asked, a Session that finished its turn and is done. **Every Job or Session whose row says it needs him is a call**, whether or not it raised one, and the Calls tile lists exactly those (the owner, 15 Oct 2026). It is raised on the deck as an Incoming call, and the deck is the only Cockpit, so there is no second place one is drawn. **One call for each Job or Session**, the newest, however many questions and waiting items it holds: opening it opens the Job or Session, which holds them all. A press opens it on the stage. A call that asks (a question, a waiting item, a Job's gate or review) stays in Incoming until it is answered; any other call, a reply, a done or stuck Session included, leaves when it is seen, and the next turn's is a new one. A Session's call opens the Session, whose own thread and Needs you band hold what it asks, so no call strip is drawn above it; a Job's call keeps its strip.

While the window is on another tab a toast says a call has come and a press brings it forward on the stage. On the Cockpit tab no toast is drawn, and the Cockpit tab carries a pip while a call waits and the window is elsewhere. A call in front takes the bare digits and letters the `call` scope names, and the deck's handlers stand down for it.

## Moving about

| To | Do |
|---|---|
| Another tab | `⌘1` Cockpit, `⌘2` Jobs, `⌘3` Sessions, `⌘4` Studios, `⌘5` Retros, `⌘6` System, or `g` then `c`, `j`, `s` or `y` for the first three and System |
| A System page | The strip under the tabs: Kit, Mods, Settings, Guides, Workflows. System opens the page last used. `⌘,` is Settings |
| Doctor | The title bar's menu, top right, or `⌘K` by name |
| A page with no tab or strip place | A link on the deck, or the palette, `⌘K`, by name: Worktree Slots, Merge line, Manifest, Checks |
| A panel | `l b c q f a d` |
| A flight control | `1` to `6`, the Session's own while one is open |
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

## What is capped, and why

The deck stays quick by drawing less, not by drawing it faster. What is held back, and where the line is drawn:

| Held back | By | Why |
|---|---|---|
| Activity counts the newest events of the last hour only | `MOST_GRAPH` in `cockpit/ActivityChart.tsx`, cut before the lanes are built, after the filters narrow | A dot costs a button, and Fleet holds far more events than a column shows |
| Ages move in whole minutes | `AGE_MS` in `cockpit/deck/clock.ts`; `CommandCentral` hands the deck the minute | A clock handed on each second drew the whole deck each second. A part that must move faster keeps a clock of its own: the field and the replay run on five seconds |
| A burst of events tells its readers at most four times a second, and the last event always lands | `fleet-timeline.ts`, through `TICK_MS` | One event, one draw, was a draw for every append of a burst |
| An idle drone runs no animation; a working or calling one does, and so does the one selected | `battlefield.css`, on `data-alive` | Any animation that runs forever forces a full frame of the map, whatever it moves. Reduced motion stays static |
| No rotor turns, and a stranded bay's ring and a picked tile's glow hold steady | `battlefield.css` | A turning SVG group, an SVG stroke and a box shadow repaint the map each frame; the rotors were most of a focused window's cost (the owner, 15 Oct 2026: the machine over the rotors) |
| The window draws no motion while hidden or unfocused, and the map none after a minute with no input | `still.ts`, `data-still` and `data-idle`, ruled in `styles/index.css` | One loop still holds the compositor at the display's rate. Any input moves the map again |

A part of the deck reads the one field of `store.ts` it draws from, `useDeck((deck) => deck.cursor)`, and not the whole state, so the cursor's move on `j` draws the rows and the map's ring and nothing else.
