# Project

**What it is:** The set of repositories one person works across as one body of work, with the tasks he keeps for it.
A member is a whole repository or any folder inside one, and a folder needs no `armada.yml` of its own.
A Job is started for a project or moved into one, lifecycles and rules are scoped to one, and a task can sit in one.
A task can link issues, pull requests and a running Job, and Fleet asks him before it marks one done.

---

**Kind:** A file at a place he chooses, and tasks in a store.
The file is `projects/<id>.yml` in Kit's folder, or the same file in a synced folder or the cloud, chosen per project and This Mac by default.
It names the project, lists its members and holds the values the project sets, and it is never kept in a repository.
The tasks are in Fleet's store on the Mac that holds the project and survive a restart, wherever the file is.
A project he shares is also kept on his other paired Macs as a copy ([Sharing with his other Macs](#sharing-with-his-other-macs)).
The wire is [Projects: tasks and linked issues, across repositories](../practices/protocol/projects.md), and this page does not repeat it.

## The model

**A project is his, not a repository's, and a repository may be in several.**
Its members are the repositories its tasks name and the ones he adds.
The word Workspace keeps only the monorepo meaning, a folder with its own Manifest ([Manifest](manifest.md), *Workspace mapping*).

| Piece | What it is |
|---|---|
| Member | A whole repository, or any folder of one; a folder needs no `armada.yml` of its own |
| Place | Where the file is kept: This Mac, a synced folder or the cloud |
| Task | A card with a title and notes, written by him. It names its repository and, optionally, its project |
| Subtask | A task under a task, one level deep |
| Issue row | A tracker reference Fleet already lists as a row of a project. Bridge adds no new ones: an issue rides on a task |

> **Rule (17 Nov).** Delete, Merge and moving a project to another place take its file away wherever it was kept: Kit's folder, a synced folder or a bucket.
> Why: a file left behind is a stale project nobody asked to keep.

A bucket that refuses the delete, for a key with no delete right, does not stop the act.
The project is still deleted, merged or moved, and the answer to Delete or the move names the file left and gives the bucket's words.
A Merge's give-up runs later on the other Mac, so there the refusal goes to the log.
| Linked issue | A tracker reference on a task, in the order linked |
| Linked pull request | A pull request number on a task |
| Held Job | The one running Job that holds a task |

> **Rule.** A task written or moved into a project, in a repository that is not a member, makes that repository a whole member.
> Why: a project's members are the repositories its tasks name.

> **Rule.** A repository leaves a project only when he edits its members.
> Why: a task moving away or being deleted says nothing about whether the repository belongs.

> **Rule (13 Nov).** A Job, live or ended, can be moved into a project or out to no project, and its repository becomes a whole member as a task's does.
> Why: he makes a project after the work has begun, and wants the Jobs he already ran grouped under it.
> A Job of a repository the project does not list is still not that project's work, so the move adds the repository rather than refusing.

A moved Job keeps the values it froze when it started and the tasks it holds.
From the move on, what Fleet reads of the Job's project as it goes follows the new one: its tracker, the delivery setting, the lifecycles and rules scoped to it, and whether the Job is shared with his other Macs.
A Job started for a project still needs its repository to be a member when it starts.

**In the native Bridge the Jobs list moves Jobs into a project.**
Move to project in a Job row's menu, in the Job's own menu, in the palette and on `P` opens a picker of his projects, each with its repositories under its name, and No project last.
The project every Job being moved stands in is ticked.
Fleet's `set_session_project` writes it for live and ended Jobs alike, and the list regroups on the `session.changed` it publishes.
A Job another Mac holds is left off, with why.

**In the native Bridge the dispatch sheet starts a Job in a project.**
Its picker lists his projects by name, each with its repository on a line under the name, once for every repository the project holds.
Under No project come every repository and folder Fleet serves, and a Job started from one of those has no project, except that a start naming none is put in the one project that holds its repository, where exactly one does.
A window scoped to a project offers that project's rows alone, and a window on one repository in no project starts there.
The words of the request choose the first row of the repository they name, a project's before none.
Queue for later keeps the row's project with the idea, and the Job it starts is for that project, wherever the window is then.
⌘T over the sheet, and the card under it, start in the row picked, for its project.

> **Rule.** A project whose file cannot be read is shown as unread, with its members empty, and is not written over.
> Why: a write over a file Fleet could not read would lose what it held.

**A task in no project is loose.**
It names a repository and has no project, and is listed under "No project", first in a view or a repository.
It works as a task in a project does: the same cards, subtasks, ticks and Dispatch.

A task is `open`, `held`, `asked` or `done`.
A task has several issues and pull requests of its own, and at most one Job.
A subtask has no subtasks, and an issue row is neither parent nor subtask.

> **Rule.** A task is hidden once it is done, in every list, and one toggle in the pane's head shows it.
> Why: the lists are for what is still to do. A task Fleet asks about is not done, so it stays until he answers.

> **Rule.** Fleet keeps only a reference to an issue or a pull request, never its title or state.
> Why: the tracker owns them, so they are read on every read, held five minutes, and absent where the tracker cannot say.

The tracker is [Tracker](tracker.md), and a pull request is read from the forge.

**A project may name its own tracker.**
It replaces its members' trackers for the Jobs in it, for the project's issues, and for the issues its tasks and issue rows link, which are read and checked through it.
A loose task and a task in a project that names none keep their repository's ([Tracker](tracker.md), *A project's tracker*).

## The built-in project

**On a Mac where Fleet serves the armada repository (manifest id `armada`), Fleet makes a project named Armada by itself.**
Its only member is the armada repository, whole, its place is This Mac, and it sets one value, `delivery: auto_land`.
Auto merge and the tracker already come from the repository's committed `armada.yml`, and theme and layout stay each Mac's, so the project sets nothing else.
Fleet makes it when it first serves the repository, or at its start for a repository it already serves, and it never asks him.

> **Rule (13 Nov).** Fleet makes the project once per Mac and keeps a record that it did, so a project he deletes or renames is never made again.
> Where a project already has the armada repository as a member, Fleet makes nothing and records it as made.
> Why: he wants Armada ready to use on a new Mac, such as his work Mac, and a project he removed on purpose stays removed.

The record is one row of `project_imports`, under `built-in:armada`, the same table that keeps a committed Workspace from coming back after a delete.

## What a project scopes

A lifecycle, a rule or an autonomy piece names a project in its `scope:`.
A piece scoped to a project is read from Kit and from every member repository's `.armada/<kind>/`.
[Protocol](../practices/protocol.md), *Lifecycles and rules, read*, says how each is served.

## Its store

**A project's `store:` block names the store it joins and the GitHub login that joins it, and a repository of the project may override the login.**
A project with a store and no login sends nothing, and says so.
The rule is [Central store](../contracts/central-store.md), *A store per project*, and the login is [Account](account.md), *Which login a project joins as*.

## Sharing with his other Macs

**A project is shared with his other Macs only when he turns it on, and every project starts off.**
The switch is `shared_with_macs` in the project's own file, on the Mac that holds it, and is written only while on.

> **Rule.** Nothing of a Job leaves this Mac unless its project is shared, and a Job started for no project is never shared.
> Why: he keeps private projects on one Mac whose Jobs must never be seen from the other, and he called that absolutely critical.

The Mac that holds a Job decides, reading the Job's project and that project's switch at the moment of sending.
Everything bound for another Mac passes one filter in Fleet, and only the filter makes what a link may send.

| Thing | Crosses when |
|---|---|
| A Job's row, thread, live changes, documents, sketches and shown files | Its project is shared |
| An act from the other Mac on a Job (message, answer, approve, stop, merge) | Its project is shared. Otherwise it is refused as not found, the same answer as a Job that does not exist |
| A Job started from the other Mac | Only into a shared project, in one of its repositories, and never as a fork, in a folder or waiting on a Job of this Mac |
| A dispatch from the other Mac or the phone, which the Operator may hand to a live Job | Only to a Job in a shared project. One in an unshared project is not shown to the Operator's call, as if it did not exist |
| A message from a Job to a Job on the other Mac | Both Jobs' projects are shared |
| A bug report | The Job that filed it is in a shared project, since a report's text can quote private work |
| A project and its tasks | It is shared. Loose tasks never cross |

A stream event that is not about one Job or a shared project, such as servers, settings, mods or the Operator, stays on its Mac.

**A shared project is kept on the other Mac as a copy, with its tasks, and the Mac that holds it stays its authority.**
The copy is listed beside that Mac's own projects with the Mac it is from, and turning the switch off removes it, at once or when the link next opens.
A task edited, added, ticked, linked, moved within the project or removed on the copy is sent through the link to the holding Mac, which makes the change with its own code, and the copy is what that Mac answers.
So is a change to the project itself made on the copy: its name, members, values, a workflow switched off, its share switch, and its deletion.
An edit on the holding Mac reaches the copy the same way.
A read of the copy, of its values, issues, claims or features, is answered on this Mac from what the copy carries, over this Mac's own Kit, repositories and feature catalog.

> **Rule (19 Nov).** A change to a shared project made on either Mac reaches both, and the holding Mac makes it.
> Only the holding Mac chooses where the file is kept, and sets the claims store, and an edit on the copy that names a place is refused whole, `fleet.other_mac_project_held`.
> A Job linked to a task and a task's repository also stay the holding Mac's.
> A merge keeps this rule: each Mac changes only what it holds, and the other Mac's part crosses as an act.
> Why: he said "If I make a change to the project or anything in that project on one mac, it updates on the other"; the place is a folder on that Mac's disk, and a store is joined and deployed with that Mac's sign-in.

> **Rule (20 Nov).** A copy keeps the holder's store and login, and a repository on this Mac that the copy lists claims at that store with this Mac's own profile for that login.
> This Mac enrols that profile the first time it needs it, and writes nothing to the holder's file; a Mac not signed in as that login sends nothing, and the copy's store setting says so.
> Why: joining is per Mac, and a repository that is in a copy only claimed nothing on the Mac that held the copy.

> **Rule (19 Nov).** A Job of this Mac moves into a copy, or starts in one, where the copy lists the Job's repository, and is shared with the holding Mac from then; otherwise the move or the start is refused, `fleet.project_copy_not_member`, and the copy's members are not touched.
> Why: he chose one shared project across his Macs, with Jobs from either Mac in it, and a merge already moves Jobs into a copy this way.

> **Rule (13 Nov).** An edit made on a copy while the holding Mac is asleep shows on the copy at once and is sent when that Mac wakes, oldest first.
> Where both Macs changed the same field of one task during the sleep (title, notes, state or place), the later edit wins on both, and he is never asked.
> Why: he chose that edits are never refused and he is never asked; a clash needs two edits to one field of one task within one sleep.

### Merging two projects of one name

**Each Mac holds its own project of a name, so a shared project shows twice on the other Mac, once as its own and once as a copy.**
One Merge, pressed on either Mac, makes them one.
The request names the project to keep and the project that goes, and either may be this Mac's own or the copy.

> **Rule (17 Nov).** Merge needs the project to keep to be shared with his other Macs, and the Bridge offers the switch first.
> Why: a project that is not shared cannot take tasks from another Mac, and the other Mac could not list it afterwards.

- The Mac that holds the kept project takes the gone project's tasks, members and values.
  A task keeps its id and its state, except that a held task comes over open, because linking a Job on one Mac to a task on the other is not built.
  Members are joined.
  A key only the gone project sets is copied, a key only the kept project sets stays, and for a key both set to different values he picks one project's, key by key.
- The Mac that holds the gone project moves its Jobs into the kept project, keeping their frozen values.
  It sends any task added to the gone project after the kept Mac took its copy, and then deletes the project, its tasks and its file, wherever the file is kept, a bucket included.
- Each Mac moves only its own Jobs, so a Job started on the other Mac in the copy moves there.
- The Mac he pressed it on lists one project from the press, the kept one with both sets of tasks.
  Where the other Mac is asleep it is told when it wakes, in the order of the other acts waiting for it, and nothing waits on it here.

The merge ends with one project on each Mac: the kept one, its own on one Mac and a copy on the other.

> **Rule (19 Nov).** A Mac gives its project up only while it holds the copy of the kept one; otherwise it refuses, `fleet.merge_kept_not_held`, and changes nothing.
> Pressed on the Mac that keeps the project, the give-up is sent first, and a refusal leaves both projects as they were.
> Why: on 19 Nov a merge deleted the other Mac's project while that Mac had no copy of the kept one, so the project vanished there and its Jobs stopped being shared.

Sync never ticks a task: a copy is only what the holding Mac says, and a tick crosses only as his own.
The Operator's call is handed the open tasks of every project, the copies' among them, and only reads them.
The phone passes the same filter: it sees a Job in a shared project, or one held from another Mac, and nothing of any other (`docs/concepts/pocket.md`, *What the phone sees*).

## What a project sets

**Behaviour is a project's to set, key by key over Kit.**
How a repository runs (its commands, Checks, ports and setup) stays the repository's `armada.yml`.
How work behaves is a value: pull request mode, auto merge, autonomy, the review model, reviews, how a Job's work lands (`delivery`) and the look (layout, theme, density).
The look also holds the project's Dashboard board, where each block stands, kept in the project's own file and by no other layer.

| Layer, furthest first | Says |
|---|---|
| Default | What Armada ships |
| Machine | This installation's settings |
| Kit | `values.yml` in Kit's folder, for every project and repository |
| Repository | `pr_mode:` and `auto_merge:` in its `armada.yml`, only where it writes them |
| Project | The `values:` block of its file |
| Job | What the Job itself was started with |

**The nearest layer that sets a key wins, and every answer names the layer it came from**, so a person can see whose word a value is.
A project sets only the keys it sets, so what it leaves out is Kit's, then the Machine's.
Autonomy keeps the places it already has (a piece scoped to every repository, to a project or to a repository), and the answer names them Kit, Project and repository.

> **Rule.** A project's value is over its repository's, and the repository's own keys sit just under it.
> Why: the owner chose this explicitly. A project's auto merge replaces its repository's, and nothing is folded between them.

> **Rule.** A Job takes its project's values when it starts and keeps them, and a Job moved into another project keeps them too.
> Why: a Job in the middle of its work does not change what it was told to do because a layer moved. A Job started for no project runs on its repository's values.

**Reviews are set a kind at a time: plans, pull requests and walks.**
Each is `off`, `review` or `review_and_approve`, and a walk takes only `off` or `review`, since a walk is his to look at.
A layer sets only the kinds it names, and a kind it leaves out falls through, kind by kind, from the project to Kit to `off`.
A project can set plans and leave pull requests and walks to Kit, and clearing one kind gives it back to Kit's word or the default.
Nothing is reviewed until a layer says so.
[The Operator](operator.md), *Reviewing work before you see it*, says what each does.

The Machine's own word stays live: a Job reads the Machine's pull request mode and review model as they stand, since a person moves them while a Job runs.
The wire is [Projects](../practices/protocol/projects.md), *Values*.

> **Rule (13 Nov).** A Job another Job starts is for its starter's project and takes that project's values, and its repository becomes a whole member where it was not.
> Why: a Job an agent started was in no project, so how its work lands was unset and he was asked every time.
> A starter in no project starts its Job in the one project that holds its repository, where exactly one does, and otherwise in none.
> A starter in a copy of another Mac's project starts its Job in the copy, and a repository the copy does not list is sent to the holding Mac as a member to add; where that Mac refuses, the Job starts in no project.
> A fork or a handoff keeps its source Job's project, and a move into another repository adds that repository as a whole member.
> Why: a Job that drops out of its project looks lost in a list grouped by project.

## Dispatching tasks

**Selecting cards and pressing Dispatch opens the Dispatch sheet, prefilled with them.**
A ⌘-click selects a card and a ⇧-click selects the range from the card last clicked, in any repository.
Dispatch is drawn only once cards are selected.
A Job runs in one repository, so cards from several open one request for each, the next once the one before has started or been put away.
The Dispatch icon on a task's card opens the sheet with that task and its subtasks alone.
A subtask's card has the same Dispatch icon.
It opens the sheet with the subtask as the work: its title on the first line, then `Part of:` and the parent's title, the parent's notes and the parent's issues as tags.
The sheet's words say that only the subtask is to be done, and the Job holds the subtask only.
Cards selected with a subtask whose parent is not selected are sent the same way for that subtask.
Starting sends the rows with the Job start, and the Job holds those tasks and their subtasks.
An issue row among them is only named, so the Job's brief can carry it.

**Linking a running Job to a task does the same.**
The Job must be live and of the task's repository, and a task holds one Job at a time.
Unlinking lets a `held` task go back to `open`.

## Asking whether a task is done

| What happens | The Job's tasks |
|---|---|
| The Job says it is done | `asked` |
| The Job's pull request merges | `asked` |
| A pull request linked to the task merges | `asked` |
| The Job ends having done neither | `open`, no longer held |

> **Rule.** A task is never marked done by Fleet; it is `asked`, and he answers Mark done or Keep open.
> Why: the owner chose to be asked rather than have tasks tick themselves.

**While the holding Job lives, the question is a `task_done` item in its Needs you list.**
[Waiting on the person](session/waiting-on-the-person.md) says how it is answered.
After the Job ends, the question stays on the cards, and he answers it there.
A task no Job holds is asked the same way when a linked pull request merges, on the cards alone.

## The Project place and its Dashboard

**In the native Bridge the place that was Features is Project, and it opens on the project's Dashboard.**
Its tabs are Dashboard, Features, Since you looked, Who's on what and Settings, and Dashboard is the one it opens on.
The three in the middle are the [Feature catalog](feature-catalog.md)'s screens, drawn in a mode each over one library.
Settings is the project's sheet, members and values, drawn as a page.

**The Dashboard is a board of blocks on a grid six columns wide, as tall as its blocks.**
No two blocks overlap and none has room above it: a move or a resize pins the block it is about, and the others rise to the first row where they fit.

| Block | What it draws |
|---|---|
| Needs you | The Jobs of the project waiting on him, then the Operator's cards no listed Job stands for |
| Run | The project's servers and runs, started and stopped on the checkout the header names |
| Reviews | The pull requests requested of him, assigned to him and his own, and under them the open pull requests of others he might be interested in |
| Issues | The issues assigned to him, and under them the ones he might pick up |
| Features | What moved since he looked, and in its Picture the rooms of the code map. A feature opens on the Features tab |
| Checks and commands | The Checks and Commands of its manifests, run in the main checkout |
| System | What the project is made of: the people who use it, its entry points, what they reach, its stores and what it reaches outside. A part zooms into its pieces, and a feature into its path through them |

The might-pick-up and might-be-interested rows are Fleet's ranking, which [Feature catalog](feature-catalog.md), *Ranking the issues he might pick up*, owns.

**Each block has a view switch, and it offers only the views the block's data fits.**
Needs you is a List or Cards, Run a List, Table or Cards, Reviews Cards, List or Table, Issues a Table, List or Cards, Features Cards, List or Picture, Checks and commands a List or Table, and System a Graph or List.
The first of a block's views is the one it starts in.

**Edit turns the blocks into things to arrange.**
While it is on, dragging a block's header moves it and dragging its corner resizes it, and the arrow keys move the block that has the keys by a cell and Shift with the arrows grows or shrinks it.
A block is least two columns wide and one row tall, and most six rows tall.
Edit also offers Hide on each block, and Add block lists the kinds that are not on the board, so a hidden block is never lost.
Add block puts a kind in the first free cells.

### Where a board starts, and where it is kept

> **Rule.** A project that has saved no board draws the preset for its kind, and one he has arranged keeps what he arranged.
> Why: the preset is a guess from the project's manifests, and a guess must not move a block he placed.

The kind is read from the manifest kinds the code map already names, so no second reading is made.

| Project | Manifest kinds it holds | The preset puts first |
|---|---|---|
| Web | `npm` | Run, then Needs you |
| Compiled | `cargo`, `go`, `gradle`, `maven`, `dotnet` or `swift`, and no `npm` | Checks and commands, then Needs you |
| Unknown | None of those, or the map not read yet | The web board |

Every preset puts System first, the full width of the board, and the rest under it.

The layout is kept in the project's own file as `look.board`, and no other layer sets it ([Projects](../practices/protocol/projects.md), *The board*).
A drag or a run of arrow keys is one save, sent once the board has stood still a moment.
Reset board, on the Settings tab, has Fleet forget the layout, and the board draws the preset again.

A block's kind and view are plain text on the wire.
A kind a later Bridge adds crosses without a wire break, and a block of a kind this build does not draw is kept and sent back as it was saved.

### The System block

**Fleet reads a project's parts from its own files, with no model.**
Where the project picked folders of a repository, it reads only those folders, so a part, a feature or an owner outside them is not shown.
Build manifests, workspaces, compose files and Kit servers name the entry points people use, the services behind them, the stores and what the project reaches outside.
A library every part uses folds into the parts that use it, and shows as a piece when a part is zoomed into.
A package's description names its part, as `Bridge Classic: the Electron desktop application` names Bridge Classic.

**A click selects a part, and double-click, Return or Look inside zooms into it.**
Level one is the whole project, level two a part's pieces and the parts they call, and level three one feature's numbered path through them with its files at each stop.
Esc steps out a level, and at level one gives the board back.
The zoomed block covers the board.

**Every part, every feature and the project itself answer the same nine sections**: features, code, who is working on it, owner, what it looks like, docs, how to watch it, open issues and open pull requests.
A section with nothing in it says why in one line.
New feature… under Features takes a sentence and starts a Job.

**Find missed parts has the librarian read the project for parts its files do not show.**
Its parts draw dashed with a chip until he keeps them, and Work with the librarian takes a note it redoes the part from.
A part he removes is not proposed again ([Projects](../practices/protocol/projects.md), *The system map*).

### The header

The header is the same on every tab.

| Part | What it does |
|---|---|
| Project chips | A press shows that project |
| Ask the librarian | A box that keeps what is typed. While it has the keys it offers example asks, and a press on one fills the box. Sending is not built |
| Edit | Turns editing the board on and off, and with it Add block, on the Dashboard tab |
| Members and Settings | Open the Settings tab. Members puts the keys in the list of members |
| Start | Starts the project's run on the checkout beside it |

The checkout menu beside Start lists Main and the worktree of each live Job of the project on this Mac, so a run can be started on work not yet landed.
Run and Start both act on the checkout it names.

**A Command marked `confirm: true` asks before the board runs it.**
Run and Cancel are asked over the board, the keys are on Cancel, and nothing runs until Run.
Any other Command runs on a press ([Manifest](manifest.md), *Commands*).

**The Dashboard is the native Bridge's alone.**
Classic is unchanged.

## What a person sees

| Surface | What it draws |
|---|---|
| The Projects tab | Sits beside Issues, and takes the next digit after it. A left column of three views (all tasks, the ones Fleet asks about, the ones a Job holds), his projects and the repositories; then the cards of what is open with their subtasks. A view or a repository groups its cards by project, "No project" first; a project lists its cards in his order |
| A card | A neutral tile: its title, notes, linked issues and pull requests with their state from the tracker, and the Job holding it. State is the colour of its glyph. Its repository is a quiet chip where the list holds more than one. Its acts are Dispatch, add a subtask and delete; a subtask has Dispatch and delete |
| An asked card | Mark done and Keep open |
| The add fields | Take `#` for an issue or a pull request and `@` for a Job or a file, as the composer does. They are how a task is linked. The key that goes to the task field sits inside it, at its right edge, and in the composer at the right of its tool row. The task field grows into a composer with a button for each, and a file stays in the words as `@path`. The composer also names the repository, required, and the project, optional; a pane fixes the one it is open on |
| The search | A field above the views, focused by `/`. Words in it list every task and subtask whose title, notes or linked issue reference or title match, in all projects and repositories, grouped by project as a view is. Done tasks are listed only while they are shown. Esc clears it and the view that was open is back; while it is on, the column marks nothing as open |
| The command palette | Words typed in it can be added as a task or a project. It also finds any task or subtask by title, with its project (or No project) and repository under it, and choosing one opens its project, or All tasks, with done shown where it is done, and its card in focus |
| The Project place | Opens on the Dashboard, a snapping grid of blocks, over the tabs Features, Since you looked, Who's on what and Settings. [The Project place and its Dashboard](#the-project-place-and-its-dashboard) |
| A block | A card with a header, a view switch for the views its data fits, and its rows. While the board is edited the header drags it, its corner resizes it and a Hide button takes it off the board |
| The header | The project chips, the Ask the librarian box, Edit, Members, Settings, the checkout menu and Start |

The palette's rows and the keys on the tab are in [Keyboard and command palette](../contracts/design-system/keyboard-and-command-palette.md).

## What it does not do yet

| Missing | Waiting on it |
|---|---|
| A project's autonomy set from its own file | Autonomy stays in its pieces, which a project is already a scope of; nothing here writes one |
| A Job's own values | Nothing starts a Job with a value of its own, so the Job layer is empty |
| The title-bar project switcher in the native Bridge | The native Bridge, built by another Job |
| The switch in the native Bridge | The native Bridge |
| A Job on one Mac linked to a task on the other | Acts across Macs, a later stage of two Macs as one Armada |
| Sharing a project with teammates | Not built. A project's file is on this Mac, in a synced folder or in the cloud its owner chose |
| The System, Docs and Errors blocks | Each is a later Job. Block kinds are plain text, so they add without a wire break |
| Sending the Ask the librarian box | A later Job. The box keeps what is typed and offers examples, and sends nothing |
| The librarian choosing the preset | A later Job. The preset follows the manifest kinds alone |
