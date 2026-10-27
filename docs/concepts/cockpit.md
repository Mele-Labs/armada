# Cockpit

**What it is:** Bridge's first tab and the screen it opens on — the flight deck: every Job running, every call waiting on a person, the worktree slots and the merge queue, on one screen.

---

**Kind:** Surface.

You open Bridge.
The list on the left holds a Job that is waiting and one that is working.
The waiting one's row is yellow and pulses.
Press `c` to reach it, open it and answer, and the row stops pulsing.

The Cockpit is the one body behind the Cockpit tab.
The grid it replaced, the call deck and tray, the Bays metro view and the left-column rail are gone (#2301). [Bridge](bridge.md) owns the shell around it.

## Panels

```
+-----------------+----------------------------------+------------------+
| repository      |                                  | Bays             |
| Jobs            |  Stage                           |                  |
|                 |                                  +------------------+
|                 +----------------------------------+                  |
|                 | Terminal, while a Job's shell    |                  |
|                 | is docked                        |                  |
+-----------------+----------------------------------+                  |
| Activity        | Flight    | Dispatch | Autopilot |                  |
|                 +-----------+----------+-----------+                  |
|                 | Merge queue                      |                  |
+-----------------+----------------------------------+------------------+
```

| Panel | Holds | Key |
|---|---|---|
| Repository picker | Top of the left column. A combobox: pressing it makes the field a search box in place, the list narrows as he types, arrows move, Enter picks and Esc puts the previous pick back. All repositories leads, a rule under it, then the repositories; New project and Add a repository are footer rows the filter never removes. Narrows the list, the feed, the bays and the queue to one repository, or All repositories | |
| Work | One row for each Job, drawn as *The Jobs list* says. A row draws no icon for its kind; the one glyph on a row is `link` before the name of a linked Job. Filter, one of the panel's acts, holds a search and the scopes Active, Quiet, Ended and All; `/` opens it on the search. **The live rows keep a fixed order**: a new Job lands at the bottom, nothing placed moves when a Job's state or call changes, and he moves one by dragging its row or with `⌥↑` and `⌥↓`, among the rows of its repository heading and never across one. The order is kept on this machine. The owner, 17 Oct 2026: "I click on a job and all of a sudden it jumps to the bottom of the list. Its very jarring. Jobs should have a fixed order and if I want to drag them to reorder I can". Quiet and Ended rows stay newest first. An ended Job is a row here and never a drone on the Field. **A Job started from another stands directly under its parent** with a connector, led by `link` (tooltip `Linked Session`, and `Linked Session, handed from <group>` where it was handed a plan group); a press on it opens it from the parent, and Esc returns to the parent's thread as it was. `docs/concepts/session.md`, *Linked Jobs*. **A Job dispatched to wait on another** holds no slot, shows the `after` state and says when it starts (`Starts when <Session> ends`), stands under the Job it waits on, asks nothing of the owner and takes no reply until it has started; its stage is its brief, the condition and Start now. `docs/concepts/session.md`, *A Job that waits on another* **A row that needs him is yellow and pulses**, on opacity alone and still under reduced motion; a reply that only tells wears a small faint reply glyph (tooltip `Unread reply`) and raises no call. | `l` |
| Activity | What Fleet has done in the last hour, as a beeswarm with no words: a dot for each event at its time, the newest at the right, in the colour of its tone and smaller and fainter for the coalesced reads, edits and commands. Hovering a dot opens a card of the events near it, the `Hour` card, which keeps its own look by the owner's choice (16 Oct 2026) and is not an inspect card, and a press picks the event and the map shows what moved. While a Job is open on the stage it tells of that one alone, a Job's subagents included, and tells of all again when it closes | |
| Stage | The timeline or the map at rest. When something is picked, that thing: a Job as the Job view (its head with the context figure and the rename, its thread and its box), a bay, a pull request, a call, Autopilot's lane | |
| Terminal | The open Job's shell while it is docked, between the Job and the merge queue, full width ([The shared terminal](session/shared-terminal.md)). The thread keeps the Job's whole pane. Its height is dragged against the Job's and kept with the deck's other panels; put away, the Job takes the room back | |
| Merge queue | Main as one light, then each repository's pull requests on one line that scrolls across and never wraps: those in the forge's merge queue first, in its order, framed together with each one's place, then the rest past a rule. A press on a chip opens the pull request on the stage. On All with several repositories served, an open Job or pull request narrows the line to its repository; with nothing open, the line is a stack of one row per repository, each led by its own main light named for it. One row shows at a time: it scrolls across, the stack scrolls down and snaps to a row, and ↑ and ↓ move it. A main that turns red brings its row into view, and while nothing is using the line it moves to the next repository every 15 seconds. A pull request the hub marks `merge_by_hand`, the one a Manifest form's save opened where the forge would not turn auto-merge on, carries a person's glyph after its number (`Needs merging by hand`), its pane has Merge and Review, and it is also a call, a pulsing row in Work when a Job owns it; the Manifest form shows the same line under Save. While a main is red, hovering or focusing the light opens an inspect card holding the Merge line's red panel for each red repository: what failed, the merge that broke it, who is fixing it or holds the claim, and, where nobody has it, the acts to dispatch a Job on it, the same acts and routes the Merge line sends. A press on the light pins the card; Esc or a press outside closes it. Main's light also carries a small amber mark while a served repository's checkout is behind main and Fleet would not catch it up; its dot keeps meaning only main's CI. The card then opens on a green main too, and under main's state a section, Your checkout, names what holds it back and, where one press is safe, the command that brings it level, to run or copy. Run asks Fleet to do it, which reads the hold again and refuses if it is no longer that one; Copy gives the command for a terminal, and both show without the pointer over the block. Beside them a Pull request button sends the uncommitted files up as a pull request with auto-merge on, committed by Fleet without touching the checkout; the card then shows the pull request's chip with an auto-merge mark in place of the command, drawn from the hold Fleet publishes, and the files stay listed. Once the pull request is on main Fleet catches the checkout up by itself and the mark goes. Each uncommitted path is a button: a press opens that file's change under it, read by Fleet, and a second press folds it. A change past Fleet's bound is drawn as far as Fleet sent it, with one line after it, `First N lines`. A long list of uncommitted files scrolls inside the card, the files main also changed first. A green main with a level checkout has no card | `q` |
| Bays | The worktree slots and who holds each, as *Bays* says. While a Job is open on the stage, that Job's Workflow, Blueprint, Field and Delivery are drawn here instead (*A Job open*), and the Bays return when it is closed | `b` |
| Needs you | Not a panel. A Job that needs him is yellow and pulses on its row in Work, and `c` goes to the first such row, else to the first place that holds a call no row holds (see Calling places). A reply that only tells is no call: its row wears a small faint reply glyph | `c` |
| Flight control | Checks, Calls, Map and Timeline as keys, `⌥⇧1` to `⌥⇧4` until rearranged. While a Job is open on the stage, that Job's own (*The Job's flight controls*). Edit, in the panel's top-right corner with its other acts, shapes either set (*Shaping the flight controls*). The merge queue stands under it | `f` |
| Dispatch | One tile, the same width as the others, carrying `n`. It opens the Dispatch sheet, the one `n` opens. Once a repository is chosen, `@` in the request also offers its files and `#` its open issues and pull requests, as a Job's message box does. A request that tags a Job with `@` and says when draws a sentence over it saying when the new Job starts; a tag with no condition in the words only refers to the Job and starts the new one at once, and `⌥A` opens the sheet with `@` typed | `d` |
| Autopilot | Hidden unless Settings → Features shows it. Shown, it is one row, its name and its switch. On, it gives the stage to the lane, with its sidings, Next up and Set aside | `a` |

