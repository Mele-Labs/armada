# Projects: tasks and linked issues, per repository

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

A person keeps projects per repository, each an ordered list of rows. A row is a task he wrote or
an issue he linked. They are kept in Fleet's store on this Mac, survive a restart, and are never
written to the repository or its Manifest.
The protocol ID moves with the files below, and there is no number to take.
Bridge's half is `packages/protocol/src/projects.ts`, written by hand, field for field.

| Where | Side | Change |
| --- | --- | --- |
| `list_projects` | Bridge calls | `GET /projects/:repository`, answers `Projects { manifest_id, projects }`, each `Project { id, name, rows }` in order |
| `ProjectRow` | Fleet writes, Bridge reads | Tagged by `kind`: `task { id, title, notes, state }`, `state` `open` or `done`; or `issue { id, issue, title?, state? }`, `state` `open` or `closed` |
| `add_project` | Bridge calls | `POST /projects/:repository`, body `NameProject { name }` |
| `rename_project` | Bridge calls | `POST /projects/:repository/:project/rename`, body `NameProject { name }` |
| `delete_project` | Bridge calls | `POST /projects/:repository/:project/delete`, no body. Its rows go with it |
| `add_project_task` | Bridge calls | `POST /projects/:repository/:project/tasks`, body `ProjectTask { title, notes? }`. Starts `open` |
| `edit_project_task` | Bridge calls | `POST /projects/:repository/:project/rows/:row/edit`, body `ProjectTask`, whole |
| `link_project_issue` | Bridge calls | `POST /projects/:repository/:project/issues`, body `LinkProjectIssue { issue }`, a reference as `IssueReference` reads it |
| `remove_project_row` | Bridge calls | `POST /projects/:repository/:project/rows/:row/remove`, no body |
| `move_project_row` | Bridge calls | `POST /projects/:repository/:project/rows/:row/move`, body `MoveProjectRow { position }`, from 0; past the end is the end |
| `tick_project_task` | Bridge calls | `POST /projects/:repository/:project/rows/:row/tick`, body `TickProjectTask { done }` |
| `projects.changed` | Fleet publishes | `Projects`, whole, on every write above |

Every write answers `Projects` whole, as `list_projects` does, and publishes the same value.

**An issue row keeps no state.** Only the tracker's reference is kept. Its `state` and `title` are
read when the rows are: from the open list Fleet holds for the Issues tab, and an issue not on it
read by its reference. Each is held five minutes. Where the tracker cannot say, both are absent and
the list is still answered. The tracker is not asked whether a linked issue exists.

**A task's state has room.** `open` and `done` today. A later change adds a Job holding a task and
the done? ask, so the store column is unchecked text and a client treats a state it does not know
as open.

**Errors.**
404 `fleet.no_such_project` is a project the repository has none of.
404 `fleet.no_such_project_row` is a row the project has none of.
422 `fleet.project_blank` is a blank project name or task title.
422 `fleet.project_issue_unreadable` is a link `IssueReference` refuses.
422 `fleet.project_row_not_a_task` is an edit or a tick on an issue row.
422 for a repository Fleet does not serve, as every repository route answers it.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** `projects.changed` is published only on a person's write, so it is rare.
The lifeboat is untouched.
