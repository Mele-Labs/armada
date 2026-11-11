# Project

**What it is:** The set of repositories one person works across as one body of work, with the tasks he keeps for it.
A member is a whole repository or a folder inside one that carries its own `armada.yml`.
A Job is started for a project, lifecycles and rules are scoped to one, and a task can sit in one.
A task can link issues, pull requests and a running Job, and Fleet asks him before it marks one done.

---

**Kind:** A file at a place he chooses, and tasks in a store.
The file is `projects/<id>.yml` in Kit's folder, or the same file in a synced folder or the cloud, chosen per project and This Mac by default.
It names the project, lists its members and holds the values the project sets, and it is never kept in a repository.
The tasks are in Fleet's store on this Mac and survive a restart, wherever the file is.
The wire is [Projects: tasks and linked issues, across repositories](../practices/protocol/projects.md), and this page does not repeat it.

## The model

**A project is his, not a repository's, and a repository may be in several.**
Its members are the repositories its tasks name and the ones he adds.
The word Workspace keeps only the monorepo meaning, a folder with its own Manifest ([Manifest](manifest.md), *Workspace mapping*).

| Piece | What it is |
|---|---|
| Member | A whole repository, or a folder of one that has its own `armada.yml` |
| Place | Where the file is kept: This Mac, a synced folder or the cloud |
| Task | A card with a title and notes, written by him. It names its repository and, optionally, its project |
| Subtask | A task under a task, one level deep |
| Issue row | A tracker reference Fleet already lists as a row of a project. Bridge adds no new ones: an issue rides on a task |
| Linked issue | A tracker reference on a task, in the order linked |
| Linked pull request | A pull request number on a task |
| Held Job | The one running Job that holds a task |

> **Rule.** A task written or moved into a project, in a repository that is not a member, makes that repository a whole member.
> Why: a project's members are the repositories its tasks name.

> **Rule.** A repository leaves a project only when he edits its members.
> Why: a task moving away or being deleted says nothing about whether the repository belongs.

> **Rule.** A Job keeps the project it was started from, and the Job's repository must be a member.
> Why: a Job of a repository the project does not list is not that project's work.

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

## What a project scopes

A lifecycle, a rule or an autonomy piece names a project in its `scope:`.
A piece scoped to a project is read from Kit and from every member repository's `.armada/<kind>/`.
[Protocol](../practices/protocol.md), *Lifecycles and rules, read*, says how each is served.

## What a project sets

**Behaviour is a project's to set, key by key over Kit.**
How a repository runs (its commands, Checks, ports and setup) stays the repository's `armada.yml`.
How work behaves is a value: pull request mode, auto merge, autonomy, the review model and the look (layout, theme, density).

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

> **Rule.** A Job takes its project's values when it starts and keeps them.
> Why: a Job in the middle of its work does not change what it was told to do because a layer moved. A Job started for no project runs on its repository's values.

The Machine's own word stays live: a Job reads the Machine's pull request mode and review model as they stand, since a person moves them while a Job runs.
The wire is [Projects](../practices/protocol/projects.md), *Values*.

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

## What a person sees

| Surface | What it draws |
|---|---|
| The Projects tab | Sits beside Issues, and takes the next digit after it. A left column of three views (all tasks, the ones Fleet asks about, the ones a Job holds), his projects and the repositories; then the cards of what is open with their subtasks. A view or a repository groups its cards by project, "No project" first; a project lists its cards in his order |
| A card | A neutral tile: its title, notes, linked issues and pull requests with their state from the tracker, and the Job holding it. State is the colour of its glyph. Its repository is a quiet chip where the list holds more than one. Its acts are Dispatch, add a subtask and delete; a subtask has Dispatch and delete |
| An asked card | Mark done and Keep open |
| The add fields | Take `#` for an issue or a pull request and `@` for a Job or a file, as the composer does. They are how a task is linked. The key that goes to the task field sits inside it, at its right edge, and in the composer at the right of its tool row. The task field grows into a composer with a button for each, and a file stays in the words as `@path`. The composer also names the repository, required, and the project, optional; a pane fixes the one it is open on |
| The search | A field above the views, focused by `/`. Words in it list every task and subtask whose title, notes or linked issue reference or title match, in all projects and repositories, grouped by project as a view is. Done tasks are listed only while they are shown. Esc clears it and the view that was open is back; while it is on, the column marks nothing as open |
| The command palette | Words typed in it can be added as a task or a project. It also finds any task or subtask by title, with its project (or No project) and repository under it, and choosing one opens its project, or All tasks, with done shown where it is done, and its card in focus |

The palette's rows and the keys on the tab are in [Keyboard and command palette](../contracts/design-system/keyboard-and-command-palette.md).

## What it does not do yet

| Missing | Waiting on it |
|---|---|
| A project's autonomy set from its own file | Autonomy stays in its pieces, which a project is already a scope of; nothing here writes one |
| A Job's own values | Nothing starts a Job with a value of its own, so the Job layer is empty |
| The title-bar project switcher in the native Bridge | The native Bridge, built by another Job |
| Sharing a project with teammates | Not built. A project's file is on this Mac, in a synced folder or in the cloud its owner chose |