**Hovering or focusing a row of Jobs, a bay, a merge queue chip or a drone on the map opens the inspect card** (`docs/contracts/design-system.md`, Inspect card): the kind, the name, the state, the facts the row already holds and the key that opens it.
A press on each of these is already an act, so none of these cards pins.

The two side columns resize.
A handle sits in the gap on each inner edge, drawn only while hovered, focused or dragged, and it is `DockHandle` (`packages/components/src/dock-handle.tsx`), the one Helm's dock and every sheet draw.
A drag or `←` `→` moves it, and a double click or `Home` puts the column back to its resting width.
The left column stops at 4.5 `--space-12` and the right at 4.75, and neither goes wider than leaves the stage 10 (480px), measured against the window and taken again when it resizes.
The widths are kept in this browser as dragged (`armada.bridge.deck-columns`, `deck/columns.ts`), so a column dragged wide in a big window returns to that width when the window does.
The deck has no narrow layout, so there is none to leave the handles out of.
The gap between two panels of a column is a handle too, and their heights are kept as shares of the column (`deck/splits.tsx`).
The foot of keys has no handle and is as tall as its keys, so a share kept from the older, taller foot cannot stretch it; a foot holding the Field still resizes.

`Esc` returns the stage to what it was, and with nothing open it lets the cursor go, so no row is picked and the map holds no drone's card.
A press on the list or the map away from a row or a drone lets go the same way.
The state is held in `apps/desktop/src/renderer/src/cockpit/deck/store.ts`, so the title row, the body and the keys are one thing, and leaving the tab and coming back returns the same view.

## Density

The Cockpit fits more than the rest of the app, the way mission-control and F1 telemetry screens do.
Its measures are the app's since 26 Oct 2026: the type ladder is `--text-2xs` 12px, `--text-xs` 13, `--text-sm` 14 and `--text-base` 15, with the row title at 13 and its meta at 12.
Gutters between panels are 6px, the page gutter is 12px, and the controls are 24, 28 and 32px.
[Typography](../contracts/design-system/typography.md) and [Spacing and shape](../contracts/design-system/spacing-and-shape.md) hold the decision and why.

**A panel has no name band.**
Its acts show in its top-right corner on hover or focus, and its name stays as its accessible label.
A folded panel, or one that is only a band, keeps its band, because the band is how it opens.

**Deck key caps are hidden until `⌘` is held.**
The caps crowded the small deck.
[The keyboard contract](../contracts/design-system/keyboard-and-command-palette.md) holds the exception and what it costs.

## The title bar

The Places, Cockpit, Studios, Logbook and System, are glyphs, each with its name in a tooltip.

The notch holds only the Places.
A right press on a place, or the ContextMenu key or Shift+F10 on a focused one, opens a menu: Hide, then a View group with every place checked while it is drawn, which stays open as each is toggled.
A right press on the notch and on no place opens the View group alone.
Cockpit and System are always shown, so their items are drawn disabled.
The menu writes the owner's own layout for the places, the same switches System → Layout shows. [Layout mods](layout-mods.md).

The Job's head keeps its title: the state glyph, then the name, which a click renames, and the glyph pulses while the Job works.
After the title it holds, in order:

| Part | Is |
|---|---|
| Artifacts | The `library` glyph. Opens everything the Job made or used |
| Checks | The shield, pulsing in the running hue (blue) while any of the Job's Checks runs |
| Subagents | The `drone` glyph, pulsing while any runs. The number running is in its tooltip, not beside the glyph |
| Bays | Opens the Job's bays and write clashes |
| The context figure | How full the context window is. Its popover holds Compact, which runs the Job's `/compact`: a hosted Job is sent it as a message, as picking it from `/` does, and a terminal Job is typed it through `draft.command`. Compact is off, with its reason, unless the agent or the terminal listed `compact` |
| `⋯` and close | The Job's other acts, then closing the Job |

