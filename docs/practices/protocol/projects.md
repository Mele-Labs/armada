# Projects: tasks and linked issues, across repositories

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

A person keeps projects on this Mac, each an ordered list of rows, and **a project has no repository**. A row is a task he
wrote or an issue he linked, and each row names the repository it is in, so one project may hold rows of several. A task may
be in no project, and is then loose. They are kept in Fleet's store on this Mac, survive a restart, and are never written to
the repository or its Manifest.
The protocol ID moves with the files below, and there is no number to take.
Bridge's half is `packages/protocol/src/projects.ts`, written by hand, field for field.

| Where | Side | Change |
| --- | --- | --- |
| `list_projects` | Bridge calls | `GET /projects`, answers `Projects { projects, loose }`: each `Project { id, name, rows }` in his order, and the loose tasks, of every repository, in their own order |
| `ProjectRow` | Fleet writes, Bridge reads | Tagged by `kind`: `task { id, manifest_id, title, notes, state, held_by?, parent?, issues?, pulls? }`, `state` `open`, `done`, `held` or `asked`, `held_by { session_id, title? }` present for `held` and for an `asked` a Job raised, `parent` the id of the task a subtask belongs to, `issues` `[{ issue, title?, state? }]` the issues linked to it, `pulls` `[{ number, title?, state? }]` the pull requests linked to it, `state` `open`, `merged` or `closed`; or `issue { id, manifest_id, issue, title?, state? }`, `state` `open` or `closed`. `manifest_id` is the repository of the row, and a subtask takes its parent's |
| `start_session` | Bridge calls | `project_rows?: [{ row }]` on `StartSession`: the rows the Job starts with, each named by its id alone. Each written task among them is held by the Job; an issue row is only named, so the brief can carry it |
| `add_project` | Bridge calls | `POST /projects`, body `NameProject { name }`. A project is last in his order |
| `rename_project` | Bridge calls | `POST /projects/project/:project/rename`, body `NameProject { name }` |
| `delete_project` | Bridge calls | `POST /projects/project/:project/delete`, no body. Its rows go with it |
| `add_project_task` | Bridge calls | `POST /projects/tasks`, body `NewProjectTask { manifest_id, project?, title, notes? }`. Starts `open`, at the end of the project or, with no `project`, of the loose tasks |
| `add_project_subtask` | Bridge calls | `POST /projects/rows/:row/subtasks`, body `ProjectTask { title, notes? }`: a task under the top-level task `row`, at the end of its subtasks, in its repository |
| `link_task_issue` | Bridge calls | `POST /projects/rows/:row/issues`, body `LinkProjectIssue`: an issue on a task or subtask. Linking it again changes nothing |
| `unlink_task_issue` | Bridge calls | `POST /projects/rows/:row/issues/unlink`, body `LinkProjectIssue`. One the task did not have changes nothing |
| `link_task_job` | Bridge calls | `POST /projects/rows/:row/job`, body `LinkTaskJob { session_id }`: a running Job holds the task and its subtasks, as if started with them. A Job already holding it is replaced |
| `unlink_task_job` | Bridge calls | `POST /projects/rows/:row/job/unlink`, no body. A `held` task, and the subtasks the same Job held, are `open` again; any other state is left |
| `link_task_pull` | Bridge calls | `POST /projects/rows/:row/pulls`, body `LinkTaskPull { number }`: a pull request on a task or subtask. A task may carry several. Linking it again changes nothing |
| `unlink_task_pull` | Bridge calls | `POST /projects/rows/:row/pulls/unlink`, body `LinkTaskPull`. One the task did not have changes nothing |
| `edit_project_task` | Bridge calls | `POST /projects/rows/:row/edit`, body `ProjectTask`, whole |
| `link_project_issue` | Bridge calls | `POST /projects/issues`, body `NewProjectIssue { manifest_id, project?, issue }`, a reference as `IssueReference` reads it, a row of the project or, with no `project`, a loose one |
| `remove_project_row` | Bridge calls | `POST /projects/rows/:row/remove`, no body |
| `move_project_row` | Bridge calls | `POST /projects/rows/:row/move`, body `MoveProjectRow { project?, position, parent? }`, `position` from 0 among the rows of the parent the row ends up under; past the end is the end. `project` is the project to go into, `""` for none, absent to stay where it is. `parent` is the task to go under, `""` for the top level, absent to keep the one it has, or the top level where the project changes. The row keeps its repository, and its subtasks go with it |
| `tick_project_task` | Bridge calls | `POST /projects/rows/:row/tick`, body `TickProjectTask { done }`. On an `asked` row it answers the ask: done, or open again |
| `WaitingSource::task_done` | Fleet writes, Bridge reads | A Job's own waiting item while it is live and holds `asked` tasks: id `task_done:<project>`, with `loose` for the tasks in no project, an `answer` act, options `Mark done` (choice 0) and `Keep open` (choice 1), through `answer_waiting` |
| `projects.changed` | Fleet publishes | `Projects`, whole, on every write above |

