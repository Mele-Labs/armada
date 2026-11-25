# Projects: tasks and linked issues, across repositories

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

A person keeps projects on this Mac, each an ordered list of rows, and **a project is in no repository**. A row is a task he
wrote or an issue he linked, and each row names the repository it is in, so one project may hold rows of several. A task may
be in no project, and is then loose. The rows are kept in Fleet's store on this Mac, survive a restart, and are never written to
the repository or its Manifest. **A project's name and members are a file Fleet holds**, kept where he chose for it (this Mac, a
synced folder or the cloud), and a project is also what a Workspace was: the set of repositories a lifecycle or a rule is scoped to.
The protocol ID moves with the files below, and there is no number to take.
Bridge's half is `packages/protocol/src/projects.ts`, written by hand, field for field.

| Where | Side | Change |
| --- | --- | --- |
| `list_projects` | Bridge calls | `GET /projects`, answers `Projects { projects, loose, left_out }`: each `Project { id, name, rows, members, place, unread?, values? }` in his order, the loose tasks, of every repository, in their own order, and the project files Fleet could not read |
| `Project.members` | Fleet writes, Bridge reads | `[{ manifest_id, folder? }]`, the repositories the file lists in its order, each whole or, with `folder`, any folder of it, which needs no `armada.yml` of its own. Empty where `unread` |
| `Project.place` | Fleet writes, Bridge reads | `{ place, synced_folder?, cloud? }` as `RunMemoryPlace` carries a repository's: `this_mac` (where none was chosen), `synced` with its folder, or `cloud` with `{"store":{}}` or `{"bucket":"s3://b/p"}`. **Never `repository`** |
| `Project.unread` | Fleet writes, Bridge reads | Present where the project's file could not be read: the sentence that says why. `name` is then the last the store saw and `members` is empty |
| `Project.values` | Fleet writes, Bridge reads | `ValuesSet`, **only the keys the project sets**, absent where it sets none: `{ pr_mode?, auto_merge?, review_model?, look? { layout?, theme?, density? }, reviews? { plans?, pull_requests?, walks? }, agent_starts?, delivery? }`. `pr_mode` is `ready` or `draft`, `auto_merge` is `never`, `checks-pass` or `always`, `density` is `compact`, `default` or `roomy`, and each kind of `reviews` is `off`, `review` or `review_and_approve`, a walk's only `off` or `review`, a kind left out falling through to Kit's and then `off` (a block naming no kind is dropped from the file; clearing one kind is an edit whose block lacks it), and `agent_starts` is `at_once` or `ask_me`: whether a Job that a Job or the Operator starts begins at once or waits on its start card, and `delivery` is `auto_land`, `open_pr`, `keep_local` or `ask`: how a Job in the project lands its work, unset being the delivery question, whose first answer Fleet then keeps here, and `workflows_off` is the ids of the workflows the project switches off for its Jobs (see *Workflows switched off* under Values). It is the `values:` block of the project's file. Absent where `unread` |
| `Project.shared_with_macs` | Fleet writes, Bridge reads | `true` where its Jobs may be seen and acted on from his other Macs; **absent while false**, and every project starts false. It is `shared_with_macs: true` in the project's file, written only while on. Absent where `unread`. [Project](../../concepts/project.md), *Sharing with his other Macs* |
| `Project.mac` | Fleet writes, Bridge reads | The id of the paired Mac that holds the project, where it is a copy of one of that Mac's shared projects; **absent for this Mac's own**. A copy's edits, of a task or of the project itself, are sent to that Mac, and what only that Mac may change, its place, its store and a Job link, is 409 `fleet.other_mac_project_held`. [His other Macs, Fleet's half](his-other-macs-fleets-half.md) |
| `Projects.left_out` | Fleet writes, Bridge reads | `[{ place, path, why }]`, every project file that would not read, `place` `this_mac`, `synced` or `cloud`, `path` the file or the object a bucket holds it as. A file a project of his names is here too, and that project says `unread` |
| `Projects.left_behind` | Fleet writes, Bridge reads | `[{ place, path, why }]`, **absent while empty**. A project file an act took out of its place and could not take away: `place` `cloud`, `path` the object a bucket holds it as, `why` the bucket's words. Only the answer to `delete_project` or `edit_project` with a `place` carries it, and `projects.changed` does not |
| `list_holding_projects` | Bridge calls | `GET /projects/holding?manifest_id=`, the repository Fleet started in where absent: the projects whose members include it, whole or by a folder, answered as `Projects` with every `rows` and `loose` empty. It replaces `list_repository_workspaces`. 422 `fleet.unacceptable_proposal` for a repository Fleet does not serve |
| `list_forge_repositories` | Bridge calls | `GET /repositories/forge?q=`, `q` optional: the repositories his forge account can see, to pick one to clone into a project. Answers `ForgeRepositories { repositories, cut? }`, each `ForgeRepository { owner, name, url, private, pushed_at?, local_path? }`, newest pushed first; `url` is what `clone_repository` takes, and `local_path` the root of the served repository whose remote is this one, absent where Fleet serves none. At most 200, `cut` where more matched or the forge held more than the 1,000 Fleet reads. 422 `fleet.forge_repositories_unreadable` where the forge would not list them. Bridge's half is `packages/protocol/src/setup.ts` |
| `ProjectRow` | Fleet writes, Bridge reads | Tagged by `kind`: `task { id, manifest_id, title, notes, state, held_by?, parent?, issues?, pulls? }`, `state` `open`, `done`, `held` or `asked`, `held_by { session_id, title? }` present for `held` and for an `asked` a Job raised, `parent` the id of the task a subtask belongs to, `issues` `[{ issue, title?, state? }]` the issues linked to it, `pulls` `[{ number, title?, state? }]` the pull requests linked to it, `state` `open`, `merged` or `closed`; or `issue { id, manifest_id, issue, title?, state? }`, `state` `open` or `closed`. `manifest_id` is the repository of the row, and a subtask takes its parent's |
| `start_session` | Bridge calls | `project_rows?: [{ row }]` on `StartSession`: the rows the Job starts with, each named by its id alone. Each written task among them is held by the Job; an issue row is only named, so the brief can carry it |
| `StartSession.project` | Bridge calls | `project?: string`, the project of his the Job is for. 404 `fleet.no_such_project`, 422 `fleet.project_not_member` where the Job's repository is not a member. **Absent or blank, the Job is in the one project of his that holds its repository**, and in none where none does or more than one; a fork and a handoff keep their source Job's project, and a Job an agent starts takes its starter's, or this default where the starter has none |
| `HostedFacts.project` | Fleet writes, Bridge reads | `project?: string`, the id of the project the Job is in: the one it was started for, kept in a new column `hosted_sessions.project`, until `set_session_project` moves it. Absent for a Job in none |
| `add_project` | Bridge calls | `POST /projects`, body `NewProject { name, members?, place?, values? }`. Writes the file at the place, this Mac where absent, and the row, last in his order. `values` is the block it starts with, none where absent. It replaces the body `NameProject { name }` |
| `edit_project` | Bridge calls | `POST /projects/project/:project/edit`, body `EditProject { name?, members?, place?, values?, shared_with_macs? }`; only what is present changes, `members` replacing the whole list, `place` moving the file and taking the old one away (a refused bucket delete is in the answer's `left_behind`), `values` replacing the whole block, `{}` taking every value back, and `shared_with_macs` turning sharing with his other Macs on or off. It replaces `rename_project` and its route `/projects/project/:project/rename` |
| `get_values` | Bridge calls | `GET /projects/values?project=&manifest_id=`, both optional. Answers `ResolvedValues`: `pr_mode`, `auto_merge`, `review_model`, `layout`, `theme` and `density`, each `{ value?, from }`, `tracker`, `{ value?, from }` with `value` the Manifest's `tracker:` block as a map, resolved repository then project, and `reviews`, `{ plans, pull_requests, walks, from, plans_from?, pull_requests_from?, walks_from?, plans_inherited?, plans_inherited_from?, pull_requests_inherited?, pull_requests_inherited_from?, walks_inherited?, walks_inherited_from? }`, each a word, `from` the nearest layer that set any kind and each `*_from` the layer that kind's word came from (always filled by Fleet; an older answer lacks them), each `*_inherited` and `*_inherited_from` pair what the layers below the project give that kind, set only where its `*_from` is `project`, `agent_starts`, `{ value?, from }` with `value` `at_once` or `ask_me` (shipped `at_once`), `delivery`, `{ value?, from }` with `value` one of the four words and none where no layer sets it, `autonomy`, `[{ kind, value, from }]` for each kind of act where a repository was asked about, and `workflows`, `[{ id, on, from }]` for each workflow where a project was asked about (see *Workflows switched off* under Values). `from` is `default`, `machine`, `kit`, `repository`, `project` or `job`. **A plain key (every `{ value?, from }` above but `tracker`) also carries `learned?`, `inherited?` and `inherited_from?`**, see *Learned and inherited* under Values |
| `list_project_issues` | Bridge calls | `GET /projects/project/:project/issues?refresh=`. Answers `ProjectIssueList { project, fetched_at, issues, stale? }`, the open issues of the tracker the project names as `list_issues` carries a repository's, `holders` read from every repository the project holds. Held five minutes. 404 `fleet.no_such_project`, 422 `fleet.project_unreadable`, 422 `fleet.project_no_tracker` for a project that names none, 500 `fleet.issues_unreadable` |
| `set_project_workflow` | Bridge calls | `POST /projects/project/:project/workflow`, body `SetProjectWorkflow { workflow, on }`: one workflow switched off for the project's Jobs, or back on, and nothing else of its values changed. Written as `edit_project` writes `values`, so it answers `Projects` and publishes `projects.changed`. 422 `fleet.project_blank` for a blank `workflow` |
| `get_project_board` | Bridge calls | `GET /projects/project/:project/board`. Answers `ProjectBoard { project, layout? }`, `layout` `{ blocks: [{ kind, x, y, w, h, view }] }`. **A project that saved none answers no `layout`**, and Bridge draws its preset. `kind` and `view` are text, so a block kind a later Bridge adds crosses without a wire change and one this build does not know is read back as saved. See *The board*. 404 `fleet.no_such_project`, 422 `fleet.project_unreadable` |
| `save_project_board` | Bridge calls | `POST /projects/project/:project/board`, body `SaveProjectBoard { layout? }`: the layout replaces the one kept, whole, and an absent `layout` forgets it. Nothing else of the project's values changes. Written as `edit_project` writes `values`, so it publishes `projects.changed`, and for a copy of another Mac's project it is sent to that Mac. Answers `ProjectBoard`. 422 `fleet.project_values_refused` for a block with a blank `kind` or `view` |
| `get_home_layout` | Bridge calls | `GET /home/layout`. Answers `HomeLayout { layout? }`, `layout` `{ blocks: [{ kind, x, y, w, h, view }] }`, the project board's shape. **A user who saved none answers no `layout`**, and Bridge draws its preset. See *The Cockpit home's layout* |
| `save_home_layout` | Bridge calls | `POST /home/layout`, body `SaveHomeLayout { layout? }`: the layout replaces the one kept, whole, and an absent `layout` forgets it. Kept as `bridge.homeLayout` in settings.json, so it publishes `settings.changed`. Answers `HomeLayout`. 422 `fleet.unacceptable_home_layout` for a block with a blank `kind` or `view`, or more than 64 blocks, and nothing is written |
| `read_metrics` | Bridge calls | `GET /projects/project/:project/metrics`. Answers `MetricReadings { project, readings }`, a `MetricReading { id, title, state, answer?, read_at?, kept }` for each recipe the project keeps, in the file's order. Fleet fetches a recipe only where its last attempt is older than its `every_secs`. `state` is `read`, `no_token` or `broken`. See *Metrics*. 404 `fleet.no_such_project`, 422 `fleet.project_unreadable` |
| `try_metric` | Bridge calls | `POST /projects/project/:project/metrics/try`, body `TryMetric { recipe }`. Runs the recipe once with this Mac's token and answers its `MetricReading`, keeping nothing. 404 `fleet.no_such_project` |
| `save_metric` | Bridge calls | `POST /projects/project/:project/metrics`, body `TryMetric { recipe }`. Tries the recipe and keeps it as `look.metrics` where it read or only this Mac's token is missing, with a `metric:<id>` block at the bottom of a saved board. Answers `MetricSaved { project, recipe, reading }`. 404 `fleet.no_such_project`, 422 `fleet.metric_unreadable`, 422 `fleet.project_values_refused` |
| `remove_metric` | Bridge calls | `POST /projects/project/:project/metrics/:id/remove`. Drops the recipe and its block, and answers the remaining `MetricReadings`. Removing one that is not kept is not an error. 404 `fleet.no_such_project` |
| `save_metric_token` | Bridge calls | `POST /projects/project/:project/metrics/token`, body `SaveMetricToken { credential, token }`. Keeps the token in this Mac's keychain under the credential's name and answers `MetricReadings` read again. 404 `fleet.no_such_project`, 422 `fleet.metric_token_refused` |
| `list_ranked_issues` | Bridge calls | `GET /projects/project/:project/ranked_issues`. Answers `RankedIssues { project, days, fetched_at?, might_pick_up, might_be_interested }`: the open, unassigned issues as `RankedIssue { manifest_id?, issue, features, jobs }` and other people's open pull requests as `RankedPull { manifest_id, repository, number, title, author, draft, features, jobs }`, each `features` `[{ slug, title, by, jobs }]` and each list ranked by the owner's Jobs on those features in the last `days`. Asks no forge beyond the issue lists held five minutes. [Feature catalog](../../concepts/feature-catalog.md), *Ranking the issues he might pick up*. 404 `fleet.no_such_project`, 422 `fleet.project_unreadable`, 500 `fleet.issues_unreadable` |
| `get_system_map` | Bridge calls | `GET /projects/project/:project/system?part=&feature=`. Answers `SystemMap { project, part?, feature?, parts, links, features, stops, librarian_at? }`, read from the project's own files with no model. No query is level one, `part` level two and `feature` level three. See *The system map*. 404 `fleet.no_such_project`, `fleet.system_part_unknown`, `fleet.system_feature_unknown`; 422 `fleet.project_unreadable` |
| `get_system_part` | Bridge calls | `GET /projects/project/:project/system/part?part=&feature=`. Answers `SystemPartAnswers { project, part, feature?, … }`: `features`, `code`, `working`, `owners` with `owner_days`, `looks` with `has_screen`, `docs`, `monitor`, `issues` with `tracker`, `pulls`. Every section is present, for a part, for `feature=<slug>` and for neither, which is the whole project. 404 `fleet.no_such_project`, `fleet.system_part_unknown`, `fleet.system_feature_unknown` |
| `list_system_folder` | Bridge calls | `GET /projects/project/:project/system/folder?path=`. Answers `SystemFolder { project, path, entries }`: the names in the folder at the checkout's `HEAD`, folders first, each ending in `/`. 404 `fleet.no_such_project`, `fleet.system_folder_unknown` |
| `map_system_with_librarian` | Bridge calls | `POST /projects/project/:project/system/librarian`, no body. On his press the librarian adds the parts detection missed; level one answers with them as `origin` `librarian`. 409 `fleet.system_librarian_running` |
| `answer_system_addition` | Bridge calls | `POST /projects/project/:project/system/answer`, body `AnswerSystemAddition { addition, answer, note? }`, `answer` `keep`, `rework` or `remove`. Answers level one. 404 `fleet.system_addition_unknown`, 422 `fleet.system_addition_note` |
| `save_kit_values` | Bridge calls | `POST /kit/values`, body `SaveKitValues { values }`, `values` as a project's block is, written whole to `values.yml` in Kit's folder on this Mac. Answers `KitValuesSaved { file, values }` |
| `HostedFacts.values` | Fleet writes, Bridge reads | `ResolvedValues`, what the Job froze when it started, in its own `job_values` rows. Absent on a Job started before they were kept |
| `merge_projects` | Bridge calls | `POST /projects/merge`, body `MergeProjects { kept, gone, from_gone? }`. One of the two is this Mac's own and the other is a copy; either may be kept, and they have one name. `from_gone` is the `ValuesSet` keys where the gone project's value wins. The kept project must be shared, or 422 `fleet.merge_kept_not_shared`; names that differ are 422 `fleet.merge_names_differ`, and two own, two copies or one project are 422 `fleet.merge_not_a_pair`. Each Mac does its own part, and the other's is sent as `absorb` or `give_up`, queued while it sleeps. The answer lists one project from the press. [His other Macs, Fleet's half](his-other-macs-fleets-half.md) |
| `delete_project` | Bridge calls | `POST /projects/project/:project/delete`, no body. Its rows go with it, and its file wherever it is kept: Kit's folder, a synced folder or a bucket. A bucket that refuses the delete leaves the file and the answer's `left_behind` names it |
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
| `move_project_task_repository` | Bridge calls | `POST /projects/rows/:row/repository`, body `MoveProjectTaskRepository { manifest_id }`: a top-level task and its subtasks to another repository Fleet serves. It keeps its project, place and state, and a Job holding it keeps it. Its issues and pull requests keep the repository each was linked in. To the repository it is in changes nothing |
| `set_session_project` | Bridge calls | `POST /sessions/project`, body `SetSessionProject { session_ids, project }`: Jobs, live or ended, into a project made later, or out to none with `project: null`. Answers the `SessionList` of the Jobs as they are now, each also published as `session.changed`, and `projects.changed` once where a Job left a shared project, so his other Macs drop its row. Every Job is checked first and one refusal writes nothing. A repository the project does not hold becomes a whole member, as a task moved in brings its repository, and `projects.changed` is published where it did. The Job's frozen values and the tasks it holds are not touched. `agent_access` Bridge only |
| `tick_project_task` | Bridge calls | `POST /projects/rows/:row/tick`, body `TickProjectTask { done }`. On an `asked` row it answers the ask: done, or open again |
| `WaitingSource::task_done` | Fleet writes, Bridge reads | A Job's own waiting item while it is live and holds `asked` tasks: id `task_done:<project>`, with `loose` for the tasks in no project, an `answer` act, options `Mark done` (choice 0) and `Keep open` (choice 1), through `answer_waiting` |
| `projects.changed` | Fleet publishes | `Projects`, whole, on every write above; `set_session_project` where the project took a repository in, and where a Job left a shared project |