Each view icon opens the layer *The Job's flight controls* names, lit while its view is open, and a press on the lit icon returns to the thread.
An icon is off, with its reason as the tooltip, where its view has nothing in it, and keeps its place.

## The Jobs list

A row is one line: the name, then status glyphs in fixed columns, so the glyphs line up from row to row.
Rows are divided by a hairline and the repository groups are spaced apart.
Hovering a row opens its full inspect card.

A right press on a row, or on the list's empty room, opens a menu whose View group has seven checkable fields, in this order:

| Field | Draws |
|---|---|
| Active PRs | The Job's open pull request, on a line of its own under the row, under Now where Now is on |
| Issues | The issue attached to the Job |
| Now | The last thing the Job said, as a second line |
| Time | How long since its last turn |
| Conflicts | The write-clash mark |
| Status | The Job's state mark |
| Workflow | The step of the Job's workflow it is at |

The default is Status and Conflicts.
Every field except Now and Active PRs has a column of its own, the same for every row, so a row without the field leaves its place empty and nothing moves.
A column no row fills collapses.
The choice is kept with Bridge's settings.

## Bays

A bay's first cell is its number.
It draws no held time, no kind glyph and no state glyph.
The number cell's colour is the bay's state, and its tooltip names the state.

## With no repository

When Fleet serves no repository, the stage is a start menu over the live battlefield.
ARMADA stands above three starts, each with one plain line, and `j` and `k` move between them.

| Start | Asks | Then |
|---|---|---|
| Open a project | The macOS folder dialog | Adds the folder |
| New project | A name and a location | Makes the folder, runs `git init` in it and adds it |
| Import from Claude Code | Which of the folders Claude Code kept sessions for to add, and whether to install the Armada mod | Adds the folders, having learned how each runs from its past sessions |

**The location defaults to the folder under home that most Claude Code projects share, else `~/code`, made if missing.**
Every start ends the same way: the Cockpit, with a new Job open on the folder, empty, its box focused.
Import opens it on the first folder added.
Past the first run, New project in the repository picker opens the Add a repository dialog on a name and a location, and leaves the pick where it was so the new Job sits under its own heading.

**Not built: the start menu.** On a fresh install Bridge opens the Add a repository dialog by itself, which is the `first-launch` scenario in `docs/practices/running-locally.md`.
The mock walks `first-run`, `first-run-menu`, `first-run-new` and `first-run-claude` draw the menu.
Fleet already serves each start: `create_project`, `list_import_folders`, `get_import_runs` and `install_agent_mod`.
[First-run onboarding](../journeys/first-run-onboarding.md) is the journey.

## The Job's flight controls

While a Job is open on the stage the Flight control panel is that Job's, in the same place.
It is one row of tiles: Auto land, Pause, Judge, Report, Release, Terminal and Requests, then any tile the owner added.
When the Job closes the deck's tiles return.

The tiles are the deck's own, the same as Checks, Calls, Map, Timeline and Dispatch with no Job open, drawn as physical keys.

| State | The key is |
|---|---|
| At rest | Raised by a crisp bottom edge |
| On or pressed | Seated 1px, with an inset top shadow |
| Unavailable | Flat and recessed, with its reason as the tooltip |

Auto land and Pause carry an annunciator lamp, a flat bar at the top of the key, in the accent while on.
A key's tooltip floats above the panel.
Edit is a `square-pen` glyph.
The merge queue stands under Flight control.

A tile's key belongs to its place and not to whether the tile is on, so a tile going off moves no other's; `tileKey` in `deck/session-tiles.ts` holds them.
Pause answers `p` unless it is given another key.

**What is not here.**
Update is gone.
Compact is in the context figure, whose popover holds it.
Map is the stage's own switch.
The Job's views, Artifacts, Checks, Subagents and Bays, are icons in the Job's head (*The title bar*).
There are no pages: every tile is on the one row.

