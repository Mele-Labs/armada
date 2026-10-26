# Project

**What it is:** A list of tasks and linked issues that one person keeps for one repository, in Bridge's Projects tab.
It tracks his own portfolio of work, which is slightly different from the issues, and issues link to it.
A task can link issues, pull requests and a running Job, and Fleet asks him before it marks one done.

---

**Kind:** Store.
A project is kept in Fleet's store on this Mac, per repository, and survives a restart.
It is never written to the repository or its Manifest.
The wire is [Projects: tasks and linked issues, per repository](../practices/protocol/projects.md), and this page does not repeat it.

## The model

**A project belongs to one repository, and is an ordered list of rows.**
A row is a task he wrote or an issue he linked.

| Piece | What it is |
|---|---|
| Task | A card with a title and notes, written by him |
| Subtask | A task under a task, one level deep |
| Issue row | A tracker reference he linked to the project |
| Linked issue | A tracker reference on a task, in the order linked |
| Linked pull request | A pull request number on a task |
| Held Job | The one running Job that holds a task |

A task is `open`, `held`, `asked` or `done`.
A task has several issues and pull requests of its own, and at most one Job.
A subtask has no subtasks, and an issue row is neither parent nor subtask.

> **Rule.** Fleet keeps only a reference to an issue or a pull request, never its title or state.
> Why: the tracker owns them, so they are read on every read, held five minutes, and absent where the tracker cannot say.

The tracker is [Tracker](tracker.md), and a pull request is read from the forge.

## Dispatching tasks

**Selecting cards and pressing Dispatch opens the Dispatch sheet, prefilled with them.**
Starting sends the rows with the Job start, and the Job holds those tasks and their subtasks.
An issue row among them is only named, so the Job's brief can carry it.

**Linking a running Job to a task does the same.**
The Job must be live and of the same repository, and a task holds one Job at a time.
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
| The Projects tab | Sits beside Issues, and takes the next digit after it. The projects, then the cards of one with their subtasks |
| A card | Its title, notes, linked issues and pull requests with their state from the tracker, and the Job holding it |
| An asked card | Mark done and Keep open |
| The add fields | Take `#` for an issue or a pull request and `@` for a Job, as the composer does |
| The command palette | Words typed in it can be added as a task or a project |

The palette's rows and the keys on the tab are in [Keyboard and command palette](../contracts/design-system/keyboard-and-command-palette.md).

## What it does not do yet

| Missing | Waiting on it |
|---|---|
| Sharing a project with teammates | Not built. Projects are in this Mac's store only |