**A row's id is the whole address.** Ids are unique across every project and the loose tasks, so a route that acts on a row
names no project.

**A repository to clone is picked from the forge's list.** `list_forge_repositories` asks the forge at most once a minute: the whole list is kept in memory and each `q` narrows the kept one, so typing asks nothing, and a refusal is not kept.
`q` keeps the repositories whose `owner/name` holds every word of it, case-blind, a substring and not a fuzzy match.
**Cloned is what Fleet serves**: each served root's remote is compared with the listing's address as host, owner and name, and no other folder is read.
Additive: a Fleet without the route answers 404.

**Members follow tasks.** The owner decided on 10 Nov 2026 that a project's members are the repositories its tasks name. A task or an
issue row written into a project (`add_project_task`, `link_project_issue`, `add_project_subtask`), or moved into one
(`move_project_row`) or to another repository (`move_project_task_repository`), in a repository the project does not hold, makes that repository a
whole member, written to the file before the write answers. A repository held by a folder counts as held. A project whose file did not read is left as it is, since
a file that will not read is not written over. A repository leaves a project only by `edit_project`.

**A Job keeps its project.** `StartSession.project` is checked before anything is made: the project must be his and its file must read, and it
must list the Job's repository. The id is kept in `hosted_sessions.project`, which is new and additive, and served as `HostedFacts.project`; a Job replaced by a
handoff keeps it. It is not read with `fork` or `root`.