| Tile | Does | Through |
|---|---|---|
| Auto land | Turns the Job's Auto land flag on or off, and its lamp is lit while on. On also sends "Auto land is on. Fleet turns on auto-merge when your pull request opens and merges it once ci is green. Open the pull request. Fix ci failures and merge conflicts when you are told of them. Do not merge it yourself." Off also sends "Auto land is off. Fleet turns auto-merge off. Do not land on your own." Fleet holds the flag, so it survives a restart; a forge refusal is a toast and the flag still changes | `set_session_auto_land` (`POST /sessions/auto_land`, `{id, on}`; `crates/ipc/operations/set_session_auto_land.toml`), read as `Session.autoLand`, then `draft.send` for the words. Off where Fleet does not offer it |
| Pause | Pauses the Job, and reads Resume while it is paused, its lamp lit while paused | `draft.pause` and `draft.resume` ([Pausing a Job](session/pausing-a-job.md)). Off where Fleet does not offer it |
| Judge | Sends a judge to read the work against the plan ([Job](session.md), *The judges*) | `draft.work.judge`. Off with no plan to judge against |
| Report | Swaps the tiles for a box. What he types and sends starts a new Job in Armada's repository where Fleet serves it, else the reported Job's own, and tells it his words, verbatim, then plain labelled lines of what Bridge holds about the reported Job: its ids, transcript path, repository, branch and slot, state, model, context and pull requests, what it waits on him for, its last messages, tool calls and timeline entries, and when and where Report was pressed. The new Job is opened on the stage. Esc closes the box and keeps the words. Empty words cannot be sent | `draft.start(undefined, reportRoot(...))` then `draft.send` (`deck/SessionTiles.tsx`, `deck/ReportBox.tsx`); the message is built by `reportBrief` in `deck/report-brief.ts` from the window's state, never by asking the agent. No Fleet operation. A Job that ended or went quiet can be reported |
| Release | Opens a confirmation that names each slot, branch, open pull request and need the Job holds, each a checkbox ticked when it opens, with a picker: Release, the default, or a live Job of the same repository to hand them to. The confirm key names the act, Release all, Release while some are unticked, or Hand to and the Job's title, is off while nothing is ticked, and sends only what is ticked. Cancel holds focus and changes nothing. Once Fleet answers, what moved stands in place of the tiles, then what is still held with why, the unticked among it, until Close | `draft.releaseHolds`, over `release_session_holds` ([Job](session.md), *Releasing what a Job holds*); `deck/ReleaseHolds.tsx`. Off where the Job holds nothing |
| Terminal | Opens the Job's shell, docked in its panel between the Job and the merge queue, in the side panel, or in a window of its own, as `terminal.openJobTerminalIn` says, with a new shell where none is running. Pressed while the shell shows docked or in the side panel, it puts the shell away and the shell keeps running. Pressed while the window is open, it brings the window to the front | `useJobTerminal` in `job-shell-dock.tsx` ([The shared terminal](session/shared-terminal.md)). Off where the Job ended or Fleet serves no shell. |
| Requests | Opens Requests in the side panel for this Job: the repository's collections and environments, the Job's own sends under History, and the Job's running servers as environments. A send with the Job open writes a row in its thread and tells its agent. A `saved` or `requested` row in the thread opens it there too | `openRequestsInPanel(session.id)`; the panel is `shared-panel/requests.tsx`, over the five operations in `docs/practices/protocol.md`, *Requests*, and `docs/concepts/requests.md`. Always on. |

A tile is off with its reason as the tooltip when the Job ended or its terminal is not listening, except Report, which is off only where the Job's repository is not known.
A press that sends is lit until Fleet answers, and a refusal is the toast the composer shows.
Terminal is lit while the Job's shell shows, docked, in the side panel or in its window, and a press while the window is open brings it to the front.

**The views.**
Each opens a layer over the thread, or over the right column for Artifacts, and is a press on its icon in the Job's head.
Checks pulses while any of the Job's Checks runs, and is the one place that shows it: the thread's header carries no checks mark.
Subagents pulses while any runs and carries the number running in its tooltip, so no count is drawn beside the glyph.

| View | Opens | Through |
|---|---|---|
| Artifacts | Opens everything the Job made or used over the right column, and the composer has a button for it (`⇧⌘A`) | A layer (*A Job open*). Always on |
| Subagents | Opens every subagent the Job started | A layer. Off, with its reason, where it started none |
| Checks | Opens the Checks the Job ran | A layer. Off where it ran none |
| Bays | Opens the Job's write clashes, then every bay the Job holds, drawn as the Bays panel draws it with how far it is behind its base | A layer. **Lit in the stranded hue only when a bay is stranded or 20 or more commits behind its base**, and off where the Job holds no bay. A write clash puts the swords glyph at the key's corner and leaves the key unlit, named for the other Jobs; the Job's header carries no clash notice. A clash's file, pressed in the panel, opens as the two sides' patches in a sheet beside the stage, read by Fleet from git at the press. |

## Shaping the flight controls

Edit, a `square-pen` glyph, is one of the Flight control panel's acts in its top-right corner, and the panel's key is `f`.
Pressing Edit turns the tiles of the set drawn, the Job's or the deck's own with no Job open, into things to arrange where they stand; Done and Cancel take its place.
The owner's ask of 18 Oct 2026.

| To | Do |
|---|---|
| Move a tile | Drag it, or `Alt` with an arrow while it is focused, or `Space` to lift it, the arrows to move it and `Space` to drop it |
| Hide or show a tile | `Delete` or `Backspace`. A hidden tile is drawn quiet and dashed, keeps its place and has no key |
| Change a tile | `Enter` or a press opens its sheet |
| Add an action | The `+` tile |
| Leave | Done saves; `Esc` leaves without saving |

The tiles keep the size they have outside Edit.
Hidden tiles and `+` follow the shown ones.

| Kind | Does | Offered |
|---|---|---|
| Words | Sends the Job its words through `draft.send` | The Job's tiles |
| Slash command | Types the command to the Job as the context figure's Compact does, and is off where the agent or terminal has not listed it | The Job's tiles |
| Shell command | Runs the line in the Job's bay through `run_session_shell` (`docs/concepts/kit.md`, *Flight controls*). The output stands in place of the tiles, where they were, until Close. Off where the Job holds no bay | The Job's tiles |
| Page | Opens the address as Bridge opens an external page. In the Job's tiles `{branch}`, `{pr}` and `{repository}` are filled from the Job, and the tile is off where one cannot be | Both sets |

With no Job open the deck's tiles edit the same way, and only Page can be added there.

