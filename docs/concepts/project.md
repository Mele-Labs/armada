# Project

**What it is:** A list of tasks that one person keeps across his repositories, in Bridge's Projects tab.
It tracks his own portfolio of work, which is slightly different from the issues, and issues ride on its tasks.
A task can link issues, pull requests and a running Job, and Fleet asks him before it marks one done.

---

**Kind:** Store.
A project is kept in Fleet's store on this Mac and survives a restart.
It is never written to the repository or its Manifest.
The wire is [Projects: tasks and linked issues, per repository](../practices/protocol/projects.md), and this page does not repeat it.

## The model

**A project is his, not a repository's, and is an ordered list of rows that may stand in several repositories.**
A row is a task he wrote, and each row names the repository it is in.
A task in no project is just a task: it names a repository and has no project.

| Piece | What it is |
|---|---|
| Task | A card with a title and notes, written by him. It names its repository and, optionally, its project |
| Subtask | A task under a task, one level deep |
| Issue row | A tracker reference Fleet already lists as a row of a project. Bridge adds no new ones: an issue rides on a task |
| Linked issue | A tracker reference on a task, in the order linked |
| Linked pull request | A pull request number on a task |
| Held Job | The one running Job that holds a task |

A task in no project is listed under "No project", first in a view or a repository.
It works as a task in a project does: the same cards, subtasks, ticks and Dispatch.

A task is `open`, `held`, `asked` or `done`.
A task has several issues and pull requests of its own, and at most one Job.
A subtask has no subtasks, and an issue row is neither parent nor subtask.

> **Rule.** A task is hidden once it is done, in every list, and one toggle in the pane's head shows it.
> Why: the lists are for what is still to do. A task Fleet asks about is not done, so it stays until he answers.

> **Rule.** Fleet keeps only a reference to an issue or a pull request, never its title or state.
> Why: the tracker owns them, so they are read on every read, held five minutes, and absent where the tracker cannot say.

The tracker is [Tracker](tracker.md), and a pull request is read from the forge.

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
| Sharing a project with teammates | Not built. Projects are in this Mac's store only |
| A project across repositories, and tasks in no project, in Fleet | Not built. Fleet keeps each project in one repository. Bridge reads every row of a project as that repository's, reads the project with the id `own:<manifest id>` as tasks in no project, and the mock Fleet serves a project in several repositories. The Fleet side is a later step |