**A row's id is the whole address.** Ids are unique across every project and the loose tasks, so a route that acts on a row
names no project.

**What moved from the per-repository wire.** A project once belonged to one repository and every route named it
(`/projects/:repository/...`). Moving to this shape, Fleet's store gave each row its project's repository, made the rows of every
`own:<repository>` project loose rows of that repository and deleted those projects, and dropped the repository from the
others, keeping names and order. Two projects of one name in two repositories stay two. A pre-alpha change with no
compatibility kept.

Every write answers `Projects` whole, as `list_projects` does, and publishes the same value.

**An issue row keeps no state.** Only the tracker's reference is kept. Its `state` and `title` are
read when the rows are: from the open list Fleet holds for the Issues tab, and an issue not on it
read by its reference. Each is held five minutes. Where the tracker cannot say, both are absent and
the list is still answered. The tracker is not asked whether a linked issue exists.

**A task may have subtasks, one level deep.** A subtask is a task whose `parent` is the id of a top-level
task of the same project, or of the loose tasks. Rows are flat: a parent is followed by its subtasks, so a client indents a row
that has a `parent`. Positions count among the rows of one parent, and `move_project_row` may change the
parent (`""` is the top level). Removing a parent removes its subtasks. A subtask has no subtasks, a task
that has subtasks cannot become one, and an issue row is neither parent nor subtask. Holding and the done?
ask treat a subtask as any task, and a start that names a parent holds its subtasks too.

**An issue linked to a task keeps no state either.** A task, or a subtask, carries the references of the
issues linked to it, in the order linked. `title` and `state` come from the tracker as an issue row's
do, held five minutes, and are absent where it cannot say. Linking the same reference twice is one link.

**A pull request linked to a task keeps no state either.** Only its number is kept. `title` and `state` come from
the forge, one read of the pull request by its number, held five minutes as an issue's are, and are absent where the
forge cannot say. A draft reads `open`. The forge is not asked whether a linked number exists.

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

**A running Job may be linked to a task** (`link_task_job`), and the task is then held exactly as if the Job had
started with it, subtasks included, so every row above applies. A task holds one Job: linking another replaces it.
`unlink_task_job` lets a `held` task go to `open`. The Job must be a live Job of the task's repository.

**A linked pull request's merge asks about the task.** When Fleet learns that a pull request merged, whoever opened
it, each task linked to it that is `open` or `held` becomes `asked`, once per link: a task answered `Keep open` is not
asked about the same merge again, and unlinking and linking the pull request again resets that. A `done` task is left
alone. A Job that held the task keeps holding it and the ask is on its list as above; a task no Job holds is `asked`
with no `held_by`, and the ask lives on the rows alone, for the Projects tab to answer through `tick_project_task`. A
pull request that had already merged when it was linked asks at the link, by the same rule and the same flag, and the
answer to `link_task_pull` already shows the task `asked`. If the forge cannot say at that moment, the next sweep asks
while Fleet still reads it among the recently merged.

**The ask is held on the rows, not on the Job.** While the holding Job is live, Fleet adds a
`task_done` item to its waiting list, one per project: id `task_done:<project>`, text `Mark <title>
done?` for one task or `Mark the <project> tasks done?` for several, an `answer` act, and the options
`Mark done` and `Keep open`. Answering through `answer_waiting` settles every task the Job asked in that
project: `Mark done` makes them `done`, `Keep open` makes them `open`, and neither sends the agent a
message. The item cannot be dismissed. The ask stands on the rows when the Job ends, and the tick route
settles one row of it from the Projects tab: done, or open again. The item leaves with the last task asked.

**Errors.**
404 `fleet.no_such_project` is a project he has none of.
404 `fleet.no_such_project_row` is a row he has none of.
422 `fleet.project_blank` is a blank project name or task title.
422 `fleet.project_issue_unreadable` is a link `IssueReference` refuses.
422 `fleet.project_row_not_a_task` is an edit, a tick, a link (of an issue, a Job or a pull request) or a subtask on an issue row, or an issue row moved under a task.
422 `fleet.project_subtask_too_deep` is a subtask of a subtask, of a task that has subtasks, or of itself.
404 `fleet.no_such_session` is a Job to link that Fleet has no record of. 422 `fleet.project_job_other_repository` is a Job of
another repository, and 422 `fleet.project_job_ended` one that has ended. 422 `fleet.project_pull_unreadable` is a pull request number of 0.
`start_session` names rows: 404 `fleet.no_such_project_row` as above, and 422
`fleet.project_row_other_repository` is a row of another repository than the Job's. Nothing is made on a refusal.
422 for a repository Fleet does not serve, as every repository route answers it, on a task or an issue row written into one.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** `projects.changed` is published on a person's write, when a Job holds, asks about or lets go of tasks, and when a linked pull request's merge
asks about one, so it is rare.
The lifeboat is untouched.