A built-in has a name, a glyph, a key and whether it is shown; Auto land also has the words it sends and Compact the command it types, and Reset puts Armada's own back.
A key is set by pressing it in the tile's sheet: a digit, a letter, or a chord with ⌘, ⌃ or ⌥ such as `⌥R` or `⇧⌘J`, spelled as the keymap spells it and kept so in the file.
A lone letter in an older file means that key unshifted.
Esc lets go of the control and Delete clears the key.
Every tile can have one, Pause too.
A key another tile has, or the Cockpit already answers, is named under the control ("Used by Report", "Used by Show the map") and Save waits; so are ⌘C, ⌘V, ⌘X, ⌘A, ⌘Z and ⇧⌘Z, which a text box keeps. ⇧ with a letter is not a key, and ⌘ and ⌃ are one key to a tile, as they are to the keymap.
Tile keys are not listed in Settings → Keyboard: Edit is the one place to set them.
A key presses its tile anywhere on the Cockpit while a Job is open.
In a text box a chord with ⌥ or ⇧ types, and one with ⌘ or ⌃ presses.
Nothing presses while the tiles are edited.
By default keys follow position, `⌥⇧1` to `⌥⇧9` then `⌥⇧0`, and Pause answers `p`; a bare digit is a question's answer.
Once Pause has another key, `p` is free for a tile given it; in the list `p` still pauses the row under the cursor.
A built-in the file does not list is appended in Armada's order.

What is kept is `~/.armada/flight-controls.toml`, the third Kit file, read and saved through Fleet's `get_flight_controls` and `save_flight_controls`.
Its shape, and the rules a save is refused for, are in `docs/concepts/kit.md`, *Flight controls*.
Bridge reads it when Fleet connects and saves the whole file on Done (`deck/flight-controls.ts`).
A file that will not read leaves the built-in tiles, and a save that fails stays for that window.
Bridge refuses a `kind` on a built-in id and `on_text` or `off_text` off the Auto land tile before it saves; the words are the ones the tile sends with the flag.
Comments typed into the file by hand are lost on a save.

## A Job open

Since 16 Oct 2026. The owner, 16 Oct 2026, short of room: the Work list, the thread and composer, the right column, and Flight control, and nothing else.
The right column tells the Job as a story, top to bottom: Workflow, Blueprint, Field and Delivery (`docs/concepts/session.md`, *In Bridge*).
Every Job with `work` is drawn this way; one without is drawn as it was, with its Ledger.

```
+------------+-----------------------------+--------------+
| Work       |  thread ..................  | Workflow     |
|  rows pulse|  [Needs you ^] band         | Blueprint    |
|  when      |  composer                   | Field        |
|  calling   |                             | Delivery     |
+------------+-----------------------------+              |
| Activity   | Flight control              |              |
+------------+-----------------------------+--------------+
```