**A Job's project can be moved afterwards.** `set_session_project` sets `hosted_sessions.project` for each Job it names, live or ended, to a project made later or to none.
It checks every Job and the project before writing anything: 404 `fleet.no_such_session` for a Job this Fleet does not host, a copy from another Mac included, 422 `fleet.session_command_refused` for no Job or a Job with no repository (Setup, mod), 404 `fleet.no_such_project` and 422 `fleet.project_unreadable` for the project.
Unlike a start, **a repository the project does not hold is not a refusal**: it becomes a whole member, written to the file before the Jobs are set.
`HostedFacts.values` and the tasks a Job holds stay as they were; the readers that follow the Job's project live (tracker, delivery question, lifecycles, rules, sharing) follow the new value.

**Values.** Pull request mode, auto merge, autonomy, review model and the look (layout, theme, density) are a project's to set, key by key over Kit.
How a repository runs (commands, Checks, ports, setup) stays the repository's `armada.yml`.
The layers, **nearest winning**, are default, Machine, Kit, repository, Project and Job, and every answer names the one it came from.

**Learned and inherited.** Two things a project's settings need beyond the word and its layer, both on `ResolvedValue` and both left off the wire when they say nothing.
`learned` is true only on `delivery`, only where its layer is `project`, and only while the project's word is the one Fleet learned from the person's first answer to the delivery question.
Fleet keeps that in a store note, one row for (project, key) holding the word and when, **never in the project's file**: a file holds only what a person could have written.
The note is kept after the learning's own `edit_project` succeeds, and dropped when another write to the project's file (the person's edit) leaves `delivery` absent or a different word; a rewrite of the same word keeps it, and deleting the project drops its notes.
A read also demands that the note's word equal the resolved word, so a file changed some other way never shows `learned` for a word it was not learned as. A Job's frozen values never carry it.
`inherited` and `inherited_from` are set only on a key whose `from` is `project`, from a second resolve for the same repository with no project: the word and layer the layers below (repository, Kit, Machine, default) give.
Nothing below setting a word leaves `inherited` absent and `inherited_from` `default`, so a reader can tell "unset below" from "not asked".
`reviews` carries them per kind: `plans_inherited` and `plans_inherited_from`, `pull_requests_inherited` and `pull_requests_inherited_from`, `walks_inherited` and `walks_inherited_from`, each pair set only where that kind's own `*_from` is `project`, from the same second resolve.
A kind always has a word, so where nothing below sets one the pair reads `off` and `default`, and the word is present rather than absent.
`tracker` and `autonomy` carry none, and neither does a Job's frozen `hosted.values`, reviews included.

| Layer | Where Fleet reads it |
| --- | --- |
| default | What Armada ships: pull requests `ready`, auto merge `never`, the shipped theme, density `default`, and the review model the Machine's settings ship. Layout has none |
| Machine | `settings.json`, only what is saved: `features.draftPullRequests`, `models.review`, `bridge.theme`, `bridge.layout`, `spacing.density` |
| Kit | `values.yml` in Kit's folder, `version: 1` and a `values:` block. Read each time it is asked. **An absent file sets nothing**, and one that will not read sets nothing and is said on the log |
| repository | The Manifest's `pr_mode:` and `auto_merge:`, **only where it writes them**: a repository that says nothing does not beat Kit. They stay in `armada.yml` and sit just under the project |
| Project | The `values:` block of the project's file. **A project's auto merge replaces the repository's**, and so does its tracker; nothing is folded between them |
| Job | What the Job itself was started with. Nothing sets one yet |

**Autonomy has no second home.** The pieces it already has are read: those scoped to every repository are `kit`, those scoped to the named project are `project`, and those scoped to the repository are `repository`, a repository's laid over its project's as the pieces always were.
**Autonomy's `from` follows that order, not the one every other value follows**: where a repository and its project both scope a piece for one act kind, the repository's wins and `from` reads `repository`, though for `pr_mode`, `auto_merge` and the rest a project is over its repository.
It is the existing order, and the one the Operator's rules take.
A project is only asked about by `get_values` with a `project`.
With no repository there is no autonomy to read, so `autonomy` is left out.

**The tracker is a project's value and not Kit's.** `values.tracker` is the Manifest's `tracker:` block as a map of text, `provider` and every other key carried as written, on `Project.values`, `NewProject.values` and `EditProject.values`.
A repository's own is its Manifest's block, which `get_values` reads as the repository layer, and a Job in a project that names one asks it over its repository's.
Kit has none: `save_kit_values` refuses one, and Kit's `values.yml` does not read one.

**Workflows switched off.** A project's `values.workflows_off` lists the workflows it switches off for its Jobs, by id.
The machine's list and each repository's are the `workflows.off` setting in `settings.json`, and Kit has none: `save_kit_values` refuses one, and Kit's `values.yml` does not read one.
**Off wins.** The three lists add up: a workflow any of them names is off for a Job in the project, and a project cannot switch back on what the machine or its repository switched off.
So each switch names the furthest layer that holds it off, `machine`, then `repository`, then `project`, and `default` where none does and it is on; a project's switch changes it only where `from` is `project` or `default`.
`get_values` with a `project` serves `workflows`, one `{ id, on, from }` for each workflow the repository asked about holds, or every served repository's where none was, in id order.
Without a repository, a workflow every repository the project keeps switches off is off from `repository`, since no Job in it can follow it; one off in only some is left as the machine's list and the project's say.
`features.workflows`, the switch for all of them, is not folded in. `set_project_workflow` adds an id to the list or takes it out.
Whether a workflow is on is read live, by the proposer, Autopilot's picker, `set_session_workflow` and the rail, for the Job's project and repository; it is not frozen in `HostedFacts.values`, whose `workflows` is always empty.

**A Job takes its project's values at start.** A Job that holds lifecycles resolves for its project and its repository, and keeps every key with its layer in `job_values`, a table of its own and additive, served as `HostedFacts.values`.
A Job started for no project freezes its repository's.
**The tracker is not frozen.** `HostedFacts.values` leaves `tracker` out, since a Job asks the tracker where it acts and the project's word is read then.
A later edit of a layer does not move a Job that has begun, and a Job handed on takes the values the first froze.
Where it reads them:

| The Job's | Reads |
| --- | --- |
| Pull request | `pr_mode` where Kit, the repository, the project or the Job set it, else the Machine's word as it stands. A `draft` opens the pull request Fleet opens for it as a draft |
| Auto merge | The Job freezes the word and the layer it came from, and serves them. **Auto land still starts off for every Job**, as it always did, and is turned on from the card or by his choice; a value of `checks-pass` or `always` does not turn it on |
| Review model | `review_model` where Kit, the repository, the project or the Job set it, else `models.review`, which is live |
| Agent starts | `agent_starts` as the Job froze it, served as `hosted.values.agent_starts`. A Job that started before the key existed has none. Fleet decides a start by reading the word live for the calling Job's project and the new Job's repository |
| Delivery | `delivery` is frozen with the rest and served as `hosted.values.delivery`, **but the question reads it live** for the Job's project and repository: a word moved since the Job started is the one that answers. Fleet writes the project's file with the first answer where no layer sets it, which is an `edit_project` that changes `delivery` and nothing else. Refused 422 `fleet.project_values_refused` for any other word, in a project's values and in Kit's |
| Reviews | `reviews` as the Job froze it, **never live**: the Operator reads a Job's plan, pull request or walk as its start said. A Job that froze none is Off |

**The Machine's word and the default are not frozen into a read.** They are settings a person moves while a Job runs, and `models.review` and `features.draftPullRequests` were live.
A Job that could not freeze its values (the repository is not served) reads the layers where it reads a value, and no start is refused for it.

**What the file said before.** Fleet read each Workspace file Kit or a repository kept, and each task list in the store, into a project at start, once:
`projects_migrated`. `GET /workspaces/all` and `GET /workspaces?manifest_id=` (`list_workspaces`, `list_repository_workspaces`) answered the Workspaces
under their old shape, `WorkspaceOfRepositories`, and are gone; `list_projects` and `list_holding_projects` answer the same files. Bridge's Workspaces page
reads `list_projects` and draws each project that reads as the Workspace it did. A pre-alpha change with no compatibility kept.

**What moved from the per-repository wire.** A project once belonged to one repository and every route named it
(`/projects/:repository/...`). Moving to this shape, Fleet's store gave each row its project's repository, made the rows of every
`own:<repository>` project loose rows of that repository and deleted those projects, and dropped the repository from the
others, keeping names and order. Two projects of one name in two repositories stay two. A pre-alpha change with no
compatibility kept.

Every write answers `Projects` whole, as `list_projects` does, and publishes the same value.

**An issue row keeps no state.** Only the tracker's reference is kept. Its `state` and `title` are
read when the rows are: from the open list Fleet holds for the Issues tab, and an issue not on it
read by its reference. Each is held five minutes. Where the tracker cannot say, both are absent and
the list is still answered. **In a project that names a tracker** the rows and the issues linked to its tasks are read through it,
from its own open list and by reference, held five minutes under the project's key. A loose task and a task in a project that names none
read their repository's. The tracker is not asked whether a linked issue exists, **except in a project that names one**: `link_project_issue` and
`link_task_issue` read the issue there, and 404 `fleet.no_such_issue` where it answers it has none. A tracker that cannot answer refuses nothing.

**A task may have subtasks, one level deep.** A subtask is a task whose `parent` is the id of a top-level
task of the same project, or of the loose tasks. Rows are flat: a parent is followed by its subtasks, so a client indents a row
that has a `parent`. Positions count among the rows of one parent, and `move_project_row` may change the
parent (`""` is the top level). Removing a parent removes its subtasks. A subtask has no subtasks, a task
that has subtasks cannot become one, and an issue row is neither parent nor subtask. Holding and the done?
ask treat a subtask as any task, and a start that names a parent holds its subtasks too.

**An issue linked to a task keeps no state either.** A task, or a subtask, carries the references of the
issues linked to it, in the order linked. `title` and `state` come from the tracker as an issue row's
do, held five minutes, and are absent where it cannot say. Linking the same reference twice is one link.

**A link keeps the repository it was linked in.** `TaskIssue` and `TaskPull` carry `manifest_id`, the repository the issue or pull request
was linked in, which is not its task's once the task has moved (`move_project_task_repository`). Fleet always says it, and a client reads it absent as
the task's own. The tracker and the forge are asked in that repository.

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
422 `fleet.project_malformed` is members that do not make a project: a folder that is not a path inside its repository, a member listed twice, a whole repository beside a folder of it, or an id that is a member's too.
`add_project` and `edit_project`.
422 `fleet.project_place_refused` is a place that cannot be used: `repository`, a synced folder that is not there, a cloud place with no address, a bucket that cannot be reached.
`add_project` and `edit_project`.
`fleet.project_unwritable` is a project's file that could not be written or removed, a 500, or one that would not read and so is not written over by `edit_project`, a 422. A task written into a project whose file did not read is kept and adds no member.
422 `fleet.project_values_refused` is a value its key does not take: an `auto_merge` that is not `never`, `checks-pass` or `always`, a `density` that is not `compact`, `default` or `roomy`, or blank text.
`add_project`, `edit_project` and `save_kit_values`, a tracker in Kit's values among them.
Nothing is written.
422 `fleet.project_no_tracker` is a project's issues asked of a project whose file names no tracker. `list_project_issues`.
`fleet.kit_values_unwritable` is Kit's `values.yml` that could not be written, a 500.
`get_values` names a project: 404 `fleet.no_such_project`, 422 `fleet.project_unreadable` where its file did not read, 422 `fleet.project_not_member` where it does not list the repository, and 422 `fleet.unacceptable_proposal` for a repository Fleet does not serve.
422 `fleet.project_not_member` is a Job started for a project that does not list its repository, `start_session`.
422 `fleet.project_unreadable` is a Job started for a project whose file did not read, so Fleet cannot say what it holds. `start_session`.
422 `fleet.project_issue_unreadable` is a link `IssueReference` refuses.
422 `fleet.project_row_not_a_task` is an edit, a tick, a link (of an issue, a Job or a pull request) or a subtask on an issue row, an issue row moved under a task, or an issue row moved to another repository.
422 `fleet.project_subtask_too_deep` is a subtask of a subtask, of a task that has subtasks, or of itself, or a subtask moved to another repository on its own.
422 `fleet.unacceptable_proposal` is a task moved to a repository Fleet does not serve.
404 `fleet.no_such_session` is a Job to link that Fleet has no record of, and 422 `fleet.project_job_ended` one that has ended.
422 `fleet.project_pull_unreadable` is a pull request number of 0.
`start_session` names rows: 404 `fleet.no_such_project_row` as above, and a project: 404 `fleet.no_such_project`, 422 `fleet.project_not_member` or `fleet.project_unreadable`.
Nothing is made on a refusal.

**A Job holds a task of any repository.** The owner chose it on 29 Oct 2026, after a task filed in the wrong repository refused every start from the dispatch sheet.
The task keeps the repository it was filed in, and the Job its own: holding is a link between the two, and the store never compared them.
422 for a repository Fleet does not serve, as every repository route answers it, on a task or an issue row written into one.

**Skew.** There is no number to take: the protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, all of which changed here.
The values are additive on top of that: `values` on `Project`, `NewProject` and `EditProject` and on `HostedFacts` is absent for none, and `get_values`, `save_kit_values` and `list_project_issues` are routes a Fleet without them answers 404.
The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves. A Bridge that still calls `GET /workspaces/all`, `GET /workspaces` or
`POST /projects/project/:project/rename` would meet a 404 on a Fleet that has them gone, which is why the two are never run together.
**The event stream.** `projects.changed` is published on a person's write, when a Job holds, asks about or lets go of tasks, and when a linked pull request's merge
asks about one, so it is rare.
The lifeboat is untouched.

## Review on a new pull request

A project's `values.review_on_new_pr` is a list of review tool ids, and it is the project's alone: a Kit file that names it is refused with `fleet.project_values_refused`.
No list means nothing happens.
When Fleet sees an open pull request in a project's repository that it has not seen before, from any author and draft or not, it adds an item to the Operator's Your turn list: `Run <tools> on #N?` with Run and Not now.
The item id is `review:<manifest>:<number>`.
Run starts one review run per tool through `start_review_run`, and Not now dismisses the pull request for good.
A tool is offered only when the repository defines it as a review tool, and the item is withdrawn when the pull request closes.
The first look at a repository records what is open and offers none of it, so turning the setting on does not offer the backlog.
Fleet keeps the numbers it has seen in the store table `review_offered`, so a restart offers nothing twice.
The wire change is `review_on_new_pr` on `ValuesSet`, absent for none, and it moves the protocol ID with `crates/ipc/src` and `packages/protocol/src`.

## The board

A project's Dashboard keeps where each of its blocks stands in the project's file, as `values.look.board`, with the rest of its look.
`LookSet.board` is `{ blocks: [{ kind, x, y, w, h, view }] }`, and `get_project_board` and `save_project_board` read and change that alone.
A project that saved none has none, and Bridge draws its preset.
`kind` and `view` are Bridge's words, kept as written, so `system`, `docs` or `errors` add without a wire change and a kind this build does not know is passed back as it was saved.
A layer other than the project's ignores `board`: it is never laid over another.
The wire change moves the protocol ID with `crates/ipc/src` and `packages/protocol/src`.

## The system map

A project's system map is what it is made of and how the parts reach each other, read from its own files with no model.
Where the project picked folders of a repository, the map, its features and the nine answers read only the files in those folders, and `list_system_folder` lists only the way down to them and refuses a folder outside them with `fleet.system_folder_unknown`.
`get_system_map` answers one level at a time, so Bridge draws a level without holding the others.
No query is level one: the people, the entry points, the services behind them, the stores and what the project reaches outside.
`part` is level two: that part's pieces, the pieces of other parts they call, and the stores and outside parts they reach.
`feature` is level three: the files the feature owns, grouped by the part they fall in as `stops`, numbered in level one's order of entry, service, store, with a link stepped from each stop to the next.
A part is `person`, `entry`, `service`, `store`, `outside` or, on levels two and three, `piece`.
A piece names the part it is in as `within`, and a piece of another part keeps that part's name there.
A project that holds several repositories names the repository in `manifest_id`, and a part id that two repositories share is told apart as `id@manifest`.

Detection is table-driven and reads, never runs.
The tables are `crates/fleet/src/system_map/known.toml`, so a vendor's name is data and not code.
Fleet finds the modules and their dependencies with the floor plan's reader, the workspaces and compose services, and the servers a Kit manifest declares.
An entry point is a module on an app or frontend dependency, an HTTP server nobody runs from here, the binary named for the project, a browser extension's `manifest.json` or a script that takes arguments.
A service is a module on a server dependency with a `main`, a module that implements an HTTP API, or a deployed service by its files.
A store is a dependency that keeps state, a compose image, or a folder of state files a source file names, and an outside part is a client dependency, a file named for it or an `api.` address the code names.
A library every part uses is a piece of the part that reaches it, and the project's own tooling is nothing.
A link is drawn only where the code names it: a route a client names and a server declares, a crate or an import a file uses, a dependency a file reaches a store or outside part by.
Its `label` is that route, and a link of several routes has none.
A client that names a route an API library declares reaches the service that serves it, and the API links to the service as `served by`.

The map of a repository is held per `HEAD`, so a read after the first costs nothing until a commit lands.
A repository that is no git checkout is read from its folder and held nowhere.

`get_system_part` answers the nine sections from what Fleet already holds.
`features` are the catalog features that own files under the part, `working` the live Jobs whose observed writes fall there, `owners` the share of the commits over its paths in the last 30 days, `docs` the repository documents that name its folder or its name, `monitor` the Kit servers, health routes, log folders and Checks over its paths, `issues` the open issues from the lists held that match its features, and `pulls` the open pull requests whose files fall in it.
`looks` is absent until a feature shot is kept, which Fleet does not keep yet, and `has_screen` says a part or a piece of one that draws a screen has none to show: a piece within an entry that is a GUI or web app, or an extension, has a screen.
No forge is asked.

**Every level shows the same nine sections.**
`part` is one part or piece.
`feature=<slug>` is one feature: its prefixes and the files filed to it as `code`, the live Jobs writing them as `working`, the owners of those files, the documents the Jobs that changed them read first as `docs`, the issues matched to it and the open pull requests whose files fall in it.
Neither is the whole project: each repository's root as `code`, every feature, the Jobs writing anywhere in it, owners across it, the documents at the root (README, then `docs/`), every Kit server and run as `monitor`, every issue matched to a feature and every open pull request.
`part` answers empty for those two, and `feature` is present only for a feature.
Both given, `feature` wins.

The librarian adds on his press only.
`map_system_with_librarian` proposes each folder of source that no detected part covers, and the part comes back with `origin` `librarian` until he answers it.
Keep makes it a detected part, Rework stores his note and the next press redoes that addition, and Remove drops it and it is not proposed again.
The additions are kept in the store tables `system_additions` and `system_librarian`.
The pass reads files and calls no model yet.

## The Cockpit home's layout

The Cockpit home, the right side of Bridge while no Job is selected, keeps one layout per user, as `bridge.homeLayout` in Fleet's settings.json.
It is `BoardLayout`, the project board's own shape, so Bridge draws both with one grid: `{ blocks: [{ kind, x, y, w, h, view }] }`.
`get_home_layout` and `save_home_layout` read and change it alone, and a save publishes `settings.changed` as any key of the file does.
A user who saved none has none, and Bridge draws its preset.
`kind` and `view` are Bridge's words, kept as written, so a kind this build does not know is passed back as it was saved.

**It follows him across his Macs, the latest save winning.**
`HomeLayout` carries an additive `saved_at`, the time of the save that made it, and `bridge.homeLayout` keeps it beside the blocks as `{ saved_at, layout? }`.
A forgetting is a stamp with no `layout`, so it beats an earlier layout on another Mac; a bare layout written by hand has no stamp and loses to any stamped one.
Bridge reads `layout` as before and need not read `saved_at`.
It crosses on the channel Kit uses: `save_home_layout` publishes `home_layout.changed`, which carries nothing and crosses to the linked Macs, whose Gateways read `share_home_layout` (`GET /other_mac/home_layout`, answered `HomeLayout`) and hand it to their Fleets as `hold_other_mac_home_layout` (`PUT /other_macs/home_layout`, body `OtherMacHomeLayout { mac, home }`).
A Gateway also reads it when its link opens and on every resync, so a Mac that was asleep catches up.
A Fleet replaces its layout only where the one handed in has the later `saved_at`; two saves of one instant are settled by their text, so both Macs pick the same.
A layout taken is kept with its stamp and is not told on, so two Macs settle and stop; `settings.changed` tells Bridge as for any write.
A Mac on a build from before this answers 404 to the read, and the link stays up without it.
Clocks are the order: two Macs whose clocks differ by more than the time between his saves can take the earlier save for the later.
The wire change moves the protocol ID with `crates/ipc/src` and `packages/protocol/src`.
## Metrics

A project's Dashboard can show live metric blocks, and Fleet reads each one with no model.
The librarian writes a recipe once: one call over a command line, HTTP or an MCP server, as a tracker mod's call is written, and a `jq` `map` from its answer to rows.
A recipe is kept in the project's file as `values.look.metrics`, beside `values.look.board`, so it reaches the project's other Macs with the file.
`LookSet.metrics` is `[MetricRecipe]`, and `save_metric` and `remove_metric` change that and the board alone.
A project that kept none has none, and a layer other than the project's ignores `metrics`, as it does `board`.
A recipe's block on the board is kind `metric:<id>`, view `list`, three columns by two rows.
`save_metric` appends it at the bottom of a board the project saved, and where the project saved none the board stays absent and Bridge appends the block to its preset.

A call runs as a tracker mod's does, by the same code, with the same bounds of 30 seconds and 4 MiB of answer and no shell.
`{credential}` is filled into `env`, `arguments`, `headers` and `body`, and never into `argv`, `server` or `url`, which a process list or a log would show.
A cli call takes `env` as an mcp call does, the variables set on the program, so a command line tool that reads its token from the environment can be called.
A recipe that uses it names a credential.
An HTTP call is a `GET` or a `POST`, to an `https://` address or to this machine.
What `map` prints is read as one `MetricAnswer { rows, picture? }`, each row a `label`, a `value` that is a number or words, and optionally `unit`, `at` and `link`.
Rows that carry `at` are a series.
`picture` is offered only where an answer gives one.

The token is an API token the person pastes once per Mac, kept in the macOS keychain with one item per tool: its service is `armada-metric-token` and its account the credential's name, so two recipes naming `sentry` share it.
Tokens never leave the Mac.
A token is in no file, answer, reading, error or log: every why is scrubbed of it before it is said, and `SaveMetricToken` prints without it.
A recipe whose credential this Mac's keychain lacks reads as `no_token` with the credential to make, and `save_metric_token` reads the project again.

Fleet fetches only while the project's dashboard is open, and there is no timer in it.
Bridge asks with `read_metrics`, and Fleet fetches a recipe only where it was never read or its last attempt is older than the recipe's `every_secs`, 60 where it names none.
A recipe that is broken is asked again after its interval and not at each ask.
Two asks at once wait on one fetch.
A broken fetch keeps the last `answer` that read, and `read_at` says when.
`kept` is the first numeric row of each good reading Fleet made, oldest first, at most 500.
It is held in memory, so a restart starts it again, and Graph draws it where the answer is not itself a series.

`save_metric` tries the recipe and keeps it where it read, or where the only thing missing is this Mac's token.
A recipe that cannot read is refused 422 `fleet.metric_unreadable` with the scrubbed why, and nothing is kept.
`try_metric` runs a recipe once and keeps nothing.
These routes have no agent door: the librarian's Job reaches `save_metric`'s work through Fleet directly.
The wire change moves the protocol ID with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`.
