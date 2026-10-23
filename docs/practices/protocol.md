# Protocol: the Fleet/Bridge seam

Armada has exactly one place where Rust stops and TypeScript starts: the wire
between Fleet (the daemon, Rust) and Bridge (the desktop app, Electron). Every
other boundary in the system is a function call or a file. This one is a
process boundary between two binaries with independent release cadence and
independent lifetimes, and everything in this document exists because that
combination has already gone wrong once, in v1, and cost real debugging time
figuring out which side was lying.

If your change touches `crates/ipc`, `packages/protocol`, the route table in
`crates/api/src/routes/`, anything under `apps/desktop/src/preload`, or the
WebSocket event stream, read this first.

<!-- split: one file per section; re-run the split on the whole doc -->

| Section |
|---|
| [The protocol ID](protocol/the-protocol-id.md) |
| [DTOs, not domain types](protocol/dtos-not-domain-types.md) |
| [Changes that read like they break nothing](protocol/changes-that-read-like-they-break-nothing.md) |
| [What Bridge does with the ID it reads](protocol/what-bridge-does-with-the-id-it-reads.md) |
| [Why a mismatch is dangerous here specifically](protocol/why-a-mismatch-is-dangerous-here-specifically.md) |
| [The v0 lifeboat](protocol/the-v0-lifeboat.md) |
| [The first socket: the fleet's facts](protocol/the-first-socket-the-fleets-facts.md) |
| [The unmeasured risk: the WebSocket sink has no back-pressure](protocol/the-unmeasured-risk-the-websocket-sink-has-no-back-pressure.md) |
| [The run sockets: one run, one server](protocol/the-run-sockets-one-run-one-server.md) |
| [The Helm socket: retired](protocol/the-helm-socket-retired.md) |
| [The Commander routes and the wave routes: gone](protocol/the-commander-routes-and-the-wave-routes-gone.md) |
| [Changes before IDs](protocol/changes-before-ids.md) |
| Protocols 10.x to 21.x, archived: `git show ecfffb2c1ebd:docs/practices/protocol/protocol-<major>-<minor>-<title>.md` |
| [Other things specific to this seam](protocol/other-things-specific-to-this-seam.md) |
| [Protocol 22.0: a task in between, a task that failed, and the Record's two new signers](protocol/protocol-22-0-a-task-in-between-a-task-that-failed-and-the-records-two.md) |
| [Protocol 22.1: the merge line, read off disk](protocol/protocol-22-1-the-merge-line-read-off-disk.md) |
| [Protocol 23.0: a node sits inside a Zone or a Cluster](protocol/protocol-23-0-a-node-sits-inside-a-zone-or-a-cluster.md) |
| [Protocol 23.1: a Drone per task, and the task a change moved](protocol/protocol-23-1-a-drone-per-task-and-the-task-a-change-moved.md) |
| [Protocol 23.2: what landed, and what was sent back](protocol/protocol-23-2-what-landed-and-what-was-sent-back.md) |
| [Protocol 23.3: the files a fix holds off the Jobs that hit its test](protocol/protocol-23-3-the-files-a-fix-holds-off-the-jobs-that-hit-its-test.md) |
| [Protocol 23.4: groups, a task that failed, and two acts on them](protocol/protocol-23-4-groups-a-task-that-failed-and-two-acts-on-them.md) |
| [Protocol 23.5: a pull request's title and comment count, after it merges](protocol/protocol-23-5-a-pull-requests-title-and-comment-count-after-it-merges.md) |
| [Protocol 23.6: a model per task, and a person's edit to a task](protocol/protocol-23-6-a-model-per-task-and-a-persons-edit-to-a-task.md) |
| [Protocol 23.7: one Check's log on a merge line](protocol/protocol-23-7-one-checks-log-on-a-merge-line.md) |
| [Protocol 23.8: the proposal is what runs](protocol/protocol-23-8-the-proposal-is-what-runs.md) |
| [Protocol 23.9: a node is added inside a Zone](protocol/protocol-23-9-a-node-is-added-inside-a-zone.md) |
| [Protocol 23.10: several Drones at once, and one of them addressed](protocol/protocol-23-10-several-drones-at-once-and-one-of-them-addressed.md) |
| [Protocol 23.11: a Drone resting at the gate, and turns nobody counted](protocol/protocol-23-11-a-drone-resting-at-the-gate-and-turns-nobody-counted.md) |
| [Protocol 23.12: a Job's retro, and who acted](protocol/protocol-23-12-a-jobs-retro-and-who-acted.md) |
| [Protocol 23.13: an Epic's wave, released in one press](protocol/protocol-23-13-an-epics-wave-released-in-one-press.md) |
| [Protocol 23.14: each wave's line, and the order a wave waits in](protocol/protocol-23-14-each-waves-line-and-the-order-a-wave-waits-in.md) |
| [Protocol 23.15: where a retro item's fix lands](protocol/protocol-23-15-where-a-retro-items-fix-lands.md) |
| [Protocol 23.17: a person reshapes the slot pool](protocol/protocol-23-17-a-person-reshapes-the-slot-pool.md) |
| [Protocol 23.18: walk notes, kept on the Job, and a server started for review](protocol/protocol-23-18-walk-notes-kept-on-the-job-and-a-server-started-for.md) |
| [Protocol 23.19: a step's phase, and the grants a canvas draws](protocol/protocol-23-19-a-steps-phase-and-the-grants-a-canvas-draws.md) |
| [Protocol 23.20: a step tuned at the press](protocol/protocol-23-20-a-step-tuned-at-the-press.md) |
| [Protocol 23.21: a branch that does not exist yet](protocol/protocol-23-21-a-branch-that-does-not-exist-yet.md) |
| [Protocol 23.22: where an approved Job lands, set once](protocol/protocol-23-22-where-an-approved-job-lands-set-once.md) |
| [Protocol 23.23: the harness a step runs under](protocol/protocol-23-23-the-harness-a-step-runs-under.md) |
| [Protocol 23.24: how the work leaves the worktree, as approved](protocol/protocol-23-24-how-the-work-leaves-the-worktree-as-approved.md) |
| [Protocol 23.25: a note sent back to the proposer](protocol/protocol-23-25-a-note-sent-back-to-the-proposer.md) |
| [Protocol 23.27: a retro item says where it stands](protocol/protocol-23-27-a-retro-item-says-where-it-stands.md) |
| [Protocol 23.26: a retro item has parts, and a person answers it](protocol/protocol-23-26-a-retro-item-has-parts-and-a-person-answers-it.md) |
| [Protocol 23.28: a workflow definition saved, and the folders read again](protocol/protocol-23-28-a-workflow-definition-saved-and-the-folders-read-again.md) |
| [Protocol 23.29: a Scout reads a stranded slot](protocol/protocol-23-29-a-scout-reads-a-stranded-slot.md) |
| [Protocol 23.33: where a workflow came from, one read whole, and a start that leaves out](protocol/protocol-23-33-where-a-workflow-came-from-one-read-whole-and-a-start.md) |
| [Protocol 23.34: Helm is told when a person is on Workflows](protocol/protocol-23-34-helm-is-told-when-a-person-is-on-workflows.md) |
| [Protocol 23.30: a Scout's Finding picked up](protocol/protocol-23-30-a-scouts-finding-picked-up.md) |
| [Protocol 23.31: what a step does, in words](protocol/protocol-23-31-what-a-step-does-in-words.md) |
| [Protocol 23.35: Accept changes the Kit](protocol/protocol-23-35-accept-changes-the-kit.md) |
| [Protocol 23.36: a merge held behind a need](protocol/protocol-23-36-a-merge-held-behind-a-need.md) |
| [Protocol 23.37: Clear commits uncommitted files](protocol/protocol-23-37-clear-commits-uncommitted-files.md) |
| [Protocol 23.39: pause and resume a Job](protocol/protocol-23-39-pause-and-resume-a-job.md) |
| [Protocol 23.40: who asked for a run, and a Drone's asked runs](protocol/protocol-23-40-who-asked-for-a-run-and-a-drones-asked-runs.md) |
| [Protocol 23.41: the hub on the merge line](protocol/protocol-23-41-the-hub-on-the-merge-line.md) |
| [Protocol 23.42: acting on main's red, and what merged](protocol/protocol-23-42-acting-on-mains-red-and-what-merged.md) |
| [Protocol 23.43: the job ledger](protocol/protocol-23-43-the-job-ledger.md) |
| [Protocol 23.44: a held red, and each merge's run on main](protocol/protocol-23-44-a-held-red-and-each-merges-run-on-main.md) |
| [Protocol 23.46: needs on the job ledger](protocol/protocol-23-46-needs-on-the-job-ledger.md) |
| [Protocol 23.48: acts on a pull request](protocol/protocol-23-48-acts-on-a-pull-request.md) |
| [Protocol 23.49: a job Fleet hosts](protocol/protocol-23-49-a-job-fleet-hosts.md) |
| [Protocol 23.50: taking a Job over](protocol/protocol-23-50-taking-a-job-over.md) |
| [Protocol 23.51: a session started on a Job, and the commands an agent names](protocol/protocol-23-51-a-session-started-on-a-job-and-the-commands-an-agent.md) |
| [Protocol 23.52: a job's name](protocol/protocol-23-52-a-jobs-name.md) |
| [Protocol 23.56: a command and a compaction in a terminal job's thread](protocol/protocol-23-56-a-command-and-a-compaction-in-a-terminal-jobs-thread.md) |
| [Protocol 23.58: Triggers on the wire](protocol/protocol-23-58-triggers-on-the-wire.md) |
| [Protocol 23.69: a job forked from an ended one](protocol/protocol-23-69-a-job-forked-from-an-ended-one.md) |
| [Protocol 23.62: a mod that is out of date](protocol/protocol-23-62-a-mod-that-is-out-of-date.md) |
| [Protocol 23.64: Checks reported while they wait](protocol/protocol-23-64-checks-reported-while-they-wait.md) |
| [Protocol 23.68: steps added to one Job, and the repository's Draft default](protocol/protocol-23-68-steps-added-to-one-job-and-the-repositorys-draft-default.md) |
| [Protocol 23.68: a failed Trigger's repair on the wire](protocol/protocol-23-68-a-failed-triggers-repair-on-the-wire.md) |
| [Protocol 23.68: a Trigger that blocks holds the Job](protocol/protocol-23-68-a-trigger-that-blocks-holds-the-job.md) |
| [Protocol 23.68: a draft default for pull requests](protocol/protocol-23-68-a-draft-default-for-pull-requests.md) |
| [A saved Trigger runs a Drone](protocol/a-saved-trigger-runs-a-drone.md) |
| [Protocol 23.78: a Job's retro in its thread](protocol/protocol-23-78-a-jobs-retro-in-its-thread.md) |
| [Protocol 23.77: Start Job on a retro item](protocol/protocol-23-77-start-job-on-a-retro-item.md) |
| [Protocol 23.78: a slash command run in a terminal Job](protocol/protocol-23-78-a-slash-command-run-in-a-terminal-job.md) |
| [Protocol 23.89: a run's end is a row in the thread](protocol/protocol-23-89-a-runs-end-is-a-row-in-the-thread.md) |
| [Protocol 23.88: a Job's run is on a terminal](protocol/protocol-23-88-a-jobs-run-is-on-a-terminal.md) |
| [Protocol 23.76: a shell block run from a Job's thread](protocol/protocol-23-76-a-shell-block-run-from-a-jobs-thread.md) |
| [Protocol 23.75: a Job's Checks](protocol/protocol-23-75-a-jobs-checks.md) |
| [Protocol 23.74: a guided review of the open retro items](protocol/protocol-23-74-a-guided-review-of-the-open-retro-items.md) |
| [Protocol 23.87: one file of a pull request, read when it is opened](protocol/protocol-23-87-one-file-of-a-pull-request-read-when-it-is-opened.md) |
| [Protocol 23.86: a Job's standing, and the replied and done items](protocol/protocol-23-86-a-jobs-standing-and-the-replied-and-done-items.md) |
| [Protocol 23.85: the model a hosted Job resolved](protocol/protocol-23-85-the-model-a-hosted-job-resolved.md) |
| [Protocol 23.83: a pull request's page and the writes on it](protocol/protocol-23-83-a-pull-requests-page-and-the-writes-on-it.md) |
| [Protocol 23.81: write clashes between agents](protocol/protocol-23-81-write-clashes-between-agents.md) |
| [Protocol 23.80: a Job an agent started](protocol/protocol-23-80-a-job-an-agent-started.md) |
| [Protocol 23.84: a Job's running turn stopped](protocol/protocol-23-84-a-jobs-running-turn-stopped.md) |
| [Protocol 23.92: a Job paused and resumed](protocol/protocol-23-92-a-job-paused-and-resumed.md) |
| [Protocol 23.79: a Job forked at a message](protocol/protocol-23-79-a-job-forked-at-a-message.md) |
| [Protocol 23.73: Skill and Drone steps run](protocol/protocol-23-73-skill-and-drone-steps-run.md) |
| [Protocol 23.72: repair on an added step](protocol/protocol-23-72-repair-on-an-added-step.md) |
| [Protocol 23.70: the merge queue on the hub](protocol/protocol-23-70-the-merge-queue-on-the-hub.md) |
| [Protocol 23.71: a Job's question answered](protocol/protocol-23-71-a-jobs-question-answered.md) |
| [Merge pressed while the forge's checks run](protocol/merge-pressed-while-the-forges-checks-run.md) |
| [Mods: a theme or a layout a job can make](protocol/mods-a-theme-or-a-layout-a-job-can-make.md) |
| [settings.json: every Machine setting, described and saved](protocol/settings-json-every-machine-setting-described-and-saved.md) |
| [A call's arguments are no longer served](protocol/a-calls-arguments-are-no-longer-served.md) |
| [Autopilot: a repository works its own backlog](protocol/autopilot-a-repository-works-its-own-backlog.md) |
| [A Job's Now: what runs, what is wrong, what it waits on](protocol/a-jobs-now-what-runs-what-is-wrong-what-it-waits-on.md) |
| [Autopilot's follow-ups](protocol/autopilots-follow-ups.md) |
| [Routing a thought: Helm reads words for where they belong](protocol/routing-a-thought-helm-reads-words-for-where-they-belong.md) |
| [Helm's message box: files, tags, model and effort, the `/` list](protocol/helms-message-box-files-tags-model-and-effort-the-list.md) |
| [The plan's windows, and what the limit holds](protocol/the-plans-windows-and-what-the-limit-holds.md) |
| [The Job model steps down](protocol/the-job-model-steps-down.md) |
| [A Job's own default model](protocol/a-jobs-own-default-model.md) |
| [The Fleet's timeline: kept as it happens, read as a window](protocol/the-fleets-timeline-kept-as-it-happens-read-as-a-window.md) |
| [When a Job's Drone was last told](protocol/when-a-jobs-drone-was-last-told.md) |
| [A stranded slot's last holder, and whether Fleet told it](protocol/a-stranded-slots-last-holder-and-whether-fleet-told-it.md) |
| [An agent lists Jobs in a line each](protocol/an-agent-lists-jobs-in-a-line-each.md) |
| [What a Job is doing, from its tool calls](protocol/what-a-job-is-doing-from-its-tool-calls.md) |
| [A Job's build record](protocol/a-jobs-build-record.md) |
| [A Job's plan and its file touches](protocol/a-jobs-plan-and-its-file-touches.md) |
| [Protocol 23.91: a Job's request is read, and its workflow is on the record](protocol/protocol-23-91-a-jobs-request-is-read-and-its-workflow-is-on-the-record.md) |
| [A Job's rail and its judges](protocol/a-jobs-rail-and-its-judges.md) |
| [A terminal's permission prompt is a call](protocol/a-terminals-permission-prompt-is-a-call.md) |
| [Helm is a Job](protocol/helm-is-a-job.md) |
| [A waiting item's steps](protocol/a-waiting-items-steps.md) |
| [A Job moves to another repository](protocol/a-job-moves-to-another-repository.md) |
| [A Job hands off to a new one](protocol/a-job-hands-off-to-a-new-one.md) |
| [An issue is named by its tracker's reference](protocol/an-issue-is-named-by-its-trackers-reference.md) |
| [A Job's kept follow-ups, and settling one](protocol/a-jobs-kept-follow-ups-and-settling-one.md) |
| [A bay's history: the pool's own record, and the slot's reflog](protocol/a-bays-history-the-pools-own-record-and-the-slots-reflog.md) |
| [A Job walks any page: `steps` on `show_window`](protocol/a-job-walks-any-page-steps-on-show-window.md) |
| [A Job's window: origins and a sign-in](protocol/a-jobs-window-origins-and-a-sign-in.md) |
| [The Reviews tab: the lists waiting on a person, a page's threads and a review's comments](protocol/the-reviews-tab-the-lists-waiting-on-a-person-a-pages-threads-and-a.md) |
| [The Reviews tab: agent runs and review tools](protocol/the-reviews-tab-agent-runs-and-review-tools.md) |
| [A Job's running Check: its log live, and a stop](protocol/a-jobs-running-check-its-log-live-and-a-stop.md) |
| [A Job's reach past its slots: the folder answers](protocol/a-jobs-reach-past-its-slots-the-folder-answers.md) |
| [A Job follows the Jobs its plan was handed to](protocol/a-job-follows-the-jobs-its-plan-was-handed-to.md) |
| [A Job waits on another](protocol/a-job-waits-on-another.md) |
| [A full pool asks the person: `slot_choice`](protocol/a-full-pool-asks-the-person-slot-choice.md) |
| [A Job's delivery question](protocol/a-jobs-delivery-question.md) |
| [A sketch wireframes a screen: the `screen` node](protocol/a-sketch-wireframes-a-screen-the-screen-node.md) |
| [Autopilot starts Sessions, not Jobs](protocol/autopilot-starts-sessions-not-jobs.md) |
| [Sessions absorb Jobs, group 1: the Job arms and piloting a Job leave the wire](protocol/sessions-absorb-jobs-group-1-the-job-arms-and-piloting-a-job-leave-the.md) |
| [Sessions absorb Jobs, group 2: the Job surface leaves the wire](protocol/sessions-absorb-jobs-group-2-the-job-surface-leaves-the-wire.md) |
| [A Job's Loadout](protocol/a-jobs-loadout.md) |
| [A Job's attachment history](protocol/a-jobs-attachment-history.md) |
| [Needs are deleted](protocol/needs-are-deleted.md) |
| [A write clash's file is read as two patches](protocol/a-write-clashs-file-is-read-as-two-patches.md) |
| [Usage a turn at a time](protocol/usage-a-turn-at-a-time.md) |
| [The usage summary](protocol/the-usage-summary.md) |
| [The usage rules](protocol/the-usage-rules.md) |
| [A question refused for a walk is kept whole](protocol/a-question-refused-for-a-walk-is-kept-whole.md) |
| [Sessions are called Jobs: four tools listed under new names](protocol/sessions-are-called-jobs-four-tools-listed-under-new-names.md) |
| [Every file of a pull request, read as its patch](protocol/every-file-of-a-pull-request-read-as-its-patch.md) |
| [A walk's notes, sent as one turn: `send_session_notes`](protocol/a-walks-notes-sent-as-one-turn.md) |
| [A Job's MCP servers: `get_session_mcp`, `act_on_session_mcp`](protocol/a-jobs-mcp-servers.md) |
| [Kit's Add and Remove: a plugin or a skill](protocol/kit-add-and-remove-a-plugin-or-a-skill.md) |
| [Open questions](#open-questions) |

## A pinned note, sent whole: `send_session_note`

Since 17 Oct 2026. `docs/concepts/session.md`.
**Additive only**: one operation, one route and the types they carry; no event kind, no error and no field on an existing type.
The protocol ID moves with `crates/ipc/src`, the operation file, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/session-notes.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `send_session_note` (`POST /sessions/note`, `agent_access = "No"`) | Bridge sends, Fleet answers 202 | `SendSessionNote` `{ session_id, said, address, note_id?, place, state, crop? }`; answered with `NoteSent` `{ session, candidates }` |
| `NotePlace` | Bridge sends | `component?`, `owners?`, `selector`, `element` (`NoteElement` `{ tag, text, label? }`), `nearby?`, `stamp?`, `location`, `screen?`, `layer?` |
| `NoteState` | Bridge sends | `selected?`, `walk?` (`NoteWalk` `{ name, step, of }`), `take?` (`NoteTake` `{ letter, name }`), `scenario?` |
| `NoteCrop` | Bridge sends | `{ media_type, data }`, `data` base64 |
| `Candidate` | Fleet answers | `{ file, line, why, text }`, at most five, best first; `file` is relative to the checkout Fleet searched, `line` is 1-based |

**Bridge's call** is `sendSessionNote` on `window.armada` (channel `bridge:send-session-note`, `SessionsHost.sendNote` in main).
It folds `NoteSent.session` into the held Sessions as a message's answer is, and hands the candidates back to the caller.
An empty array on the wire is the same as an omitted one, so the mirror marks each as optional.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
Were a pair to meet anyway, a Fleet without the route answers 404, which Bridge words as a send that failed.

**The event stream.** Untouched: the turn it takes publishes `session.changed` as a message does, and nothing new. The open risk below is neither better nor worse.

## Flight controls, and a shell action: `get_flight_controls`, `save_flight_controls`, `run_session_shell`

Since 18 Oct 2026. `docs/concepts/kit.md`, *Flight controls*.
**Additive only**: three operations, three routes and the types they carry; no event kind and no field on an existing type.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/flight-controls.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `get_flight_controls` (`GET /kit/flight_controls`, `Bridge only`) | Bridge reads | `FlightControlsRead`, tagged by `what`: `read` with `controls`, or `not_read` with `why` |
| `save_flight_controls` (`POST /kit/flight_controls`, `Bridge only`) | Bridge sends the whole file, Fleet answers 200 | `FlightControls` `{ version, job[], deck[] }`; answers `FlightControlsRead`, read again. 422 `fleet.flight_controls_refused` naming the field, 500 `fleet.flight_controls_unwritable` |
| `run_session_shell` (`POST /sessions/shell`, `agent_access = "No"`) | Bridge sends, Fleet answers 200 when the line has ended | `RunSessionShell` `{ session_id, command }`; answers `SessionShellRan` `{ exit, timed_out, output, truncated }`. 422 `fleet.session_shell_refused` |
| `FlightControl` | both | `id`, `name?`, `glyph?`, `key?`, `hidden?`, `text?`, `on_text?`, `off_text?`, `kind?` (`words`, `command`, `shell` or `page`) |

**Fleet does not know the built-in ids**, so it does not check that a `kind` sits only on an id that is not built in, nor that `on_text` and `off_text` sit only on the land tile.
Bridge checks both before it saves.
Fleet checks what it can see: the version, a blank or repeated `id`, and a `key` that is not one of `1` to `9`, `0` or a letter, or is used twice in one list.

**Neither the save nor the shell is an agent tool.**
A saved shell action runs when the person presses it, and `Bridge only` and `No` keep both off the agent door.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
Were a pair to meet anyway, a Fleet without the routes answers 404, which Bridge words as a read or a press that failed.

**The event stream.** Untouched: nothing here publishes an event, and the run is answered on the request that started it.
The open risk below is neither better nor worse.

## An issue read whole: `get_issue`

Since 18 Oct 2026. `docs/concepts/tracker.md`.
**Additive only**: fields on one existing answer and the types they carry; no operation, route, event kind or error.
The protocol ID moves with `crates/ipc/src`, the operation file and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/issues.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `IssueRead` (`GET /issues/:repository/:reference`) | Fleet writes, Bridge reads | Gains `body` (whole markdown), `author` (login), `opened_at` (RFC 3339), `labels`, `assignees`, `comments` and `linked_pull_requests`. `reference`, `title`, `url` and `state` are unchanged |
| `IssueComment` | Fleet writes | `{ author, at, body }`, oldest first, the first hundred |
| `LinkedPullRequest` | Fleet writes | `{ number, title, url, state }`, `state` one of `open`, `merged`, `closed` |

**One forge request.** The forge's tracker reads the issue, its comments, the pull requests that close it and the ones that name it in one `api graphql` call.
A pull request that closes an issue and names it too is listed once, by address.
A tracker that reads only a title and a state (Jira) answers the new fields empty.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** Untouched: no event carries an issue. The open risk below is neither better nor worse.

## Starting out: a new project, found folders, how a repository runs, where it is kept

Since 19 Oct 2026. `docs/concepts/kit.md`, `docs/concepts/session/installing-the-claude-code-mod.md`.
**Bridge's half only**: Fleet serves these routes already, and no operation, route or error is added here.
The protocol ID moves with `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/session-import.ts` and `run-memory.ts`, with `CreateProject` in `setup.ts`, written by hand, field for field.

| Bridge calls (`window.armada`) | Route | Answers |
| --- | --- | --- |
| `createProject(parent, name)` | `POST /repositories/create` | `LocateAnswer`, as `addRepository` does; listed before it answers |
| `listImportFolders()` | `GET /import/folders` | `ImportFolders` |
| `getImportRuns(path)` | `GET /import/runs?path=` | `ImportRuns` |
| `installAgentMod()` | `POST /import/install_mod` | `ModInstalled`; main sends `{"requested":true}` itself, and no caller can send `false` |
| `getRunMemory(repository)` | `GET /repositories/runs?repository=` | `RunMemory` |
| `getRunMemoryPlace(repository)` / `setRunMemoryPlace(body)` | `GET` / `POST /repositories/runs/place` | `RunMemoryPlace`; `fleet.run_place_refused` and the three `fleet.run_cloud_*` codes arrive as a 422 refusal in Fleet's words |
| `dropRunFact(body)` | `POST /repositories/runs/drop` | `RunMemory`, as it now stands |

Every call but `createProject` answers `OnboardingAnswer<T>`: `{ ok: true, value }` or `{ ok: false, outcome }`.
The waits: the two reads of past sessions wait 120 s and the install 7 min, past Fleet's own bounds, by the comment in `apps/desktop/src/main/onboarding.ts`.

**The event stream.** Untouched: nothing here publishes an event. The open risk below is neither better nor worse.

## What holds a plan: `SessionPlan.held_by`

Since 19 Oct 2026. `docs/concepts/session/the-plan.md`.
**Additive only**: one field on an existing type and the type it carries; no operation, route, event kind or error.
The protocol ID moves with `crates/ipc/src` and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/session-plan.ts`, written by hand, field for field. Bridge draws nothing from it yet.

| Where | Side | Carries |
| --- | --- | --- |
| `SessionPlan` (on `SessionRecord.plan`) | Fleet writes, Bridge reads | Gains `held_by`, absent when empty: each window keeping a pending plan from the person |
| `PlanHold` | Fleet writes | `{ url, why }`, `why` one of `unapproved`, `changing`, `changed`, `walk` |

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** `SessionChanged` carries the record, so it carries the field; no new event.

## A cut command is asked whole: `HelmCallInFlight.whole`

Since 19 Oct 2026. `docs/concepts/session/a-job-fleet-hosts.md`.
**Additive only**: one field on an existing type; no operation, route, event kind or error.
The protocol ID moves with `crates/ipc/src` and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/helm-calls.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `HelmCallInFlight` (`helm.asking_to_run`, `list_helm_calls`, `SessionRow::Ask`, `hosted.asked`) | Fleet writes, Bridge reads | Gains `whole`: the argument as the session sent it, set only where `detail` was cut, absent otherwise. `detail`, `truncated` and `length` are unchanged |

**Bridge's card draws `whole` where it is present**, so a person allowing a command reads all of it; the Needs you row still reads its first line.
This serves an ask's argument where *A call's arguments are no longer served* left none: an ask only, never a row of the activity log.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** Every event and record that carries an ask carries the field, so a cut command now travels whole on the socket and in the stored thread.
The open risk below is slightly worse: an ask's frame grows with its command, which `CallDetail` does not bound.

## A repository's own tiles: `get_repository_flight_controls`, `save_repository_flight_controls`

Since 19 Oct 2026. `docs/concepts/kit.md`, *Flight controls*.
**Additive only**: two operations, one route path and the types they carry; no event kind and no field on an existing type.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/flight-controls.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `get_repository_flight_controls` (`GET /repositories/flight_controls?repository=`, `Bridge only`) | Bridge reads | `RepositoryFlightControls` `{ repository, place, read }`: `place` is a `RunPlace`, `read` a `FlightControlsRead` |
| `save_repository_flight_controls` (`POST /repositories/flight_controls`, `Bridge only`) | Bridge sends the whole file, Fleet answers 200 | `SaveRepositoryFlightControls` `{ repository, controls }`; answers `RepositoryFlightControls`, read again. 422 `fleet.flight_controls_refused` naming the field, 422 `fleet.flight_controls_unplaced`, 422 `fleet.run_cloud_*`, 404 `fleet.run_memory_repository_not_served`, 500 `fleet.flight_controls_unwritable` |

**Kept where the repository's runs are**, by the place chosen for them: `flight-controls/<key>.toml` beside `runs/<key>.toml` under Kit's folder, a synced folder or a bucket's prefix.
`place` on the answer says which was read, so Bridge never guesses where a tile lives.
`cloud` with Armada's store, and `repository`, keep the file on this Mac as `this_mac` does, and the answer's `place` says `this_mac`: a repository's tiles are the person's own, and sharing them with collaborators gets designed later.

**Every tile carries a `kind`.** A repository holds a person's own actions; the built-ins' order and hiding stay in `get_flight_controls`.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
Were a pair to meet anyway, a Fleet without the route answers 404, which Bridge would word as a read that failed.

**The event stream.** Untouched: nothing here publishes an event. The open risk below is neither better nor worse.

## Commit to repository: `commit_to_repository`, and the repository's own tiles read

Since 19 Oct 2026. `docs/concepts/kit.md`, *Commit to repository*.
**One route for every kind of piece**: the body's `piece` says which, so a fourth kind is a variant and not a route.
It replaces the parked approach of committing tiles as a save place: `save_repository_flight_controls` never writes the repository.
Bridge's half is `packages/protocol/src/committing.ts` and `in_repository` in `packages/protocol/src/flight-controls.ts`, written by hand, field for field.
Bridge reads `in_repository` with the open Job's `session_id`, and sends a commit from a tile's sheet, a Kit workflow or a Kit review tool.

| Where | Side | Carries |
| --- | --- | --- |
| `commit_to_repository` (`POST /repositories/commit`, `Bridge only`) | Bridge sends, Fleet answers 200 once committed | `CommitToRepository` `{ repository, session_id?, piece, overwrite? }`; answers `CommittedToRepository` `{ repository, branch, paths[], commit? }`. 422 `fleet.commit_no_job`, `fleet.commit_not_yours`, `fleet.commit_exists`, `fleet.commit_refused`; 404 `fleet.run_memory_repository_not_served`; 500 `fleet.commit_unwritable` |
| `PieceToCommit` | both | tagged by `kind`: `flight_controls` `{ job[]?, deck[]? }` (tile ids), `workflow` `{ workflow_id }`, `review_tool` `{ id }` |
| `get_repository_flight_controls` (`GET /repositories/flight_controls?repository=&session_id=`) | Bridge reads | `RepositoryFlightControls` gains `in_repository`, a `FlightControlsRead` of the repository's own `.armada/flight-controls.toml`, from the tree of the Job `session_id` names, or the checkout with none |

**A copy, never a move.** The person's piece is read and left as it is; the copy is written in the Job's tree and committed alone on its branch.
**With no Job named that holds a slot in the repository, the commit is refused** with `fleet.commit_no_job`, since Fleet does not guess among several Jobs.
An id the branch already holds is a 422 `fleet.commit_exists` without `overwrite`, as `save_review_tool` refuses a clobber.

**Not an agent tool.** `Bridge only`: what is committed can run a shell line for everyone on the repository.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
Were a pair to meet anyway, a Fleet without the route answers 404, and an older Fleet's read lacks `in_repository`.

**The event stream.** Untouched: nothing here publishes an event.

## Kit's wins, and says what it hides: `hides`

Since 19 Oct 2026. `docs/concepts/workflow.md`, *Where a definition comes from*, and `docs/concepts/pull-review.md`, *Review tools*.
**Additive only**: one field on two existing types; no operation, route, event kind or error.
The protocol ID moves with `crates/ipc/src` and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/setup.ts` and `review-runs.ts`, written by hand, field for field.
Bridge marks a workflow row and a review tool's tile that carry it with the eclipse glyph, its tooltip naming what it hides.

| Where | Side | Carries |
| --- | --- | --- |
| `WorkflowSummary` (`list_workflows`) | Fleet writes, Bridge reads | Gains `hides`, `"repository"` where the row is Kit's and replaced a repository definition that says something else; absent otherwise |
| `ReviewTool` (`list_review_tools`, `save_review_tool`) | Fleet writes, Bridge reads | Gains `hides`, the same, as a `ReviewToolSource` |

**What changed under the field is the order.** Kit's now replaces the repository's of the same id, so `OverriddenWorkflow.source` and `ReviewTool.replaces` can say `repository`, and a repository's file is never the one that hides Kit's.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** Untouched: nothing here publishes an event. The open risk below is neither better nor worse.

## A person's own armada.yml: `ManifestSummary.kit`

Since 19 Oct 2026. `docs/concepts/manifest.md`, *Kit's copy*.
**Additive only**: one field on an existing type and the two types it carries; no operation, route, event kind or error.
The protocol ID moves with `crates/ipc/src`, the operation files and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/setup.ts`, written by hand, field for field.
Bridge marks each Check or Command of `names` with `hides` on the Manifest surface's list, with the eclipse glyph.

| Where | Side | Carries |
| --- | --- | --- |
| `ManifestSummary` (`list_manifests`, and every answer that carries one) | Fleet writes, Bridge reads | Gains `kit`, absent where Kit keeps no copy for the repository |
| `ManifestFromKit` | Fleet writes | `{ file, whole?, names?, not_read?, left_out? }` |
| `ManifestKitName` | Fleet writes | `{ kind, name, hides? }`, `kind` one of `check` or `command` |

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** Untouched: nothing here publishes an event. The open risk below is neither better nor worse.

## Keep on this Mac: `get_kept_on_this_mac`, `set_kept_on_this_mac`

Since 19 Oct 2026. `docs/concepts/kit.md`, *Kept on this Mac*.
**Additive only**: two operations, one route path, the type they carry and one error code; no event kind and no field on an existing type.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/run-memory.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `get_kept_on_this_mac` (`GET /repositories/kept_on_this_mac?repository=`, `No`) | Bridge reads | `KeptOnThisMac` `{ repository, kept }`, `false` absent a mark; 404 `fleet.run_memory_repository_not_served` |
| `set_kept_on_this_mac` (`POST /repositories/kept_on_this_mac`, `No`) | Bridge sends `KeptOnThisMac`, Fleet answers 200 | The same, read again; marking moves the run memory place to `this_mac` |
| `set_run_memory_place` | Fleet answers | Gains `422 fleet.run_place_kept_here` for any place but `this_mac` while the repository is marked |

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
Were a pair to meet anyway, a Fleet without the route answers 404, which Bridge would word as a read that failed.
**The event stream.** Untouched: nothing here publishes an event. The open risk below is neither better nor worse.

## A tile the repository committed asks first: `approve_repository_tile`

Since 19 Oct 2026. `docs/concepts/kit.md`, *Flight controls*; `docs/design/who-sees-each-piece-of-configuration.md`, *A tile a teammate committed asks before it runs*.
**Additive only**: one operation, one route, the types it carries, and optional fields on two existing types.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/flight-controls.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `approve_repository_tile` (`POST /repositories/flight_controls/approve`, `agent_access = "No"`) | Bridge sends, Fleet answers 200 | `ApproveRepositoryTile` `{ repository, session_id?, tile_id, command_hash }`; answers `RepositoryFlightControls`, read again. 422 `fleet.tile_command_changed`, 422 `fleet.tile_not_in_repository`, 404 `fleet.run_memory_repository_not_served`, 500 `fleet.tile_approval_unwritable` |
| `run_session_shell` | Bridge sends | `RunSessionShell` gains `repository_tile?` `{ repository, id }`, set for a tile that came from the repository, which then runs the repository's command, not `command`. Refused with 422 `fleet.tile_not_approved` while that command is not approved, and `fields` carry `repository`, `tile_id`, `command` (the repository's) and `command_hash`; 422 `fleet.tile_not_in_repository` for an id the repository does not hold |
| `get_repository_flight_controls` | Bridge reads | `RepositoryFlightControls` gains `asking?`, a `TileAsk` `{ id, command_hash, command, approved }` for each shell tile of `in_repository`; left out where there is none |

**The approval names a hash, so it cannot cover a later command.**
Fleet reads the tile as committed now and refuses a `command_hash` that is not its current command.
A press with `repository_tile` names the tile, not the command: Fleet reads the tile's command from the repository's committed `.armada/flight-controls.toml` itself, from the tree of the Job the press names, then hashes and checks that, and runs it.
The `command` the press sent is not used, and a `repository_tile` id the repository does not hold is refused with 422 `fleet.tile_not_in_repository`.
A tile that changed asks again.

**Kept on this Mac only**, in `~/.armada/approved-tiles/<owner--name>.toml` by tile id, never in the store, a synced folder or a bucket.
Only a press with `repository_tile` asks: a person's own tile, and one imported from a package, never carries it.
A tile has no working directory yet, so the hash covers the command text alone.

**Not an agent tool.** `agent_access = "No"`: an agent that could approve a tile could run its command.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.

## Share: `share_package`, `read_package`, `add_package`, `list_unbound_pieces`, `bind_unbound_piece`

Since 19 Oct 2026. `docs/concepts/kit.md`, *Share*; `docs/design/who-sees-each-piece-of-configuration.md`, *Share is a package*.
**Additive only**: five operations, five paths under `/share` and the types they carry; no event kind and no field on an existing type.
**Fleet makes the text and reads it**: Bridge writes the file and the clipboard, and takes the text from a dropped file or a paste.
Bridge's half is `packages/protocol/src/sharing.ts`, written by hand, field for field.
Bridge calls all five: Share and Copy on a piece, and Import for a package dropped or pasted on its window.

| Where | Side | Carries |
| --- | --- | --- |
| `share_package` (`POST /share/package`, `Bridge only`) | Bridge sends the chosen pieces, Fleet answers 200 | `SharePieces` `{ pieces: [{ repository, piece }] }`, `piece` being `commit_to_repository`'s `PieceToCommit`; answers `SharedPackage` `{ file_name, text }`. 422 `fleet.share_not_yours`; 404 `fleet.run_memory_repository_not_served` |
| `read_package` (`POST /share/read`, `Bridge only`) | Bridge sends the text, Fleet answers 200 and writes nothing | `PackageText` `{ text }`; answers `PackageRead` `{ pieces[] }`, each `{ kind, id, name, list?, action?, commands[], remote?, repository?, exists }`. 422 `fleet.share_malformed` |
| `add_package` (`POST /share/add`, `Bridge only`) | Bridge sends the text | `AddPackage` `{ text, replace? }`; answers `PackageAdded` `{ pieces[] }`, each `{ kind, id, name, list?, remote?, repository?, landed, unbound_id? }`, `landed` being `added`, `replaced` or `unbound`. 422 `fleet.share_malformed`, `fleet.share_exists`, `fleet.share_refused`, a workflow's own `fleet.workflow_*`; 500 `fleet.share_unwritable` |
| `list_unbound_pieces` (`GET /share/unbound`, `Bridge only`) | Bridge reads | `UnboundPieces` `{ pieces[] }`, each `{ unbound_id, kind, id, name, list?, commands[], remote? }` |
| `bind_unbound_piece` (`POST /share/bind`, `Bridge only`) | Bridge sends | `BindUnboundPiece` `{ unbound_id, repository, replace? }`; answers `PieceAdded`. 404 `fleet.share_no_unbound_piece`, `fleet.run_memory_repository_not_served`; 422 as `add_package` |

**`read_package` is a command** because the text travels in the body, and the route table holds a query to `GET`.
**The text is a `version` key and one `[[piece]]` table per piece**: its `kind` (`tile`, `workflow` or `review_tool`), the piece as stored, and `remote` where the repository has one.
A remote is `owner/name`, as the central store's slug is.
Nothing else is in it: no path, user name, machine, Kit place or timestamp.

**A piece lands on the one repository whose remote is its own.**
A remote that names none of the repositories Fleet serves, or several, keeps the piece unbound in Kit's folder `unbound/` on this Mac, and `bind_unbound_piece` writes it once onto a repository the person chooses.
Only the pieces the package holds are written.
**An id Kit holds, a repository's tiles that would break a rule, and a workflow `save_workflow` would refuse are found for every piece before the first is written**, and leave Kit as it was.
A workflow is checked against the repository's Checks as `save_workflow` checks it.
**A file that cannot be written leaves the pieces written before it written**: workflows come first, then review tools, then each repository's tiles one repository at a time, then the unbound pieces.
A workflow and a review tool have one place in Kit for every repository, so a package holding one id of either for two repositories is refused on read, naming the id.

**Not an agent tool.** `Bridge only`: what is added can run a shell line on the person's Mac.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
Were a pair to meet anyway, a Fleet without the routes answers 404.
**The event stream.** Untouched: nothing here publishes an event.

## Kit's armada.yml follows the place, and commits: `PieceToCommit::Manifest`

Since 19 Oct 2026. `docs/concepts/manifest.md`, *Kit's copy*; `docs/concepts/kit.md`, *Commit to repository*.
**One new variant on a type that exists, and no operation, route, event kind or error code.**
The protocol ID moves with `crates/ipc/src` and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/committing.ts`, written by hand, field for field.
Bridge sends it nowhere yet.

| Where | Side | Carries |
| --- | --- | --- |
| `PieceToCommit` (`commit_to_repository`) | Bridge sends | Gains `manifest`, tagged by `kind` and carrying nothing: the person's own `armada.yml` for `repository` |
| `fleet.commit_exists` | Fleet answers | Now also where the Job's branch already holds `armada.yml` and `overwrite` was not asked for |
| `fleet.commit_not_yours` | Fleet answers | Now also where Kit holds no `armada.yml` for the repository, or its place would not answer |
| `fleet.commit_refused` | Fleet answers | Now also where Kit's copy is not a whole Manifest |
| `ManifestFromKit.file` | Fleet writes | A path on this Mac, or the bucket address where the copy is kept in a bucket |
| `PieceKind` (`read_package`, `add_package`, `list_unbound_pieces`) | Fleet writes | Gains `manifest`, with id and name `armada.yml` |
| A Share package's `[[piece]]` | both | Gains `kind = "manifest"` with `text`, the file as Kit holds it, and `remote` |
| `fleet.share_not_yours`, `fleet.share_exists`, `fleet.share_refused`, `fleet.share_unwritable`, `fleet.share_malformed` | Fleet answers | Now also for a manifest piece: Kit holds none, Kit holds one and `replace` was not asked for, the place would not answer, the place would not take the write, the text is not a mapping of keys |

**The place is the tiles' place.**
Kit's copy is read from, and written to, the folder, synced folder or bucket the repository's tiles are kept in, and from `~/.armada` while the repository is kept on this Mac.
A place that cannot be read reads as no copy: `ManifestSummary.kit.left_out` names it where the repository has an `armada.yml`, and Fleet's log names it where it has none.
`armada check` reads the place the same way, from the store file Fleet keeps, and reads this Mac where there is none.

**Share carries the same piece.**
`share_package` takes `{kind:"manifest"}` in `PieceToShare.piece` and packages Kit's file for that repository with its remote.
`read_package` lists it with every Check and Command `run` line it would add as `commands`, and `repository` the one served repository whose remote is its own.
`add_package` writes it to the place the matched repository's Kit copy is kept in, refuses one that exists unless `replace`, and keeps it unbound where the remote matches none, as for any other piece.
A place that would not answer is refused with `fleet.share_refused` before anything is written, and the file is written after workflows and review tools and before tiles.
A package's manifest need not be a whole Manifest: a copy that names only the Checks and Commands it changes is what a person keeps beside a repository's own file.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
Were a pair to meet anyway, an older Fleet refuses a `manifest` piece as a body it cannot read.
**The event stream.** Untouched: nothing here publishes an event.

## A setup Session keeps the person's own armada.yml: `save_kit_manifest`

Since 19 Oct 2026. `docs/concepts/manifest.md`, *Kit's copy*; `docs/design/who-sees-each-piece-of-configuration.md`, *Every piece takes the same places*.
**Additive only**: one operation, one route, and the two types it carries.
The protocol ID moves with `crates/ipc/src`, the operation file, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/sharing.ts`, written by hand, field for field, and Bridge sends it nowhere.

| Where | Side | Carries |
| --- | --- | --- |
| `save_kit_manifest` (`POST /repositories/kit_manifest`, `agent_access = "Yes"`) | A setup Session's agent sends, Fleet answers 200 | `SaveKitManifest` `{ repository, text }`; answers `KitManifestSaved` `{ repository, file }`. 422 `fleet.kit_manifest_elsewhere`, 422 `fleet.kit_manifest_refused`, 422 `fleet.kit_manifest_unplaced`, 404 `fleet.run_memory_repository_not_served`, 500 `fleet.kit_manifest_unwritable` |

**Fleet writes it, so the file gate stays as it is.**
A Session in a repository with no `armada.yml` yet writes only that file, `.gitignore` and `.git/info/exclude` under the repository's root, and Kit's copy may be in a synced folder or a bucket.
The door offers the tool to such a Session beside `record_plan`, and to every other agent inside a Manifest as it offers any `Yes` row.

**Through the door, only for the calling Job's own repository.**
A person's own copy wins over the repository's and can replace a Check, so an agent that could keep one for another repository could weaken that repository's gate.
The Job is placed by its connection, and any other root is refused with 422 `fleet.kit_manifest_elsewhere`, as is a call the door cannot place on a Job.
Bridge's call carries no connection and may keep one for any repository Fleet serves.

**It follows the place, and Keep on this Mac.**
The text is written through `KitManifest::write`: to the folder, synced folder or bucket the repository's tiles are kept in, and to `~/.armada` on this Mac while the repository is marked Keep on this Mac.
A copy already there is replaced whole.

**The text is checked first, and a refusal says why in plain words.**
It is a whole Manifest, or a copy naming only the Checks and Commands it changes: a mapping holding only `checks` and `commands`, each entry with a `run` line.
Anything else is refused with a message that starts *Nothing was saved.*, and a place that would not answer is refused before it is written to.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** Untouched: nothing here publishes an event.

## A Check only yours is added: `ManifestKitName.added`, `ManifestKitName.gates`

Since 20 Oct 2026. `docs/concepts/manifest.md`, *Kit's copy*.
**Additive only**: two optional fields on an existing type; no operation, route, event kind or error.
The protocol ID moves with `crates/ipc/src` and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/setup.ts`, written by hand, field for field.
Bridge draws nothing from it yet.

| Where | Side | Carries |
| --- | --- | --- |
| `ManifestKitName` | Fleet writes, Bridge reads | Gains `added`, `true` where the repository declares no such name and Kit's added it; absent otherwise |
| `ManifestKitName` | Fleet writes, Bridge reads | Gains `gates`, whether a Check is part of a gate: on an added Check, and on each Check of a copy that is the whole Manifest; absent on a Command and on a Check that replaces the repository's |
| `ManifestFromKit.not_read` | Fleet writes | Always absent now: a name the repository does not declare is in `names` with `added` |

**An added Check gates only where Kit's copy writes `gate: true` on it.**
A Check Kit's replaces by name gates as the repository's did, and `gate` on it is refused with the rest of Kit's copy.
Neither field is sent as `null`.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** Untouched: nothing here publishes an event.

## Commit to repository starts a Job of its own: `commit_to_repository`

Since 21 Oct 2026. `docs/concepts/kit.md`, *Commit to repository*, and `.claude/decisions/2026-10-21-commit-starts-a-job-of-its-own.md`.
**Not additive**: a field leaves a body, an error code goes, and fields are added to an answer.
The protocol ID moves with `crates/ipc/src` and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/committing.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `CommitToRepository` | Bridge sends | Loses `session_id`; the body refuses it as an unknown field |
| `CommittedToRepository` | Fleet answers | Gains `session_id`, the Job made for this commit, and its own `branch` |
| `CommittedToRepository` | Fleet answers | `commit` is always present now |
| `CommittedToRepository` | Fleet answers | Gains `pull_request`, the address where one opened, and `pull_request_not_opened`, why not where none did |
| `fleet.commit_no_job` | Fleet refuses | Gone: no Job need be open |

**Every refusal is decided against main's tree in the checkout, before the Job is made.**
`fleet.commit_exists` now names what main holds, and also refuses a piece main holds exactly as it would be written.
`fleet.commit_refused` also refuses where no slot could be leased for the Job, which is closed again.
**A pull request that does not open is not a refusal**: the answer is a 200 with the commit on the Job's pushed branch, and `pull_request_not_opened` says why.
The Job is a hosted Session that runs no agent, and holds its pull request on its row as a claim does.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** Unchanged in kind: the new Job is published as `SessionChanged`, as every Job's start is.

## A Fleet notice is muted: `set_notice_mute`, and `notice` on a message row

Since 21 Oct 2026. **Additive only**: one operation and route, the types it carries, and one optional field on an existing row.
The protocol ID moves with `crates/ipc/src`, the operation file, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/notices.ts` and `notice` on `message` in `hosted-sessions.ts`, written by hand, field for field.
Bridge calls neither yet.

| Where | Side | Carries |
| --- | --- | --- |
| `set_notice_mute` (`POST /notices/mute`, `No`) | Bridge sends `SetNoticeMute` `{ notice, scope, muted }`, Fleet answers 200 | The same body; `scope` is `job` or `hour` with `session_id`, or `repo` with `manifest_id`; `muted: false` lifts it; 422 `fleet.notice_mute_unnamed` |
| `SessionRow` `message` | Fleet sends | Gains optional `notice` (`behind_base`), set on Fleet's own notice only |

**A mute stops the send, and is kept on disk.** A slot Fleet skipped is not marked told, so lifting the mute resumes the notice, and an hour mute ends at its time across a restart.
**Rows are stored, not rebuilt**, so a row kept before this change has no `notice` and is never read as one.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** Untouched.

## The Logbook: `get_logbook` and four acts on an entry

Since 22 Oct 2026. `docs/concepts/logbook.md`. **Additive only**: five operations and routes and the types they carry; no event kind and no field on an existing type.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/logbook.ts`, written by hand, field for field. Bridge calls none of them yet.

| Where | Side | Carries |
| --- | --- | --- |
| `get_logbook` (`GET /logbook?repository=&section=&state=`, `agent_access = "Yes"`) | Bridge reads | `Logbook` `{ entries }` of `LogbookEntry`. `repository` is a root, and absent the answer is the habits and this Mac's alone; `section` and `state` absent narrow nothing, so every state is listed. A root Fleet does not serve is a 404 `fleet.run_memory_repository_not_served` |
| `edit_logbook_entry` (`POST /logbook/entries/:entry_id/edit`) | Bridge sends `{ text }`, Fleet answers the entry | The old text goes on `history` with `by: "person"`, `edited` is true. 404 `fleet.no_such_logbook_entry`; 422 `fleet.logbook_entry_not_kept`, `fleet.logbook_text_refused`, `fleet.logbook_text_forgotten` |
| `pin_logbook_entry` (`POST /logbook/entries/:entry_id/pin`) | Bridge sends `{ pinned }`, Fleet answers the entry | Refused as an edit is |
| `forget_logbook_entry` (`POST /logbook/entries/:entry_id/forget`) | Bridge sends nothing, Fleet answers `{ id }` | The text, and every text it had, goes on the deny list. A second forget is a 404 `fleet.no_such_logbook_entry` |
| `teach_logbook_entry` (`POST /logbook/entries`) | Bridge sends `{ section, repository?, kind, text, paths? }`, Fleet answers the entry | Stands at once, with no Job behind it. 422 `fleet.logbook_repository_missing` for a `repo` entry naming none; teaching forgotten text lifts the forgetting |

**An id is a number for an entry Fleet keeps, and a word and a hash (`runs-…`, `machine-…`) for one read in from run memory or the machine note.** The acts refuse the second kind with `fleet.logbook_entry_not_kept`: Fleet rebuilds them on every read.
**`LogbookEvidence.says` is optional**, since a list does not read each Job's record back. `last_tripped` is the latest time a Job tripped on the entry, read off its evidence.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** Untouched.

## A Fleet notice disputed: `dispute_fleet` and the `disputed` row

Since 19 Oct 2026. `docs/concepts/session/what-fleet-tells-a-job.md`.
**Additive only**: one operation, one route, the types it carries and one thread row; no event kind and no field on an existing type.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is the `disputed` row in `packages/protocol/src/hosted-sessions.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `dispute_fleet` (`POST /sessions/dispute_fleet`, `agent_access = "Yes"`) | An agent sends, Fleet answers 200 | `DisputeFleet` `{ why, session_id? }`; answers `FleetDisputed` `{ dispute_id, review_session_id?, joined }`. Refused with `fleet.dispute_refused` for an empty `why` or a job Fleet has told nothing; a job Fleet does not host, or an ended one, is refused as `all_done` refuses it |
| `SessionRow::Disputed` (`kind: "disputed"`) | Bridge reads | `{ id, at, notice, why, spotted, review? }`: `notice` is the Fleet message row's id, `spotted` is `called` or `read`, `review` the review job's session id, absent where none was started |

**The review job's title is `Disputed: <a few words>`**, named by one `models.dispute` call, or `Disputed notice` where that call names nothing usable.
**A review job Fleet starts carries `started_by`, naming the disputing job, and that job `started_to`**, as a job an agent started does; no `started` row is written, since the `disputed` row names the review.
**The review's brief is words, not wire**: it asks for a verdict, two to four choices put to the person, and no change until the person picks.
**No review is started where `features.disputeReviews` is off**, as it ships: the dispute is kept, the row is written, and `review_session_id` and `review` are absent.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** The row is published as any other `SessionRow`; no new event kind.

## Keep on this Mac is gone: `get_kept_on_this_mac`, `set_kept_on_this_mac`

Since 21 Oct 2026. `docs/concepts/kit.md`, *A private repository*; `.claude/decisions/2026-10-21-the-place-rows-are-the-choice.md`.
**Removal only**: the two operations, the `/repositories/kept_on_this_mac` route, `KeptOnThisMac` and `SetKeptOnThisMac` in `packages/protocol/src/run-memory.ts`, and the refusal `fleet.run_place_kept_here`.
The earlier section on them stands as what the 19 Oct build did.
Bridge called neither, so nothing in it changes.

| Where | Side | Carries |
| --- | --- | --- |
| `set_run_memory_place` | Fleet answers | No longer refuses a place for a repository that was marked; the place chosen is the place kept |
| A repository's tiles and Kit's copy of its `armada.yml` | Fleet reads and writes | Follow the repository's place, `this_mac` where none was chosen |

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The store.** `run_memory_kept_here` stays in the file, unread: dropping it is a breaking migration, which every build before this one would refuse the file for.
**The event stream.** Untouched.

## Requests: collections of saved requests, environments of variables, and a send filled from them

Since 19 Oct 2026; rewritten 21 Oct 2026, before any build shipped it, so nothing here is kept for an older client.
The per-host kept header is retired: `set_request_cookie`, `forget_request_cookie`, `list_request_hosts`, `list_request_environments` and `save_request_environments` are gone, with their types and the global `~/.armada/requests.toml`.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/requests.ts` and the `requested` and `saved` rows in `hosted-sessions.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `get_requests` (`GET /requests?repository=&session_id=`, `agent_access = "Yes"`) | Bridge or an agent reads | `Requests` `{ repository, place, collections[], environments[], not_read[] }`: each collection a `RequestCollection` `{ id, name, headers[], requests[] }` and each environment a `RequestEnvironmentRead` `{ name, variables[] }`, both with `place` and `hides`; `not_read` each `{ place, why }`. 422 `fleet.requests_unplaced` |
| `save_request_collection` (`POST /requests/collections`, `No`) | Bridge sends one collection whole | `SaveRequestCollection` `{ repository?, session_id?, collection }`; answers `Requests`. 422 `fleet.request_collection_refused`, 409 `fleet.requests_unread`, 500 `fleet.requests_unwritable` |
| `save_request_environment` (`POST /requests/environments`, `No`) | Bridge sends one environment whole | `SaveRequestEnvironment` `{ repository?, session_id?, environment }`, each variable a `VariableWrite`, **the one type holding a secret's value**; answers `Requests`. 422 `fleet.request_environment_refused`, 409 `fleet.requests_unread`, 500 `fleet.requests_unwritable` or `fleet.request_secret_unkept` |
| `save_request_to_collection` (`POST /requests/collections/request`, `Yes`) | An agent sends | `SaveRequestToCollection` `{ session_id?, collection, request }`, `collection` an id or a name and `request` a `RequestToSave` `{ name, method, url, headers[], body? }`; answers `RequestSaved` `{ collection_id, request_id }` |
| `send_request` (`POST /requests/send`, `Yes`) | Bridge or an agent sends, Fleet answers 200 | `SendRequest` `{ session_id?, repository?, saved? \| method, url, collection?, environment?, headers[], body? }`, `saved` being `{ collection, request }`; answers `RequestSent` `{ url, status?, ms, headers[], body, truncated, error?, note? }`. 422 `fleet.request_refused`, 500 `fleet.request_secret_unkept` or `fleet.request_unsent` |
| `SessionRow::Requested` (`kind: "requested"`) | Fleet writes, Bridge reads | `{ id, at, by, method, url, environment?, headers?, body?, status?, ms, error?, response, truncated? }`, `by` one of `owner`, `agent`; `cookie_host` and `kept_header` are gone |
| `SessionRow::Saved` (`kind: "saved"`) | Fleet writes, Bridge reads | `{ id, at, name, collection, collection_id, request }`, `request` the saved request's id |

**Two files per repository, the same type at either place.** The person's own is `requests/<key>.toml` at the repository's place, as `get_repository_flight_controls` reads its file, and `place` says which.
The repository's is its committed `.armada/requests.toml`, which no save writes.
Both are `RequestsFile`, version 1, spelled by hand as `[[collection]]`, `[[collection.requests]]` and `[[environment]]` with `variables`.
The person's own wins by collection `id` and environment `name`, is marked `hides`, and the repository's it hides is not listed.

**A file that will not read is never drawn as empty.** It is named in `not_read` and none of its items are listed.
A save while the person's own will not read is a 409 `fleet.requests_unread`, so it is never written over.

**A variable is plain or secret, and a secret's value is in no file and no answer.**
A file spells a secret as its name and `secret = true`, and one spelling a value does not read.
`VariableRead` answers a plain one's `value` and a secret's `set`, and has no field for a secret's value.
`VariableWrite` carries it once, Bridge to Fleet, into the Keychain; a secret sent without one keeps the one kept, and a variable left out is forgotten with its Keychain item.
Its `Debug` is written by hand and prints no value.
A commit or package of the file carries a secret's name and never its value.

**A send fills every `{{name}}` in Fleet**, in the address, the headers and the body, from the environment named, secrets included.
A saved request goes with its collection's headers under its own; its own request takes the named `collection`'s the same way.
A `{{name}}` that is not filled is sent as written, and `note` names it.
**Everything answered and written carries a secret as `{{name}}`**: the answer, the row, the told words and any error.
A value shorter than six characters is not hidden, as a kept cookie's was not, so a short value is not matched inside ordinary words.

**Who sent it is the door it came through.** A request carrying Bridge's or the phone's caller header is the owner's; the agent's door, Helm's and a request naming nobody are an agent's.
An agent's answer drops `set-cookie` and is cut at 32 KiB, and the agent is not told what it already holds.
An owner's send with a Job open writes the row and tells that Job's agent, in the shape a run's end is told.
An agent's `save_request_to_collection` writes a `saved` row and tells the agent nothing.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
Were a pair to meet anyway, a Fleet without the routes answers 404, which Bridge words as a read or send that failed.

**The event stream.** A `requested` or `saved` row is published as `session.row` like any other, and nothing new.
The open risk below is unchanged by this rewrite: a `requested` row still carries up to 16 KiB of answer and 64 KiB of request body, and a `saved` row is small.

## One file a Job wrote, as its change: `get_session_file_change`

Since 22 Oct 2026. `docs/concepts/fleet.md`, *Write clashes*, has the write table it checks against.
**Additive only**: one operation, one route and the type it answers; no event kind, no error on an existing operation and no field on an existing type.
The protocol ID moves with `crates/ipc/src`, the operation file, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `SessionFileChange` in `packages/protocol/src/territory.ts`, written by hand, field for field.
Bridge's call is `ClashesHost.fileChange` in main; no channel, preload entry or screen reads it yet.

| Where | Side | Carries |
| --- | --- | --- |
| `get_session_file_change` (`GET /sessions/file_change?session_id=&path=&seen=`, `Bridge only`) | Bridge asks, Fleet answers 200 | `SessionFileChange` `{ stamp, diff, cut?, unchanged? }`. 404 `fleet.no_such_session`, 409 `fleet.session_no_worktree`, 409 `fleet.session_path_not_edited`, 500 `fleet.session_file_change_unread` |

**Only a path the write table credits the Job with is read**, and git is not asked for any other. The read is one literal path's `git diff` against the working tree, never the tree whole.
**`stamp` is opaque.** Sent back as `seen`, an equal one is answered `unchanged` with an empty `diff`.
**Bridge asks on a `session.edited` entry naming the file, never on a timer.** The operation's notes carry the cost.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** Untouched: the read is asked over HTTP and publishes nothing, so the open risk below is neither better nor worse.

## Where a failed Check failed: `blame` on the `check` row

Since 22 Oct 2026. `docs/concepts/cockpit.md`, *A Job open*.
**Additive only**: one optional field on an existing thread row and the type it carries; no operation, route, event kind or error.
The protocol ID moves with `crates/ipc/src` and `packages/protocol/src`, and there is no number to take.
Bridge's half is `CheckBlame` and `blame` on the `check` row in `packages/protocol/src/hosted-sessions.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `SessionRow::Check` (`kind: "check"`) | Fleet writes, Bridge reads | Gains `blame`, absent while running, on a pass, and where the log names no file in the slot; never `null` |
| `CheckBlame` | Fleet writes | `{ path, line? }`: `path` relative to the slot the run stood in, `line` 1-based |

**Read once, as the run ends, from the log `end` carries**, its last 64 KiB, by `adapters::check_blame`: rustc's `-->` under an error, a Rust panic's location, vitest's `❯` frames and `FAIL` header, a node `at` frame, and tsc's two spellings.
A source file wins over a test file wherever the log names both, and only a file on disk in the slot is kept.
The row is stored as written, so no read of the thread parses a log, and a row kept before this change has no `blame`.
**The Field draws its blame line from `blame` alone**; the run's `target` is no longer read for it, so a row with no `blame` draws no line.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** The row is published as `session.row` and the Session as `session.changed`, as before; a failed row grows by one short path.
The open risk below is neither better nor worse.

## Not a question: `misread` on `dismiss_waiting`

Since 23 Oct 2026. `docs/concepts/session/waiting-on-the-person.md`, *A question that asks nothing*.
**Additive only**: one optional field on an existing body; no operation, route, event kind or error.
The protocol ID moves with `crates/ipc/src` and `packages/protocol/src`, and there is no number to take.
Bridge's half is `misread` on `DismissWaiting` in `packages/protocol/src/sessions.ts`, written by hand.

| Where | Side | Carries |
| --- | --- | --- |
| `DismissWaiting` (`POST /sessions/waiting/dismiss`) | Bridge writes, Fleet reads | Gains `misread`, sent only as `true`; absent reads as `false` |

**Only an item Fleet read from a reply takes it**: on a `replied` item Fleet drops the item as before and keeps its text in `reply_misreads`; on any other source the dismiss is refused and the item stands.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** Untouched: the Session is published as `session.changed` as on any dismiss, so the open risk below is neither better nor worse.

## A question: `confirm_waiting` and `confirmed`

Since 23 Oct 2026, the thumbs up beside *Not a question* above.
**One operation and one field**: a new route, and an optional field on an existing type; no event kind or error.
Bridge's half is `ConfirmWaiting` and `confirmed` on `WaitingItem` in `packages/protocol/src/sessions.ts`, written by hand.

| Where | Side | Carries |
| --- | --- | --- |
| `ConfirmWaiting` (`POST /sessions/waiting/confirm`) | Bridge writes, Fleet reads | `session_id` and `item_id`; answered with the Session's record |
| `WaitingItem.confirmed` | Fleet writes, Bridge reads | `true` on a `replied` item whose text is in `reply_confirmed`; sent only as `true` |

**Only an item Fleet read from a reply takes it**: on a `replied` item Fleet keeps its text in `reply_confirmed` and the item stays; on any other source the confirm is refused and the item stands unconfirmed.

**Skew.** As above.
**The event stream.** The Session is published as `session.changed`, so the open risk below is neither better nor worse.

## Open questions

Naming these rather than deciding them, per this document's brief:

- **[protocol-codegen]** What generates the TypeScript from `ipc`. Hand-rolled build script,
  `ts-rs`, `specta`, something else — not decided. Whatever it is, it must not
  reach `core-model` or `adapter-traits` (their `cargo tree` is a gate rule:
  no codegen framework belongs under either).
- **[verify-protocol-task]** What checks the rest of the generated half. The
  protocol ID is held by an `xtask` rule that hashes the wire both ways; the
  DTO types are generated by nothing, so there is no candidate output to
  compare the checked-in ones against. Whether the remainder
  is a rule in `verify-foundations` or a `verify-protocol` task of its own
  follows from what `[protocol-codegen]` decides, and neither is decided.
- **[broadcast-capacity]** The bounded broadcast channel's capacity, and whether it's one number
  for all event types or tuned per event type.
- **[lifeboat-router]** Whether the lifeboat's four routes live inside the
  same `axum` `Router` as the main protocol or a separate one. Either can satisfy "no shared
  dependency with the versioned protocol"; which one hasn't been decided.