| | |
|---|---|
| Calls | What waits on him is one **Needs you** panel joined to the composer: a band on its top edge, in the waiting hue, with a glyph for each thing. A press rides the band up as the panel's header and dims the thread behind it. It opens as a list, the question first, then a judge's concerns, then the rest, one row each and no act on the row; a row opens that one thing to answer or address, and Back, Esc or Backspace returns to the list with focus on the row left. Esc on the list, or a press on the dimmed thread, folds it. A question opens it once, to the list. Nothing is pinned and the thread never moves. The owner turned down a call rail ("Lets just make the rows in the work panel pulse"), pinned and composer-drawn calls, the thread's last card and pills, and took the band on `?walk=needs-you-panel`. A plan to approve stays the thread's last card, with a chip over the composer that names it when scrolled away |
| Calling rows | A Job calling him, other than the open one, pulses a 2px bar on its row in Work, by its opacity alone and still under reduced motion; the row's hover card names the call. A row the filter hides pulses again when it comes back |
| Calling places | A call no Job's row holds pulses the place that already shows it, in the same tint and pulse, by opacity alone and still under reduced motion, and nothing is drawn or counted for it: main red or a held checkout, main's light in the merge queue; a pull request that failed, cannot merge or waits on a person, its chip in the merge queue; a decision Sleep made, the title row's menu button; Helm's own Job, the title row's Helm button while its dock is shut. Opening or answering the call stops it, as it does a row. `c` goes to the first such place when no row needs him, and `w` opens the next call, wherever it lives |
| Field | The Job's drones at work, the third panel of the right column, open at its stage; `e` opens it large over the stage. **The floor is the whole repository**, every area `list_repository_areas` names, so its shape never changes as the Job moves; the plots its plan's scope and its drones name are outlined in the accent. Its own drone and a small one for each subagent at work fly to the file each is on. The drones of every other live Job in the repository are drawn at a quarter strength, with no beam, label or action; hovering one opens its card and pressing it opens that Job on the stage, as on the map. The camera rests on the whole repository, as the map's does, and picking a drone in the Drones menu does not move it. **Each file the Job wrote glows amber and each it read glows a faint cool blue** (`--tool-look`), fading linearly over ten minutes for a write and five for a read, from the times of Fleet's `session.edited` and `session.read` bursts. A path spelled under a slot or a worktree inside the repository, as a terminal Job at its root names them, is drawn at the repository's path; a path Fleet cut at its line limit glows on the one file the Job wrote that it can be, and on none where there is no such file. The plan is one bar along its foot, and under it a slider runs from the Job's first read or write in the last two hours to now, a tick for each: scrubbed back, the glow and the drone stand as they were and a numbered route joins its stops, dotted into a read and dashed into a write, with Replay in the corner. Its right end, End, or letting go there is live. The arrows step a read or write. **A drone not coding leaves the floor**: a pad at the near edge, right of the base, carries a light for each check the Job ran, unlit until it starts in this round (a round starts at the Job's last edit), pulsing in the in-flight hue while it runs, green on a pass and red on a fail. The drone lands on the pad while a check runs, and while Fleet holds a walk window open with no end it sits there under a plain frame naming the walk. A failing check draws a red dashed line to the file its `check` row's `blame` names, outlined in red, and no line where the row carries none: Fleet reads the file and line from the failed run's log as it ends, preferring the code under test to the test file, and the run's `target` is never read for it. Hovering the blamed file says the check and the line, `test failed at layout.ts:142`. A drone whose turn works with no tool call for twelve seconds rises over its last file with no beam, three dots circling it; one calling a service beams to that outpost; one waiting on you is amber. A subagent is smaller, joined to the Job's drone by a faint line, lifts off it as it starts and flies back into it as it ends. Its card names the Job it works for in small caps and the task it was given as its title, its type where no task is recorded, and `No task recorded` where Fleet knew neither. A Drones menu over the map names the drone the card follows and what it is doing; it lists Newest writer first, then the Job's own drone and its subagents as they started, each named with what it is doing (`Writing layout.ts`, `Running checks`, `Thinking`, `Waiting on you`), a finished one greyed; picking one pins the card to it, and Newest writer lets go. **The file a drone of the Job is writing lifts off the floor** as a slab carrying a minimap of the file, a row for each stretch of it lit green where lines were added and red where they were removed, read from Fleet's `get_session_file_change`; its place on the floor is outlined dashed, and it settles back when the drone moves to another file or stops writing. A card at the top right, over the map and joined to that slab by a dashed line, names the file with `+N −M` and shows the newest hunk, typing out only the lines new since the last answer; the files the Job wrote before it stand behind as one or two faded tabs. With several writers the card follows the one that started writing last, and a drone picked in the menu holds it while it writes, until Newest writer is picked; a pinned drone that stops writing hands the card to whoever writes; every other writer's file lifts smaller with its own `+N −M`. The Job's own drone writes the file of the newer of its last touch and its last write burst, never a file it only read. A refused file lifts nothing and says nothing, and a diff Fleet cut ends `First N lines`. Fleet is asked as a drone starts writing a file, then every two seconds while one writes it and on a `session.edited` entry naming it, never more than once in two seconds for a file and always with the stamp held, so an unchanged file is answered `unchanged`; nothing is asked while the Field is scrubbed back or folded or the window is hidden. The file's length is not on the wire, so the minimap spans the file as far as its last hunk reaches. The card never moves the camera; where the card's drone or slab stands under it, the card stands at the top left instead. In the side panel the card is about half the panel wide; the Field large has room for a wider, taller card. **While the pointer or focus is in the panel, the card and the hover card float over the app** at the Field large's size, reaching past the panel toward the thread; a grace after both leave, or on Esc, they go back inside. The hover card takes the pointer: off the thing it is about it waits the inspect card's grace, and the pointer on it holds it. **While the Job messages another Job, or is messaged by one, an arc runs through the air between their two drones**, read from Fleet's `session.messaged` entry (the sender its subject, the receiver its `to`) for as long as a burst stays fresh, two minutes and fifteen seconds. The other Job's faded drone comes up to full strength with its name and beam while linked and fades again after. Three packets travel the arc from the sender to the receiver, and the message is quoted at its middle, cut to fit. A press on the arc or the quote opens the other Job, as a press on its drone does. The menu says `Messaging` or `Messaged by` and the other Job's title, unless the Job is waiting on you. One arc for each other Job, from its newest message; none to a Job with no drone on this Field, whose words still show, and none while scrubbed back. Fleet records no message to or from a terminal Job, so none draws an arc. The dots, a pulsing light, the card typing and an arc's packets count toward the loops the map runs at once, and stand still under reduced motion, where the hunk shows whole and the slab rises at once. Approved 22 Oct 2026: `.claude/decisions/2026-10-22-a-jobs-field-is-the-whole-repository.md` |
| Ledger | Not drawn. Everything it held is in the Artifacts layer |
| The story | Workflow, Blueprint, Field and Delivery, top to bottom in the right column. The panel the Job's stage names opens and the rest fold to short bands; `⌥W`, `⌥B`, `⌥F` and `⌥D` open or fold one by hand. Artifacts is a button in the composer, and `⇧⌘A`, which opens its list over the right column and Esc returns, with List and Graph as icon toggles (`V`). `docs/concepts/session.md`, *In Bridge* |
| Padding | The Job's card, its message column and its box are half as padded, here and not in the components |
| The bays | Drawn only when something is wrong: the Bays icon in the Job's head lit, the swords glyph on it for a write clash, and a call in the thread for each stranded bay. A bay behind its base and a write clash raise no call, since Fleet tells the agent |

The right column rests wider for a Job, 8.5 `--space-12`, so its four panels share it.
The Job head's Checks, Subagents and Bays icons open a layer over the thread, and Artifacts one over the right column, and Flight control's Requests opens the side panel: *The Job's flight controls*, and `docs/concepts/session.md`, *A Job open*, for what each reads.

## The map

The stage at rest is the map or the timeline, one switch (`m`, `t`, or the deck's Map and Timeline tiles).
The map is drawn on one canvas (`cockpit/deck/pixi/`) and draws each repository as its directories, each filled with a pill for every file it holds, with a drone over what it is changing.
A Job's named files light their pills: green for a file made, blue for one changed.
The directories are the 40 largest of the base branch's tree, counted by git, and a drone is placed from the tool events Fleet writes when a burst closes, and where it has none, from what its slot holds against the base ([Fleet](fleet.md), *The territory*).
A write clash is an amber hatched slab on the directory both write in, from `list_write_clashes` ([Fleet](fleet.md), Write clashes).
Both parties also carry the swords mark wherever they are drawn: their list row, their pane head and their drone, which needs no directory.
The mark names the other party and a press opens it on the stage; a clash against main is a picture with nothing to open.
A replay plays back the last two hours of events Bridge holds; before a bay's first event in the window it reads as unknown, not free.

**What can be pressed is a layer of elements over the canvas.** The canvas takes no pointer.
Over it stands `pixi/Hits.tsx`: an invisible option for each drone, bay pad, flag, outpost, pull request crate and piece of a ring, a button for a drone's call and one for each write clash it is in, put where their pictures are each time the view moves.
They carry the role, the label, the keys and the card, so the keyboard, a screen reader and the owner's notes find what the canvas draws.
The plots, files and bases are only hovered, and the pointer is asked where it is (`pixi/plates.ts`), since a repository has as many of them as it has files.

**Nothing is drawn that Fleet does not hold.** A drone whose path is not known rests by its bay pad, and a stale tool burst reads as no action.

## Outposts

A Session's calls outside the repository, drawn as stands in one row past the far edge of the field.
One stands for each service any Session in the map's scope has called, and it appears with the first call seen.
The order is the forge, the issue tracker, the web, then every other service by name.
A glyph and a label from the newest call's `call.name` mark each.
A Job's own Field (*A Job open*) draws the services that Job alone has called, and its drone's beam runs to the one it is calling.

**Read from Fleet's `session.called` entries, never from an agent's word** ([Fleet](fleet.md), *What a Session calls outside the repository*). `cockpit/deck/outposts.ts` holds the reading.

| | |
|---|---|
| Lit | While a working Session's latest `session.called` is fresh, the same freshness a doing has (`doing.ts`), a beam runs from its drone to the outpost and the outpost lights. An idle, waiting or finished Session calls nothing |
| Pressed | A Map option, in Tab order after the pads and flags. Hover or focus opens its card of up to five recent calls, newest first, each the Job, the line and its age. A press does nothing more |

## The ring

A Job that has built something has a ring lying on the ground under its drone.
It is Fleet's reading of the Job, not the agent's word: `SessionRecord.build` ([Job](session.md)), derived from the edits, commits, pushes, walks and pull request Fleet observed, with the agent's task list adding detail.
Bridge draws what the record says and holds nothing beside it.
A Job with no `build` has no ring.

| Part | What it shows |
|---|---|
| Inner ring | One segment for each task. Solid: done, and an edit was seen. Hollow, outlined: done, and no edit was seen. Scaffold: the first task not done, with edits since. Ghost, dashed: any other task not done. The ring counts the whole Job's work: the tasks a subagent writes are segments too, since subagents share the Job's task list, and one carries a dot at its middle and its hover adds what that subagent was asked (`BuildPiece.by`, kept after the subagent ends). Each subagent's tasks are drawn together as one stretch, in the order the subagents started and each where its first task stood, with a wider gap at each end and a thin arc inside it; the Job's own tasks keep list order around them |
| Outer ring | The road to main, a thinner ring of arcs: a walk shown, a push, the pull request opened, its checks, auto-merge, and the landing that closes the circle. A reached stop is lit with when it happened. A stop not reached is faint and is what the repository's recent Jobs usually did next |
| Survey | While the Job is scoping, a faint grid under the drone with a stake at each corner, and a small marker for each walk shown and each question asked |
| Workers | A small drone for each subagent running. On the map it hovers still on the ring, by its stretch where it has written pieces and by its place among the subagents where it has not. In the column it sits inside the ring at the middle of its own stretch and is still, a step inward where neighbouring stretches are too close for the drones to clear each other. Its hover says what it was asked. It leaves when the subagent ends, and its pieces stay |
| Stalled | Fleet's `stalled` draws the drone still under an amber ring. The unfinished segments dim from 0.3 at ten quiet minutes to 0.65 at forty, read from `quiet_since` |
| Landed | The last arc closes, the capture flash plays at the flag, and the ring is gone a quarter hour later. What landed is in Activity |

A hover or focus on a segment or an arc says its task or stop and where it stands, and for a subagent's piece who wrote it.
No count is drawn beside the items it counts.
Pieces are static once placed; only the scaffold's brace in the column moves, and reduced motion holds it still.

**Where it is drawn.** On the map, under the drone.
A Job with `work` does not draw the ring tall: the Blueprint and the Field are where its build is read.
A Job with no `work` is drawn as it was, the ring above its ledger in the right column.

**Replay shows none.** A record of now is not the field at the playhead, so a replay draws no ring.

Why a ring, and what it cost the owner: [a Job builds a ring](../../.claude/decisions/2026-10-14-a-session-builds-a-ring.md). The owner chose on 15 Oct 2026 that subagents' work fills the ring.

## Calls

A call is something waiting on a person: a question an agent asked, a Job that finished its turn and is done.
**Every Job whose row says it needs him is a call**, whether or not it raised one, and the Calls tile lists exactly those (the owner, 15 Oct 2026).
It is drawn on the Job's row in Work, yellow and pulsing while it needs him, and the deck is the only Cockpit, so there is no panel of calls and no second place one is drawn.
**One call for each Job**, the newest, however many questions and waiting items it holds: opening it opens the Job, which holds them all.
A press opens it on the stage.
A call that asks (a question, a waiting item or a `replied` item, a reply that asks him something in prose) keeps its row pulsing until it is answered; any other call, a done or stuck Job included, stops when it is seen, and the next turn's is a new one.
A reply that only tells raises no call.
A Job's call opens the Job, whose own thread and Needs you band hold what it asks, so no call strip is drawn above it.

While the window is on another tab a toast says a call has come and a press brings it forward on the stage.
On the Cockpit tab no toast is drawn, and the Cockpit tab carries a pip while a call waits and the window is elsewhere.
A call in front takes the bare digits and letters the `call` scope names, and the deck's handlers stand down for it.

## Moving about

| To | Do |
|---|---|
| Another tab | `⌘1` to `⌘8` for the tabs the bar draws, in its order (Cockpit, Jobs, Studios, Logbook, Reviews, Issues, Projects, System), or `g` then `c`, `j`, `s` or `y` for the first three and System |
| A System page | The strip under the tabs: Kit, Mods, Settings, Guides, Workflows. System opens the page last used. `⌘,` is Settings |
| Doctor | The title bar's menu, top right, or `⌘K` by name |
| A page with no tab or strip place | A link on the deck, or the palette, `⌘K`, by name: Worktree Slots, Merge line, Manifest, Checks |
| Requests | `⌘K` by name, which opens it with no Job and a repository picker; with a Job open, its Requests tile or a request row in its thread |
| A panel | `l b c q f a d`; `c` goes to the first row in Work that needs him |
| A flight control | The key on the tile: `⌥⇧1` to `⌥⇧4` by default, and with a Job open its own and `p` for Pause, or whatever key each tile was given |
| Walk the list | `j`, `k` and `Enter` or `o` to open, in a list: Jobs, Activity, Bays and the merge queue; `w` brings a waiting call back; `⌥↑` and `⌥↓` move the Job under the cursor up or down the Work list; `?` lists the keys |
| Fleet's panel | The status dot on the title row |

The keys are rows in `crates/core-model/domain/actions.toml`, rebound as [Key bindings](key-bindings.md) says.
The tab bar's order and the places a layout may hide are in [Layout mods](layout-mods.md).
No page is reachable only by a place the window does not draw.

The deck links to three of them: the Bays band to Worktree Slots, the merge queue's mark to Merge line, and a control beside the repository picker to the Manifest.
Checks has no link and is reached by name.

## What Fleet serves the map, and what it does not

The deck draws what Fleet holds and leaves the rest empty. F1 and F2 of #2301 are served; F3 and F4 are not.

| | Fleet gives | Where it gives none the deck shows |
|---|---|---|
| F1 | `list_repository_areas`: the directories of each repository's base to two names deep, the files of each, the 40 largest | Pads and the main flag in each territory, no plots |
| F2 | `list_slot_touches`: for each Job holding a slot, the paths it changed since the slot met the base, uncommitted and untracked included | A drone resting by its bay pad, the slab flat; never a guessed directory. |
| F3, not yet | Why a lease ended, and its branch, on `slot.released` | A bay's history with no end glyph and no branch on past rows; a bay with no events draws no History |
| F4, not yet, optional | A burst-open event, so what a drone is doing is live | What a drone is doing from closed bursts only |

**F2 is the last resort for a drone.** A tool burst places a drone first, then what its slot holds against the base.
A holder a repository's slots do not name, and a Job in a terminal Fleet does not host, has no such read and rests at its pad.
A repository with more than 40 directories is drawn from its 40 largest, and a holder with more than 200 changed paths from the first 200 in the alphabet.

Both reads are polled, since Fleet pushes no event for either: when Fleet connects, and every 35 seconds while the board changes or a holder has changes.
A Fleet before them answers 404 and the deck draws none.

## What is capped, and why

The deck stays quick by drawing less, not by drawing it faster. What is held back, and where the line is drawn:

| Held back | By | Why |
|---|---|---|
| Activity counts the newest events of the last hour only | `MOST_GRAPH` in `cockpit/ActivityChart.tsx`, cut before the lanes are built, after the filters narrow | A dot costs a button, and Fleet holds far more events than a column shows |
| Ages move in whole minutes | `AGE_MS` in `cockpit/deck/clock.ts`; `CommandCentral` hands the deck the minute | A clock handed on each second drew the whole deck each second. A part that must move faster keeps a clock of its own: the field and the replay run on five seconds, and a Job's Field glow on fifteen |
| A burst of events tells its readers at most four times a second, and the last event always lands | `fleet-timeline.ts`, through `TICK_MS` | One event, one draw, was a draw for every append of a burst |
| The map draws a frame when something changed, and its ticker runs only while a drone's action or a play of an event does | `pixi/stage.ts`; at most `MOST_LOOPS` drones act at once, at thirty frames a second where the camera is still, and the ticker stops itself | A loop that runs forever holds the compositor at the display's rate. A scene rebuilt with the same content (`pixi/same.ts`) draws nothing. Reduced motion stays static |
| The floor is built once for each tree, and the bars, glow, outlines and route on their own layer | `pixi/stage.ts`, `pixi/tiles.ts`, `pixi/marks.ts`; a glow held to fortieths (`afterglow.ts`) | A Job's Field draws every area of the repository. Drawing it again for each event, or each clock step that moved a glow, is a floor's cost for a bar's change |
| A folded pane draws and builds nothing | `PixiMap`'s `hidden`, from the pane's size; the scene it is handed waits until it is shown | A folded Field kept its drones' loops running on the ticker |
| No rotor turns and no drone bobs or circles: it hovers still, and what it is doing is drawn on the ground under it | `pixi/drones.ts`, `pixi/ground.ts` | The rotors were most of a focused window's cost on the CSS map (the owner, 15 Oct 2026: the machine over the rotors) |
| The window draws no motion while hidden or unfocused, and the map none after a minute with no input | `still.ts`, `data-still` and `data-idle`, ruled in `styles/index.css`, and read by `pixi/stage.ts`, which stops its ticker and draws nothing until they are cleared | One loop still holds the compositor at the display's rate. Any input moves the map again |

A part of the deck reads the one field of `store.ts` it draws from, `useDeck((deck) => deck.cursor)`, and not the whole state, so the cursor's move on `j` draws the rows and the map's ring and nothing else.
