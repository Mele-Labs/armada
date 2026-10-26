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
| `ProjectRow` | Fleet writes, Bridge reads | Tagged by `kind`: `task { id, title, notes, state, held_by?, parent?, issues? }`, `state` `open`, `done`, `held` or `asked`, `held_by { session_id, title? }` present for the last two, `parent` the id of the task a subtask belongs to, `issues` `[{ issue, title?, state? }]` the issues linked to it; or `issue { id, issue, title?, state? }`, `state` `open` or `closed` |
| `start_session` | Bridge calls | `project_rows?: [{ project, row }]` on `StartSession`: the rows the Job starts with. Each written task among them is held by the Job; an issue row is only named, so the brief can carry it |
| `add_project` | Bridge calls | `POST /projects/:repository`, body `NameProject { name }` |
| `rename_project` | Bridge calls | `POST /projects/:repository/:project/rename`, body `NameProject { name }` |
| `delete_project` | Bridge calls | `POST /projects/:repository/:project/delete`, no body. Its rows go with it |
| `add_project_task` | Bridge calls | `POST /projects/:repository/:project/tasks`, body `ProjectTask { title, notes? }`. Starts `open` |
| `add_project_subtask` | Bridge calls | `POST /projects/:repository/:project/rows/:row/subtasks`, body `ProjectTask`: a task under the top-level task `row`, at the end of its subtasks |
| `link_task_issue` | Bridge calls | `POST /projects/:repository/:project/rows/:row/issues`, body `LinkProjectIssue`: an issue on a task or subtask. Linking it again changes nothing |
| `unlink_task_issue` | Bridge calls | `POST /projects/:repository/:project/rows/:row/issues/unlink`, body `LinkProjectIssue`. One the task did not have changes nothing |
| `edit_project_task` | Bridge calls | `POST /projects/:repository/:project/rows/:row/edit`, body `ProjectTask`, whole |
| `link_project_issue` | Bridge calls | `POST /projects/:repository/:project/issues`, body `LinkProjectIssue { issue }`, a reference as `IssueReference` reads it |
| `remove_project_row` | Bridge calls | `POST /projects/:repository/:project/rows/:row/remove`, no body |
| `move_project_row` | Bridge calls | `POST /projects/:repository/:project/rows/:row/move`, body `MoveProjectRow { position, parent? }`, `position` from 0 among the rows of the parent the row ends up under; past the end is the end. `parent` is the task to go under, `""` for the top level, absent to keep the one it has |
| `tick_project_task` | Bridge calls | `POST /projects/:repository/:project/rows/:row/tick`, body `TickProjectTask { done }`. On an `asked` row it answers the ask: done, or open again |
| `WaitingSource::task_done` | Fleet writes, Bridge reads | A Job's own waiting item while it is live and holds `asked` tasks: id `task_done:<project>`, an `answer` act, options `Mark done` (choice 0) and `Keep open` (choice 1), through `answer_waiting` |
| `projects.changed` | Fleet publishes | `Projects`, whole, on every write above |

Every write answers `Projects` whole, as `list_projects` does, and publishes the same value.

**An issue row keeps no state.** Only the tracker's reference is kept. Its `state` and `title` are
read when the rows are: from the open list Fleet holds for the Issues tab, and an issue not on it
read by its reference. Each is held five minutes. Where the tracker cannot say, both are absent and
the list is still answered. The tracker is not asked whether a linked issue exists.

**A task may have subtasks, one level deep.** A subtask is a task whose `parent` is the id of a top-level
task of the same project. Rows are flat: a parent is followed by its subtasks, so a client indents a row
that has a `parent`. Positions count among the rows of one parent, and `move_project_row` may change the
parent (`""` is the top level). Removing a parent removes its subtasks. A subtask has no subtasks, a task
that has subtasks cannot become one, and an issue row is neither parent nor subtask. Holding and the done?
ask treat a subtask as any task, and a start that names a parent holds its subtasks too.

**An issue linked to a task keeps no state either.** A task, or a subtask, carries the references of the
issues linked to it, in the order linked. `title` and `state` come from the tracker as an issue row's
do, held five minutes, and are absent where it cannot say. Linking the same reference twice is one link.

**A Job holds the tasks it started with.** A start that names project rows moves each written task to
`held`, with the Job's id and title on the row, and publishes `projects.changed`. A task another Job held
is taken over. Only a written task has this state: a linked issue row keeps none, and an issue row named
in a start is accepted and left as it is.

| The Job | Its held tasks |
| --- | --- |
| Opens a pull request that merges | `asked` |
| Says it is done (`all_done`) | `asked` |
| Ends having done neither | `open`, no longer held |
| Ends after either | stay `asked` |

**The ask is held on the rows, not on the Job.** While the holding Job is live, Fleet adds a
`task_done` item to its waiting list, one per project: id `task_done:<project>`, text `Mark <title>
done?` for one task or `Mark the <project> tasks done?` for several, an `answer` act, and the options
`Mark done` and `Keep open`. Answering through `answer_waiting` settles every task the Job asked in that
project: `Mark done` makes them `done`, `Keep open` makes them `open`, and neither sends the agent a
message. The item cannot be dismissed. The ask stands on the rows when the Job ends, and the tick route
settles one row of it from the Projects tab: done, or open again. The item leaves with the last task asked.

**Errors.**
404 `fleet.no_such_project` is a project the repository has none of.
404 `fleet.no_such_project_row` is a row the project has none of.
422 `fleet.project_blank` is a blank project name or task title.
422 `fleet.project_issue_unreadable` is a link `IssueReference` refuses.
422 `fleet.project_row_not_a_task` is an edit, a tick, a link or a subtask on an issue row, or an issue row moved under a task.
422 `fleet.project_subtask_too_deep` is a subtask of a subtask, of a task that has subtasks, or of itself.
`start_session` names rows: 404 `fleet.no_such_project` and `fleet.no_such_project_row` as above, and 422
`fleet.project_row_other_repository` is a project of another repository. Nothing is made on a refusal.
422 for a repository Fleet does not serve, as every repository route answers it.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** `projects.changed` is published on a person's write and when a Job holds, asks about or lets go of tasks, so it is rare.
The lifeboat is untouched.
