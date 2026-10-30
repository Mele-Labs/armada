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
| [Protocol 23.93: a Job's shared terminal](protocol/protocol-23-93-a-jobs-shared-terminal.md) |
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
| [A Job's missing tools](protocol/a-jobs-missing-tools.md) |
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
| [Saved issue queries and the query writer](protocol/saved-issue-queries-and-the-query-writer.md) |
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
| [A Job starts on its start card](protocol/a-job-starts-on-its-start-card.md) |
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
| [The owner's notes on a pull request's lines: `list_pull_request_notes` and four more](protocol/the-owners-notes-on-a-pull-requests-lines.md) |
| [A Job's MCP servers: `get_session_mcp`, `act_on_session_mcp`](protocol/a-jobs-mcp-servers.md) |
| [A background act's failure: `act.failed`](protocol/a-background-acts-failure-act-failed.md) |
| [Kit's Add and Remove: a plugin or a skill](protocol/kit-add-and-remove-a-plugin-or-a-skill.md) |
| [The shared home: Kit's rows against it, three acts, and what syncs](protocol/the-shared-home.md) |
| [A Mod Job: `start_mod_session`, `mod` and `made_by`](protocol/a-mod-job.md) |
| [An adversarial review of a plan: `start_adversarial_review`, `send_finding`, `dismiss_finding`](protocol/an-adversarial-review-of-a-plan.md) |
| [The agents' account signed out: `agent_sign_in.changed`, `get_agent_sign_in`, `start_agent_sign_in`](protocol/the-agents-account-signed-out.md) |
| [A repository's app, run from Bridge: `list_app_runs`, `start_app_run`, `app_run.changed` and four more](protocol/a-repositorys-app-run-from-bridge.md) |
| [What a workflow lands in: `WorkflowSummary.lands`](protocol/what-a-workflow-lands-in.md) |
| [How a Job started, and when each step began: `SessionRecord.started`, `SessionRecord.workflow_visits`](protocol/how-a-job-started-and-when-each-step-began.md) |
| [Projects: tasks and linked issues, across repositories](protocol/projects.md) |
| [The Workflows page: `get_workflow_steps`, `delete_workflow_copy`, `reset_workflow_copy`, `mark_not_a_drift`](protocol/the-workflows-page.md) |
| [Triggers leave the wire](protocol/triggers-leave-the-wire.md) |
| [The pull request a plan's group lands in: `lands_in`](protocol/the-pull-request-a-plan-groups-lands-in.md) |
| [The Job browser: seven acts held for Bridge, `answer_browser_request`, and the `browser_reminder` row](protocol/the-job-browser.md) |
| [Several Jobs started in one call: `jobs` on `start_another_session`](protocol/several-jobs-started-in-one-call.md) |
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

**Bridge's call** is `SessionsHost.sendNote` in main, for a note on a Job's browser; Bridge's own layer sends its notes as one turn (`protocol/a-walks-notes-sent-as-one-turn.md`).
It folds `NoteSent.session` into the held Sessions as a message's answer is, and hands the candidates back to the caller.
An empty array on the wire is the same as an omitted one, so the mirror marks each as optional.

**What the agent reads** is his words, a blank line, then a block headed as where he pointed and not the request.
The block tells the best three candidates, the tabs of `selected` and no button, and the nearest three of the component chain.
No candidate is no line, so a note nothing matched does not say so; the answer still carries all five, which Bridge shows.

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
| `commit_to_repository` (`POST /repositories/commit`, `Bridge only`) | Bridge sends, Fleet answers 200 once committed | `CommitToRepository` `{ repository, session_id?, piece, overwrite?, remove? }`; answers `CommittedToRepository` `{ repository, branch, paths[], commit? }`. 422 `fleet.commit_no_job`, `fleet.commit_not_yours`, `fleet.commit_exists`, `fleet.commit_refused`; 404 `fleet.run_memory_repository_not_served`; 500 `fleet.commit_unwritable` |
| `PieceToCommit` | both | tagged by `kind`: `flight_controls` `{ job[]?, deck[]? }` (tile ids), `workflow` `{ workflow_id }`, `review_tool` `{ id }` |
| `PieceToCommit` | both | Gains, since 30 Oct 2026, `lifecycle` `{ id, scope? }`, `lifecycle_rule` `{ id, scope? }`, `workspace` `{ id }` and `autonomy` `{ id, scope? }`, each the person's own Kit piece selected by the id and scope its file declares, and `remove` on the body to take one out of the repository; see *Workspaces, lifecycles and rules, saved* |
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

**Share does not carry Workspaces, lifecycles, rules or autonomy defaults yet.** `share_package` with one of those `PieceToCommit` kinds is a 422 `fleet.share_refused` saying so, and the package text has no piece for them.
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

**It follows the place.**
The text is written through `KitManifest::write`: to the folder, synced folder or bucket the repository's tiles are kept in, and to `~/.armada` on this Mac where that place is This Mac or the repository, or none was chosen.
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
**The Job's commit can also be a deletion.** `remove` on the body, since 30 Oct 2026, deletes the file instead of writing one: `committed_in_removing` removes each path from the Job's tree and hands `commit_paths` every path, and a path the tree no longer holds is committed as a deletion by that call (`a_deleted_path_and_an_untracked_one_are_both_committed` in `adapters`).
`paths` in the answer then lists what was removed.

## A Fleet notice is muted: `set_notice_mute`, and `notice` on a message row

Since 21 Oct 2026. **Additive only**: one operation and route, the types it carries, and one optional field on an existing row.
The protocol ID moves with `crates/ipc/src`, the operation file, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/notices.ts` and `notice` on `message` in `hosted-sessions.ts`, written by hand, field for field.
Bridge calls neither yet.

| Where | Side | Carries |
| --- | --- | --- |
| `set_notice_mute` (`POST /notices/mute`, `No`) | Bridge sends `SetNoticeMute` `{ notice, scope, muted }`, Fleet answers 200 | The same body; `scope` is `job` or `hour` with `session_id`, or `repo` with `manifest_id`; `muted: false` lifts it; 422 `fleet.notice_mute_unnamed` |
| `SessionRow` `message` | Fleet sends | Gains optional `notice`, set on Fleet's own notice only. Every notice Fleet writes to a Session carries its word (`behind_base`, `pr_checks_failed`, `write_clash` and the rest of `crates/ipc/src/notices.rs`); a row of its own kind (`linked`, `moved`, `started`) carries none |

**A mute stops the send of every notice, and is kept on disk.** `told_by_fleet` reads it for any word, before a Fleet mod's rule.
A notice it holds back is not marked told, so lifting the mute resumes it, and an hour mute ends at its time across a restart.
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
| `lock_logbook_entry` (`POST /logbook/entries/:entry_id/lock`) | Bridge sends `{ locked }`, Fleet answers the entry | Refused as an edit is |
| `forget_logbook_entry` (`POST /logbook/entries/:entry_id/forget`) | Bridge sends nothing, Fleet answers `{ id }` | The text, and every text it had, goes on the deny list. A second forget is a 404 `fleet.no_such_logbook_entry` |
| `teach_logbook_entry` (`POST /logbook/entries`) | Bridge sends `{ section, repository?, kind, text, paths?, source? }`, Fleet answers the entry | Stands at once, with no Job behind it; `source` `{ session_id, cite }` is what Teach was pressed on and becomes its one evidence row. 422 `fleet.logbook_repository_missing` for a `repo` entry naming none; teaching forgotten text lifts the forgetting |

**An id is a number for an entry Fleet keeps, and a word and a hash (`runs-…`, `machine-…`) for one read in from run memory or the machine note.** The acts refuse the second kind with `fleet.logbook_entry_not_kept`: Fleet rebuilds them on every read.
**`LogbookEvidence.says` is optional**, since a list does not read each Job's record back. `last_tripped` is the latest time a Job tripped on the entry, read off its evidence.
**`LogbookEntry.title` is optional**, added 28 Oct 2026: a few words Distil writes with the entry, absent on one written before titles were until a later Distil run gives it one.
A Bridge that predates it ignores the field.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** Untouched.

## Teaching: `read_teaching`, `save_teaching` and `list_word_rules`

Since 22 Oct 2026. **Additive only**: three operations and routes and the types they carry; no event kind and no field on an existing type but `taught_on` on `ModSummary` and three `ModKind` words.
Since 23 Oct 2026 the words part's `address` is `{ template, found }` and not a string (the one read of it is Bridge's, and the IDs differ), `WordRule` gains `found`, `ModSummary` gains `entry`, and `ModChecked` gains `words` and `fleet`.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Since 23 Oct 2026, again, a Fleet rule gains `matches` and `when` and the proposal's `fleet` part gains `untestable`, all optional and left out when absent.
Since 30 Oct 2026 a proposal gains a sixth part, `rule`, a lifecycle rule with `untestable`, `TeachingSourceKind` gains `ticket`, and `SavedTeaching` gains `rule`, the rule piece written; all optional, and no number to take.
Bridge's half is `packages/protocol/src/teaching.ts` and the `ModKind` union in `packages/protocol/src/mods.ts`, written by hand, field for field.
Its calls are `readTeaching`, `saveTeaching` and `listWordRules` (`apps/desktop/src/shared/api/teaching.ts`, served by `apps/desktop/src/main/teaching.ts`); `read_teaching` waits as a model call does.
**A teaching is a Logbook entry for the agent note, mods by what it changes and a rule piece for a rule**; `save_teaching` makes all three, so Bridge files nothing itself.
`docs/concepts/mods.md` is the mod files.

| Where | Side | Carries |
| --- | --- | --- |
| `read_teaching` (`POST /teachings/read`) | Bridge sends `{ words?, message?, said, repository?, reach, session_id?, source? }` (`reach` is `repository` or `everywhere`, `source` is `{ kind, text, ids? }` with `kind` one of `words`, `command`, `notice`, `task`, `check`, `pull_request`, `ticket`, `panel`, `row`, `title_bar`), Fleet answers `{ note?, words?, fleet?, tweak?, layout?, rule? }` | One model call, nothing kept. `note` is `{ section, kind, text }`, `words` is `{ name, pattern, example, address? }` with `address` `{ template, found }` (`found` is `settings`, `pull_request #N` or `person`) for the request's repository where the person wrote one or Fleet found one, `fleet` is `{ notice, does, where, matches?, when?, untestable? }` (`when` is a list of `{ if, does }` with `if` one of `blocks_merge`, `open_pull_request`, `working`, `owner_away`; `untestable` is the person's own words for a condition outside those), `tweak` is a list of `{ token, was, now, used_in?, note? }`, `layout` is `{ name, layout }`, `rule` is a lifecycle rule as `list_lifecycle_rules` answers one with `untestable?` beside it (the person's own words for a condition a rule's `if` cannot hold; Fleet gives the `scope` from the reach and the `id` from the name, and holds the rule to what a rule piece is held to when it is read); at least one is present. 422 `fleet.teaching_words_refused`, `fleet.teaching_note_refused`, `fleet.teaching_rule_refused`, `fleet.teaching_tweak_refused`, `fleet.teaching_layout_refused`, `fleet.teaching_input_refused`, `fleet.teaching_nothing_read`; 500 `fleet.teaching_read_failed` |
| `save_teaching` (`POST /teachings`) | Bridge sends the parts as left, then `{ reach, repository?, session_id?, cite?, source? }`, Fleet answers `{ entry?, mods, rule? }` | Held to what `read_teaching` holds a proposal to, all before any write. The note becomes a Logbook entry with `{ session_id, cite }` as its evidence; each other part becomes an enabled mod, except a `rule`, which is not a mod and becomes a rule piece in Kit (`rule` in the answer is the `save_lifecycle_piece` answer for it). With `session_id` and a note the Session is told "From now on: <note>": queued behind a turn in progress, never refused |
| `list_word_rules` (`GET /teachings/words?repository=&session_id=`, `agent_access = "Yes"`) | Bridge reads | `{ rules }` of `{ mod_name, name, pattern, example, address, found }`: one per enabled, valid Words mod with an address for the repository, its own before the one for every repository. 404 `fleet.run_memory_repository_not_served` |

**`read_teaching` looks for the address first** when it proposes words and a repository is given: a Words mod with the same pattern that already has one, then the two sources below, the pull request search held to a ten second bound.
`save_teaching` writes the address the proposal carries, `found` as it came.
**A Words mod with no address for a repository is looked for**, off the request, when a save without one names the repository or `list_word_rules` is read for it: the Manifest's tracker first (`found = "settings"`, through `Tracker::address_template`), then up to five recently merged pull requests' text (`found = "pull_request #N"`).
**Where neither says, the Session is asked once**: `WaitingSource` gains `words_address`, a derived item with id `words:<mod>:<manifest id>`, text `Where does <name> open in <manifest id>?`, an `answer` act, and options of each address any Words mod holds (label the address, description the repositories using it) then `Other`.
`answer_waiting` with a `choice` of an address, or `Other` and `text`, writes it to the mod as `found = "person"`; 422 `fleet.session_waiting_empty` for an address with no `{match}`.
Dismissing it is kept: it is not asked again for that mod and repository.
**Skew for `words_address`.** A Bridge before this reads an item of a source it does not know; the IDs differ and the lifeboat serves.

**A Fleet rule that says `call` raises `WaitingSource::FleetCall`**, a derived item with id `call:<n>`, the words the Session was told as its text and a `message` act that opens the Job.
Dismissing it settles it and tells the agent nothing.
A Bridge before this reads an item of a source it does not know; the IDs differ and the lifeboat serves.

**Removed**: `list_taught_links` and `forget_taught_link`, with the `taught_links` table, which never shipped. Forgetting a Logbook entry removes the mods whose `mod.toml` records it as `entry`.
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

## A fault in Armada reported: `report_bug`, `list_bug_reports`, `keep_bug_report`, `drop_bug_report` and `bug_reports.changed`

Since 29 Oct 2026. `crates/ipc/src/bug_reports.rs`.
**Additive only**: four operations, four routes, one event kind and the `bug_reports` table; no field on an existing type.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/bug-reports.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `report_bug` (`POST /sessions/report_bug`, `agent_access = "Yes"`) | An agent sends, Fleet answers 200 | `ReportBug` `{ title, what, steps?, errors?, logs?, session_id? }`; answers `BugReported` `{ report_id }`. Refused with `fleet.bug_report_refused` for an empty `title` or `what`; a call that places no job is refused as `all_done` refuses it |
| `list_bug_reports` (`GET /bug_reports`) | Bridge sends, Fleet answers 200 | `BugReports` `{ reports }`, newest first |
| `keep_bug_report` (`POST /bug_reports/:report_id/keep`) | Bridge sends, Fleet answers 200 | No body; answers the `BugReport` with `kept` set. 404 `fleet.no_such_bug_report` for an id that names none |
| `drop_bug_report` (`POST /bug_reports/:report_id/drop`) | Bridge sends, Fleet answers 200 | No body; answers `BugReportDropped` `{ id }`. 404 `fleet.no_such_bug_report` for an id that names none, a second drop too |
| `bug_reports.changed` (`Event::BugReportsChanged`) | Bridge reads | `BugReports`: every report on this Mac, whole, after one was made, kept or dropped |

**The report is kept in Fleet's store on this Mac and posted nowhere.** Fleet writes to no forge on the owner's behalf: no issue is opened and nothing is commented.
**The job is the one whose process holds the connection**, as `all_done`'s is, and a terminal job is served.
The job's title is kept with the report as it stood, so a report reads after its job is gone.
**`steps`, `errors` and `logs` are each trimmed per line and a blank line is left out.** They may be empty.
`title` and `what` may not.
**`kept` is when the owner kept the report and is absent until then.** Keeping one again keeps the first time.
Whether he has read one is Bridge's own and is not on the wire.
**The store.** `bug_reports` is additive, `crates/store/migrations/20261029T1600Z-bug_reports.table.sql`, with no foreign key: a report outlives its job.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** `bug_reports.changed` is a new kind; a Bridge that missed one reads `list_bug_reports` on the resync that follows a `Missed`.

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
| `delete_request_environment` (`POST /requests/environments/delete`, `No`) | Bridge sends one environment's name | `DeleteRequestEnvironment` `{ repository?, session_id?, name }`; answers `Requests`. 404 `fleet.no_such_request_environment` where the person's own holds none of that name, 409 `fleet.requests_unread`, 500 `fleet.requests_unwritable` or `fleet.request_secret_unkept` |
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
Deleting an environment forgets each of its secrets first, and a Keychain that will not forget one leaves the file unwritten.
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

## What the Logbook hands out mid-task: `recall` and `get_logbook_for_subagent`

Since 22 Oct 2026. `docs/concepts/logbook.md`, *Recall*.
**Additive only**: two operations, two routes and the types they answer; no event kind, no error on an existing operation and no field on an existing type.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/logbook.ts`, written by hand, field for field; no screen reads either yet.

| Where | Side | Carries |
| --- | --- | --- |
| `recall` (`GET /logbook/recall?query=&section=&manifest_id=`, `Yes`) | An agent asks, Fleet answers 200 | `Recalled` `{ entries[] }`, each `{ section, kind, state, text, confirmed_by }`. 422 `fleet.session_unnamed` for a blank `query` or a `section` that is not `repo`, `habits` or `machine` |
| `get_logbook_for_subagent` (`POST /logbook/subagent`, `No`) | The Armada mod asks, Fleet answers 200 | Body `SubagentAsk` `{ session_id, prompt }`; `ForSubagent` `{ text }`, empty where the Logbook holds nothing |

**The start note is not on the wire.** It is built in Fleet and handed to the process as `--append-system-prompt`, so the Logbook note changes no type.
**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
A mod newer than its Fleet gets a 404 from `/logbook/subagent` and hands the subagent nothing.

## Watches on the wire: `start_watch`, `weigh_watch`, `stop_watch`, `list_watches`

Since 22 Oct 2026. The design is *Fleet watches CI and deploys for a Job*, under `docs/design/`, and spike 36 is what backs it.
Four operations, four routes, one event kind and the types they carry. `weigh_watch`'s `ours` carries a `sentence` since its first build, which a body without one is refused for.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/watches.ts`, written by hand, field for field; no main-process call, channel or screen reads it yet.

| Where | Side | Carries |
| --- | --- | --- |
| `start_watch` (`POST /watches/start`, `agent_access = "Yes"`) | An agent starts one | `StartWatch` `{ session_id?, name, recipe, values? \| command[], every, gives_up_after?, follows?, set_aside[]? }`; answers `WatchSummary`. 400 for neither shape or both; 422 `fleet.watches_unplaced`, `fleet.watch_refused`, `fleet.watch_recipe_unknown`, `fleet.watch_value_refused`, `fleet.watch_too_often`, `fleet.watch_gives_up_too_late`; 409 `fleet.watch_limit_reached` |
| `weigh_watch` (`POST /watches/weigh`, `Yes`) | An agent weighs a failed reading | `WeighWatch` `{ session_id?, watch, verdict: "ours", sentence }` or `{ ..., verdict: "not_ours", sentence, parts[] }`; answers `WatchSummary`. 404 `fleet.no_such_watch`; 409 `fleet.watch_ended`, `fleet.watch_not_failed`; 422 `fleet.watch_part_unreported`, `fleet.watch_refused` |
| `stop_watch` (`POST /watches/stop`, `Yes`) | Bridge's Stop, or an agent | `StopWatch` `{ session_id?, watch }`; answers `WatchSummary`, ended `stopped`, or as it stands where it had ended. 404 `fleet.no_such_watch` |
| `list_watches` (`GET /watches?session_id=`, `Yes`) | Bridge or an agent reads | `WatchList` `{ session_id, watches[] }`, the newest first. 422 `fleet.watches_unplaced`, `fleet.no_such_session` |
| `watches.changed` (event, Bridge only) | Fleet publishes on any change to one of a Job's watches | `WatchList`, whole |
| `WatchSummary` | Fleet writes | `{ id, session_id, name, command[], recipe?, every_seconds, gives_up_after_seconds, follows?, environment?, state, live, started_at, last?, readings[]?, set_aside[]?, verdicts[]? }`; `readings` the newest 50, oldest first, `last` the newest; a verdict `{ at, verdict, sentence, parts[]? }`, `state` one of `waiting`, `running`, `passed`, `failed`, `gave_up`, `broken`, `stopped` |
| `WatchReading` | Fleet writes | `kind: "read"` `{ at, state, finished, says, link?, parts[]? }` or `kind: "broken"` `{ at, why, output? }` |

**DTOs, not `core_model::Watch`.** The domain type keeps every part a reading carried and the last reading whole, so a weighing can be held to them; neither list reaches the wire.
**The command's line is not the wire.** `ipc::read_watch_line` reads a command's last line into `core_model::Reading`, in `ipc` because untyped JSON is read there and in `store` only, and nothing serialises it back.
**A value fills one whole argument.** A recipe's item written `{sha}` is replaced by `values.sha`, and no item is read by a shell.
**The Job is the connection's** where Fleet hosts it, else the body's `session_id`, as `send_request`'s is. A placed Job weighs and stops its own watches only; Bridge places none and stops any.

**Run and kept.** The store keeps every watch, its newest 50 readings, set-asides, verdicts and the notices told, so a restart carries on (`store::KeptWatch`).
Fleet runs due watches on a 5 second timer of their own, `fleet::watches::running`.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** `watches.changed` carries a Job's watches whole, history included, on a start, a weighing, a Stop, an end and a reading that reads differently from the one before.
A reading that reads the same is published at most every 5 minutes, so Bridge's age of a reading stays near and the stream does not carry 50 readings every 30 seconds; a reading carries up to 81 parts, 5.3 KB, in spike 36.

### Setting a watch up: `try_watch`, `list_watch_recipes`, `save_watch_recipe`, and a terminal watch

Since 26 Oct 2026. Three operations, three routes and the types they carry, and four optional fields on what was there.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/watches.ts`, written by hand; nothing in Bridge reads the new types yet.

| Where | Side | Carries |
| --- | --- | --- |
| `try_watch` (`POST /watches/try`, `Yes`) | An agent runs one reading | `TryWatch` `{ session_id?, recipe, values? \| command[] }`; answers `WatchTried` `{ reading, command[], cwd, path, login_shell, scratch }`. 400 for both shapes, neither, or any other key; 422 `fleet.watches_unplaced`, `fleet.watch_recipe_unknown`, `fleet.watch_value_refused`, `fleet.watch_refused` |
| `list_watch_recipes` (`GET /watches/recipes?session_id=`, `Yes`) | An agent reads the recipes | `WatchRecipeList` `{ recipes[], left_out[]? }`, each `{ name, values[], command[], source, every_seconds, gives_up_after_seconds?, environment? }`, `source` one of `armada`, `repository`, `machine`. 422 `fleet.no_such_session` |
| `save_watch_recipe` (`POST /watches/recipes/save`, `Yes`) | An agent keeps a recipe on this Mac | `SaveWatchRecipe` `{ session_id?, name, script?, command[]?, every, gives_up_after?, environment?, replace? }`; answers `WatchRecipeSaved` `{ recipe, file, script?, replaced }`. 409 `fleet.watch_recipe_shipped`, `fleet.watch_recipe_exists`; 422 `fleet.watch_refused`, `fleet.watch_too_often`, `fleet.watch_gives_up_too_late`; 500 `fleet.watch_recipe_unwritten` |
| `StartWatch` | Gains a third shape | `{ ..., in_terminal, regex?, gives_up_after? }`, a terminal watch. 409 `fleet.watch_terminal_closed` where the Job has no terminal open |
| `WatchSummary` | Gains three fields | `in_terminal?`, `regex?` (sent only as `true`), and `login_shell?`, `{ kind: "read" }` or `{ kind: "fell_back", why }`, while Fleet holds the watch's environment |

**The command's line is written by Armada too.** `ipc::watch_line` writes the line `armada watch-line` prints for the recipes Armada ships, and `ipc::read_watch_line` reads it back whole; it is still not the wire to Bridge.
**Always loaded.** An operation file may say `always_loaded = true`; the door then lists that tool with the harness's `_meta` key, which the harness adapter names (`AgentHarness::always_loads_tool`).
The key is on the agent's tools list only, and Bridge reads nothing of it.

### Tidying a Job's watches: a run of readings, ended watches on asking, and `delete_watch`

Since 26 Oct 2026. The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.

| Where | Side | Carries |
| --- | --- | --- |
| `WatchReading` | Fleet writes | Gains `count?` and `last_at?` on both kinds. In `readings`, back-to-back readings that read the same are one: `at` the first, `last_at` the last, `count` how many, absent for one. A read is the same in `state`, `finished`, `says`, `link` and `parts`; a broken one in `why`, with the last one's `output` |
| `WatchSummary.last` | Fleet writes | The newest reading as taken, never a run, so its `at` is its own |
| `delete_watch` (`POST /watches/delete`, `Yes`, always loaded) | Bridge's delete control, or an agent | `DeleteWatch` `{ session_id?, watch }`; answers the Job's `WatchList` after, live and ended, and publishes `watches.changed`. The store forgets the watch, its readings, set-asides, verdicts and notices. 409 `fleet.watch_live` for a live one, "Stop it first."; 404 `fleet.no_such_watch` |
| `list_watches` (`GET /watches?session_id=&ended=`) | Bridge or an agent reads | Live watches only, unless `ended=true`. Any value but `true` or `false` is a 400 `api.undecodable_request`. Bridge asks for ended ones, as `watches.changed` carries them whole |

**The store keeps every reading.** Only the answer collapses them, in `ipc::collapsed`.

### A watch that ended not ours: `not_ours` and `watch_not_ours`

Since 26 Oct 2026. The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.

| Where | Side | Carries |
| --- | --- | --- |
| `WatchState` (`WatchSummary.state`) | Fleet writes | Gains `not_ours`: every counted part passed while a part set aside still reads `failed`. An end once the reading is finished; never `passed`. The readings keep their own states |
| `WaitingSource` (`WaitingItem.source`) | Fleet writes | Gains `watch_not_ours`: id `watch_not_ours:<watch id>`, text `<watch>: <parts> failed`, act `answer` whose `target` is the watch id, options `Ignore` (choice 0) and `Fix it` (choice 1). Derived on every read from the store's watch, while the Job is live |
| `answer_waiting` on it | Bridge writes | `Ignore` settles it and sends nothing; `Fix it` sends the agent the parts, each sentence and the link, then settles it; words alone are a message to the agent. `dismiss_waiting` settles it too |

| `weigh_watch` `not_ours` | An agent writes | Gains `evidence?`, `{ links[]?, logs[]?, errors[]? }`: links to what was watched, log files the agent saved, and the exact error lines. Refused with `ours`. Kept with the verdict, blank entries dropped |
| `read_watch_log` (`GET /watches/log?watch=&log=`, `agent_access = "No"`) | Bridge reads | `ReadWatchLog` `{ watch, log }`; answers `WatchLog` `{ watch, log, text }`, the file whole. Only a path a kept reading of that watch carries is read; any other is a 404 `fleet.no_such_watch_log`. Bridge's main process opens only files a Job's ledger names, so the side panel reads a log through Fleet |
| `WatchVerdictGiven.evidence` | Fleet writes | The same, on the verdict as kept; absent where it carried none |
| `WatchPart` | Fleet writes | Gains `link?`, `error?` and `tail?`: where to look at the part, the exact lines it failed on, and the last lines of its own log, as the command's line gave them. The line's `parts[]` takes the same three, so a hand-written command can give them |
| `WatchReading.log` | Fleet writes | Both kinds gain `log?`: the path of a file under Fleet's state holding the whole stdout and stderr of a run that failed or broke, as much as the run captured (the last 64 KiB of each stream), its first line saying when either was cut. A run of readings carries its last one's. Deleting the watch, or the Job ending, deletes the file |

**No new field on the item.** Bridge reads the watch from `watches.changed` by the act's `target`, so Escalate's summary is drawn from what the session already holds and nothing reaches Fleet.
**The event stream.** No new event: the Job's row is published again when a watch ends, a weighing that ends one included, so the item arrives with it.

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

## A closed item is kept whole: `closed`, `reopen_waiting` and `list_closed_waiting`

Since 29 Oct 2026. `docs/concepts/session/waiting-on-the-person.md`, *A closed item is kept whole*.
**Two operations and one field**: a command Bridge sends, a query an agent reads, and a list on the Session's record; no event kind.
Bridge's half is `ClosedWaiting`, `ClosedWaitingList`, `ReopenWaiting` and `closed` on `SessionRecord` in `packages/protocol/src/sessions.ts`, written by hand.

| Where | Side | Carries |
| --- | --- | --- |
| `SessionRecord.closed` | Fleet writes, Bridge reads | `ClosedWaiting` `{ item, at, ended? }`, newest first, at most 50: `item` the `WaitingItem` as it stood, `at` when it was closed, `ended` sent only as `true`. Absent when none |
| `reopen_waiting` (`POST /sessions/waiting/reopen`, `agent_access = "No"`) | Bridge sends, Fleet answers 200 | `ReopenWaiting` `{ session_id, item_id }`; answered with the `SessionRecord`. 409 `fleet.session_waiting_unheld` for an id nothing closed holds; 422 `fleet.session_waiting_unreopenable` for one with `ended` |
| `list_closed_waiting` (`GET /sessions/waiting/closed?session_id=`, `agent_access = "Yes"`) | An agent reads | `ClosedWaitingList` `{ session_id, closed[] }`, the same entries. 422 `fleet.session_waiting_unnamed` for a call that places no Job |

**Kept at the close.** `dismiss_waiting` writes the item's snapshot into new nullable columns of `session_waiting_dismissed` (additive, `20261029T0100Z-session_waiting.closed`).
The id stays dismissed until `reopen_waiting` deletes the row.
**`ended`** is read on every record by lifting every closed item's dismissal in a second pass over the same reads and asking which ids come back; the agent's own items are reopenable while the Job lives.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** The Session is published as `session.changed` on a close and a reopen, as on any dismiss.
The record gains up to 50 small entries, so the open risk below is neither better nor much worse.

## The Job open when a note was pinned: `SessionNote.job`

Since 23 Oct 2026. The pinned note above, `docs/concepts/session.md`.
**One optional field on an existing type**: no operation, route, event kind or error.
Bridge's half is `job` on `SessionNote` in `packages/protocol/src/session-notes.ts`, written by hand.

| Where | Side | Carries |
| --- | --- | --- |
| `SessionNote.job` | Bridge sends | Labelled lines naming the Job open when he pinned the note: its title and id, branch and transcript; left off when empty |

They name the Job and no more: Report's messages, tool calls and timeline, carried with a note, read as the task and buried his words.
**Fleet puts them after his words and a blank line**, under a line calling them background, not the task, and before the handoff.
A batch puts them in the block of the note that carries them.
Bridge sends them on the first note of a send alone, and none to the Job that is open.

**Skew.** As above.
**The event stream.** Untouched.

## A draft withdrawn: `withdraw_draft`, and `withdrawn` on `DraftHow`

Since 27 Oct 2026. `docs/concepts/session/proposing-a-draft.md`.
**One operation and one enum value**: a new route, its body, and a value on an existing enum; no event kind and no field on an existing type.
The protocol ID moves with `crates/ipc/src`, the operation file and the route table, and with `packages/protocol/src` for Bridge's half.
Bridge's half is `"withdrawn"` in `DraftHow` in `packages/protocol/src/hosted-sessions.ts`, written by hand.

| Where | Side | Carries |
| --- | --- | --- |
| `DraftHow` (on `DraftSettled.how`, a `draft` row's `settled`) | Fleet writes, Bridge reads | Gains `withdrawn`: dropped by the person's Dismiss or the job's `withdraw_draft`. `words` is his reason, absent when he gave none and always absent for the job's own |
| `withdraw_draft` (`POST /sessions/withdraw_draft`, `agent_access = "Yes"`) | An agent writes | `WithdrawDraft` `{ draft, session_id? }`, `draft` the `draft:<row id>` or the bare row id; answers the `SessionRecord`, or through the door a receipt `{ session_id, withdrawn }`. 422 `fleet.draft_withdraw_not_open` for another job's draft, a settled one or an unknown id, with nothing written; 422 `fleet.draft_withdraw_unnamed` for a call that places no job |
| `dismiss_waiting` on a `draft:<row id>` | Bridge writes | No new field. Fleet now sends the job `Withdrawn: <title>` and `Do not propose it again.`, with the reason on a line after it where it is not blank, and settles the row `withdrawn` only once that is sent |

**A Bridge that matches `DraftHow` exhaustively** meets a value it did not know; the mirror gains it in the same change, and equal IDs hold the two together.

**Skew.** As above.
**The event stream.** No new event: either way of withdrawing publishes `session.changed`, as an answer does. The open risk below is neither better nor worse.

## A Job an agent started waits in its parent's Needs you: `held_child`, `queued` and `started`

Since 28 Oct 2026. `docs/concepts/session.md`, *A Job an agent started*.
**Two enum values and one optional field**: no operation, route, event kind or error.
The protocol ID moves with `crates/ipc/src`, the operation files and `packages/protocol/src`, and there is no number to take.
Bridge's half is `held_child` on `WaitingSource` in `packages/protocol/src/sessions.ts`, and `queued` on `StartHeld` and `started` on `LinkedEvent` in `hosted-sessions.ts`, written by hand.

| Where | Side | Carries |
| --- | --- | --- |
| `WaitingSource` (`WaitingItem.source`) | Fleet writes, Bridge reads | Gains `held_child`: id `start:<child id>`, text `Press Start on "<child title>"`, act `message` whose `target` is the child's id. Derived from a `started_to` child still held on its start card; dropped when the child starts or ends |
| `answer_waiting`, `dismiss_waiting` on it | Bridge writes | Both refused 422 `fleet.session_waiting_empty`, and the item stands: `begin_session` on the child is what settles it |
| `StartHeld.queued` (`hosted.start`) | Fleet writes, Bridge reads | `bool`, left out when false. Start was pressed while the proposer was still reading; the Job starts when the reading ends |
| `BeginSession.delivery` | Bridge writes | No new field. While the proposer is reading, the press is queued and `delivery` may be absent: the reading's landing is taken |
| `LinkedEvent` (a `linked` row's `event`) | Fleet writes, Bridge reads | Gains `started`: a Job the parent's agent started left its start card, written in the parent's thread |

**Fleet writes all three since the change after it.** The item is read off the child on every read of the parent, the queued press is in memory as the reading is, and `started` is told once per child.
**A Bridge that matches `WaitingSource` or `LinkedEvent` exhaustively** meets values it did not know; the mirror gained them first, and equal IDs hold the two together.

**Skew.** As above.
**The event stream.** No new event: the item and `queued` ride the `session.changed` a hosted Job already publishes, and `started` one more `session.row` on the parent.
The open risk below is neither better nor worse.

## A sketch approved or sent back: `answer_sketch`, and `asked`, `drawn` and `settled` on a sketch

Since 28 Oct 2026. `crates/ipc/operations/answer_sketch.toml`.
**One operation and keys of a sketch's `detail`**: a new route, its body and one enum; no event kind and no field on an existing type.
The protocol ID moves with `crates/ipc/src`, the operation file and the route table, and with `packages/protocol/src` for Bridge's half.
Bridge's half is `AnswerSketch` and `SketchHow` in `packages/protocol/src/sessions.ts`, written by hand, `answerSketch` on its Fleet client, and the `detail` keys read by `sketchOf` in `packages/screens/src/sessions-wire.ts`.

| Where | Side | Carries |
| --- | --- | --- |
| `answer_sketch` (`POST /sessions/sketch/answer`, `agent_access = "No"`) | Bridge writes | `AnswerSketch` `{ session_id, sketch, drawn, how, words? }`, `how` `approved` or `sent_back`; answers the `SessionRecord`. 409 `fleet.sketch_redrawn` for a `drawn` that is not the sketch's now; 422 `fleet.sketch_not_open`, `fleet.sketch_answer_empty` and `fleet.sketch_answer_unnamed`, with nothing written or sent |
| `detail.asked`, `detail.drawn` on an artifact of form `sketch` | Fleet writes, Bridge reads | `asked` `"true"` on every sketch `show_sketch` draws, and `drawn` an id new on every draw. A plan's sketch has neither |
| `detail.settled`, `detail.settled_at`, `detail.words` | Fleet writes, Bridge reads | How he answered, `approved` or `sent_back`, when, and his note. Gone when the title is drawn again |

**Not the agent's `waiting_for` item**: the owner answers a sketch on its card, and since 29 Oct Fleet raises it in his Needs you as `sketch` (below).
The job is told as his message, as a draft's answer is, and the rail reads an approval back off that message.

**Skew.** As above.
**The event stream.** No new event: an answer publishes `session.changed`, as a draft's does. The open risk below is neither better nor worse.

## Fleet offers a refresh of the preview: `preview_offer`, and `moved` on a left-out branch

Since 28 Oct 2026. `crates/fleet/src/session_host/preview_offers.rs`.
**One enum value and one optional field**: no operation, route, event kind or error.
The protocol ID moves with `crates/ipc/src` and `packages/protocol/src`, and there is no number to take.
Bridge's half is `preview_offer` on `WaitingSource` in `packages/protocol/src/sessions.ts` and `moved` on `left_out` in `building.ts`, written by hand, and it came first.

| Where | Side | Carries |
| --- | --- | --- |
| `WaitingSource` (`WaitingItem.source`) | Fleet writes, Bridge reads | Gains `preview_offer`: id `preview:<branch>@<tip>`, text `<branch> moved since the preview left it out: <why>`, act `run` whose `target` is `scripts/preview --restart`. Raised on each live Job holding a branch the preview left out, once that branch's local tip moves past the one it was left out at, and not for a branch left out for a breaking migration. Kept once a (Job, branch, tip, build), so a Fleet restart raises nothing again; dropped once the build changes |
| `dismiss_waiting` on it | Bridge writes | No new field. Skip: the id is dismissed and the agent is told nothing. The same tip is not offered again; a newer one is |
| `answer_waiting` on it | Bridge writes | Refused 422 `fleet.session_waiting_empty`, and the item stands. Refresh preview runs the command, as the Fleet panel's switch does |
| `FleetBuildReport.left_out[].moved` | Fleet writes, Bridge reads | `bool`, left out when false. The branch's local tip is not the one it was left out at. Read off local refs, so the report never waits on the network |

**Fleet no longer restarts the preview by itself.** Before this, the sweep ran `scripts/preview --restart` when a left-out branch moved; now it only offers, and the restart waits for the owner's press.
The build notice does not take an offer for a restart wait it answers, so a new build never sends the agent an answer to it.
**A Bridge that matches `WaitingSource` exhaustively** meets a value it did not know; the mirror gained it first, and equal IDs hold the two together.

**Skew.** As above.
**The event stream.** No new event: a new offer publishes `session.changed` on its Job, and so does a build change that drops one.
The open risk below is neither better nor worse.

## Fleet asks about an act a rule planned: `rule_act`, and the `lifecycle_rule` notice

Since 30 Oct 2026. `crates/fleet/src/session_host/rule_asks.rs`, `crates/fleet/src/lifecycles/acting.rs`.
**Two enum values**: no operation, route, event kind, error or type on the wire.
The protocol ID moves with `crates/ipc/src` and `packages/protocol/src`, and there is no number to take.
Bridge's half is `rule_act` on `WaitingSource` in `packages/protocol/src/sessions.ts` and `lifecycle_rule` in `notices.ts`, written by hand.

| Where | Side | Carries |
| --- | --- | --- |
| `WaitingSource` (`WaitingItem.source`) | Fleet writes, Bridge reads | Gains `rule_act`: id `rule_act:<row id>`, text the act in the owner's words (`Move ARM-41 to In progress`) or, once it failed past its retries, what failed, act `answer` whose `target` is the id, and exactly two options, `Do it` and `Skip`, labels only. On a failed act the first label is `Try again`. A draft act raises none: it is proposed at once and the `draft` item is the ask. Read off the kept act on every read, so a restart keeps it; gone once the Job ends |
| `answer_waiting` on it | Bridge writes | No new field. Choice 0 hands the act to Fleet to carry out, and choice 1 records it skipped. Neither sends the agent anything. Words alone are a 422 `fleet.session_waiting_empty`; a second answer finds it settled and is a 409 |
| `dismiss_waiting` on it | Bridge writes | No new field. It is Skip |
| `Notice` (`SessionRow::Message.notice`, a mute's `notice`) | Fleet writes, Bridge reads | Gains `lifecycle_rule`: a rule of the Job told its agent something, or woke it, or told it what failed. It is muted, taught and given a Fleet rule as any other notice is |

**A Job started before this holds no copy of the rules**, so it plans nothing and raises nothing; the store gained tables for the copy, the acts and what was last seen, and no table or column was changed.
**A Bridge that matches `WaitingSource` or `Notice` exhaustively** meets a value it did not know; the mirror gained both first, and equal IDs hold the two together.

**Skew.** As above.
**The event stream.** No new event: a planned or settled act publishes `session.changed` on its Job.
The open risk below is neither better nor worse.

## A note sent to a Job is kept on it: the `note` artifact

Since 29 Oct 2026. `docs/concepts/session/waiting-on-the-person.md`.
**One new value of an open field**: no operation, route, event kind, error or type on the wire.
`Attachment.detail` is already a map of strings, so the form needs no new type; the protocol ID moves with `crates/ipc/src`, where the note file's reading and rewriting live, and there is no number to take.
Bridge's half is `apps/desktop/src/shared/note-artifacts.ts`, which reads the detail into `NoteArtifact`.

| Where | Side | Carries |
| --- | --- | --- |
| `Attachment` of kind `artifact`, `detail.form` `note`, target `note:<id>` | Fleet writes, Bridge reads | `note`, `title`, `text`, `tag`, `status` (`open`, `done`, `returned`) and `sent`; `component`, `screen`, `address`, `file` and `picture` where known, the last two absolute paths |

**The event stream.** No new event: keeping a note, marking it done and a close settling it each publish `session.changed` on its Job.
Bridge's main reads a note file rewritten off that list (`NotesWatch`) and tells the dev-only annotation layer on `annotations:changed`, which is not Fleet's seam.
The open risk below is neither better nor worse: a close settling notes publishes one more `session.changed` on its Job, once.

## A sketch waits in Needs you: `sketch` on `WaitingSource`

Since 29 Oct 2026. `crates/fleet/src/session_host/sketching.rs`, `sketch_items`.
**One enum value**: no operation, route, event kind or error.
The protocol ID moves with `crates/ipc/src` and `packages/protocol/src`, and there is no number to take.
Bridge's half is `sketch` on `WaitingSource` in `packages/protocol/src/sessions.ts`, written by hand.

| Where | Side | Carries |
| --- | --- | --- |
| `WaitingSource` (`WaitingItem.source`) | Fleet writes, Bridge reads | Gains `sketch`: id the sketch's target `sketch:<title>`, text its title, act `message` on the Job. Derived on every read for each sketch with `detail.asked` and no `detail.settled`; never kept |
| `dismiss_waiting` on it | Bridge writes | No new field. The id is dismissed and the Job is sent `Owner dismissed "<title>"`. Drawing the title again lifts the dismissal |
| `answer_waiting` on it | Bridge writes | Refused 422 `fleet.session_waiting_empty`, and the item stands: `answer_sketch` answers it |
| `replied:<turn ended>` on a step that ends on his reply | Fleet writes, Bridge reads | No new value: a `replied` item, text `<step>: you reply`, raised at a turn end where nothing else waits, a reply that only reports included |

**A Bridge that matches `WaitingSource` exhaustively** meets a value it did not know; the mirror gained it in the same change, and equal IDs hold the two together.

## Workspaces, lifecycles and rules, read: `list_repository_workspaces`, `list_lifecycles`, `list_lifecycle_rules`, `get_job_lifecycles` and `job_lifecycles.changed`

Since 30 Oct 2026. The design is *Workspaces, lifecycles and rules*; `crates/ipc/src/lifecycles.rs`.
**Additive only, and reads only**: four operations, four routes and one event kind; no field on an existing type, and nothing saves a piece or moves an instance.
Bridge draws a Job's lanes from `get_job_lifecycles`, kept current by `job_lifecycles.changed`, and the Job's waiting line from `workflow_waiting_on`; it reads a Job once when the Job first appears and again after a `Missed`.
A Job whose read carries `acts` has its repository's `list_lifecycle_rules` read as well, since an act is drawn on the edge its rule is.
The other two reads are drawn by nothing yet.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/lifecycles.ts`, written by hand, field for field, and `apps/desktop/src/shared/api/lifecycles.ts` with its main-process host.

| Where | Side | Carries |
| --- | --- | --- |
| `list_repository_workspaces` (`GET /workspaces?manifest_id=`, Bridge only) | Bridge reads | `RepositoryWorkspaces` `{ manifest_id, workspaces[], left_out[] }`; a `WorkspaceOfRepositories` is `{ id, name, repositories[], home?, source, path, overridden_by?, hides? }`. 422 `fleet.unacceptable_proposal` for a repository Fleet does not serve |
| `list_lifecycles` (`GET /lifecycles?manifest_id=`, Bridge only) | Bridge reads | `RepositoryLifecycles` `{ manifest_id, lifecycles[], left_out[] }`; an `AppliedLifecycle` is `{ lifecycle, kept }`, a `LifecycleDefinition` `{ id, name, scope, states[], transitions[], ends[] }` |
| `list_lifecycle_rules` (`GET /lifecycles/rules?manifest_id=`, Bridge only) | Bridge reads | `RepositoryLifecycleRules` `{ manifest_id, rules[], left_out[] }`; an `AppliedRule` is `{ rule, kept }`, a `LifecycleRule` `{ id, name, scope, when, conditions[], does[], autonomy?, on_failure }` |
| `get_job_lifecycles` (`GET /sessions/lifecycles?session_id=`, `agent_access = "Yes"`) | Bridge or an agent reads | `JobLifecycles` `{ session_id, lifecycles[], instances[], moves[], acts? }`; a `RuleActReading` in `acts` is `{ rule, text, course, standing, said?, at, settled_at?, on?, entered? }`. 422 `fleet.job_lifecycles_unplaced`; 404 `fleet.no_such_session` |
| `job_lifecycles.changed` (event, Bridge only) | Fleet publishes | `JobLifecycles`, whole, after an instance was taken on or moved |
| `SessionRecord.workflow_waiting_on` | Fleet writes where the Job's step is served | A list of sentences, `Waiting on PR #812: Approved`, one for each lifecycle end the Job's current step lists that has not come, in the order the step lists them. Left out when empty: the step waits on no lifecycle, or an end has come |
| `StepReading.ends_any` (`get_workflow_steps`) | Fleet writes | A delivering step gated by a lifecycle now reads `every pr is approved, then it is merged`; `ends_when` still says only `it is merged` |

**`HeldInstance.ended_at` is left out unless ending the Job ended the instance**, so a Fleet older than the field answers as it did and Bridge reads every instance as live.
Ending a Job keeps each instance no end state had taken as ended, standing where it was, and raises `job_lifecycles.changed`; a Job that holds nothing raises nothing.
`all_done` and `finish_up` are refused 409 `fleet.lifecycles_in_flight` while the Job holds one in a state its lifecycle does not end on (an unread or unmapped instance holds nothing), and `lifecycles_clear` is the notice that tells the agent that was refused once the last is over (`crates/ipc/operations/all_done.toml`).

**`acts` are the acts the Job's rules planned, oldest first, and are left out when there are none**, so a Fleet older than the field answers as it did and Bridge draws no act.
`course` is `acts` or `asks`; `standing` is `planned`, `asked`, `stuck`, `done`, `skipped`, `told` or `not_carried`; `text` is the one line the owner reads, as it was kept, and `said` is what failed or why nothing here carries it.
`on` is the instance an act is done to and `entered` the one whose entering a state set it off; both are left out for an act done to none.
**Nothing links an act to the card it asked in the thread**, so a rule that asked has no card to open from the lanes yet.
**`job_lifecycles.changed` carries the acts a rule planned on a move**, since the move is kept and heard before it is raised, and not the ones it settled afterwards: a Bridge reads those at its next read.

**A Workspace here is a set of repositories**, and the wire type is `WorkspaceOfRepositories` so it is never read as the Manifest's monorepo `workspaces`, `WorkspaceCommands` or `ScannedWorkspace`.
For the same reason a rule is `LifecycleRule`, and `FleetRule` stays the notice rule `fleet.toml` carries.
**DTOs, not `core_model` types.** `fleet::lifecycles::wire` converts field by field, and a closed set it matches on is matched exhaustively, so a new state source, act or scope fails to compile there rather than reaching Bridge unnamed.
Closed sets cross as strings and are listed in the mirror's comments, as `protocol.ts` says.
**`kept` says where a piece came from and what became of it.** `rank` is `{ reach, source, home? }`; `path` is absent for what Armada carries; `overridden_by` is the rank of the one that runs instead; `hides` is the place a Kit file replaced because it says something else.
The one that runs for each id comes first.
**`left_out` is a file Fleet could not read and the sentence a person reads for it.** A file that will not read names no repository, so a Workspace's `left_out` is every Workspace file Fleet set aside, whichever repository was asked about.
Everything is read when asked and never cached.
**A rule's `autonomy` is the rule's own word and absent when it has none**; each act in `does` carries the autonomy it will run under, resolved from the default for its kind, so Bridge never has to hold that table.
**A Job's read follows the repository's current pieces.** Its definitions are `lifecycles_that_apply`, the same ones that read its instances.
When a Job's copy is frozen on start, that function changes and this follows.
**The Job is the one the connection places** where Fleet hosts it, else `session_id`, as a watch's is.
A Job that holds nothing, which is every Job that began before lifecycles existed, answers three empty lists.
A `label` is what a person reads for an instance: the ticket's key, `PR #812`, the Job's title or its id when it has none.
**The waiting line rides the Job's row and has no event of its own.** `workflow_waiting_on` is derived each time the row is drawn, as `workflow_step` is, and the row is drawn again when an instance moves under a step that waits on a lifecycle.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** `job_lifecycles.changed` is a new kind.
It is raised when a Job takes an instance on and when a read finds one somewhere else, and not for a repeat read or a Job that holds nothing.
It carries the Job's definitions each time, a few KB at a handful per Job, so a Bridge that missed one needs only the next, or `get_job_lifecycles` on the resync that follows a `Missed`.
**It makes the unbounded WebSocket sink a little worse and does nothing about it.** The load is small and rare next to a Drone's rows, and the bounded broadcast with drop-oldest is still the fix.

## A Job's own copy: `get_job_copy`, `change_session_steps`, `change_job_copy`, `keep_job_workflow`, `keep_job_piece` and `propose_workflow_change`

Since 30 Oct 2026. The design is *Workspaces, lifecycles and rules*, *Changing one Job*; `crates/ipc/src/job_copy.rs`, `crates/fleet/src/session_host/job_copying.rs`.
**Additive**: seven operations, seven routes and one event kind, `set_session_workflow` and `skip_session_step` opened to the agent, and four fields on existing types.
The agent changes its own Job's copy of its workflow, lifecycles and rules with a tool, the owner does the same from the Job's lanes, and both go through the same operations.
**A change applies to that Job at once and to no other**: the files in the repository and in Kit, and every other Job's copy, stay as they are.
Only Keep for every Job proposes it for the others, and that waits on the owner's Approve.
Bridge's half is `packages/protocol/src/job-copy.ts`, written by hand, and `apps/desktop/src/shared/api/lifecycles.ts` with its main-process host.

| Where | Side | Carries |
| --- | --- | --- |
| `get_job_copy` (`GET /sessions/copy?session_id=`, `agent_access = "Yes"`) | Bridge or an agent reads | `JobCopy` `{ session_id, workflow?, own?, steps[], lifecycles[], rules[], changes[] }`; a `JobCopyStep` is `{ id, label, about?, instructions?, ends_on[], skipped? }`, a `JobCopyChange` `{ lane, item, text, by, at, dropped? }` |
| `change_session_steps` (`POST /sessions/workflow/steps`, `Yes`) | Bridge or an agent sends | `ChangeSessionSteps` `{ session_id, change }`, a `StepChange` tagged by `kind`: `add`, `move`, `reword` or `drop`. Answers the `JobCopy` |
| `change_job_copy` (`POST /sessions/copy`, `Yes`) | Bridge or an agent sends | `ChangeJobCopy` `{ session_id, kind, id, text? }`, `kind` `lifecycle` or `rule`; a blank `id` with a `text` takes the id the file declares, and no `text` drops the piece. Answers the `JobCopy` |
| `keep_job_workflow` (`POST /sessions/copy/keep`, Bridge only) | Bridge sends | `KeepJobWorkflow` `{ session_id, source? }`. Answers the `WorkflowChange` kept, `origin` `{ kind: "job" }`, `state` `ready` |
| `keep_job_piece` (`POST /sessions/copy/keep_piece`, Bridge only) | Bridge sends | `KeepJobPiece` `{ session_id, kind, id }`, `kind` `rule` or `lifecycle`. Answers a `JobPieceProposed` `{ change, kind, id, scope, replaces }` |
| `list_keep_drafts` (`GET /lifecycles/keep_drafts?manifest_id=`, Bridge only) | Bridge reads | `KeepDrafts` `{ manifest_id, drafts[] }`; a `KeepDraft` is `{ kind, id, text }`, `kind` `rule` or `lifecycle`, `text` the piece's file as Approve would save it. The open `keep_job_piece` proposals of that repository, one per piece with the newest text standing, oldest first. 422 `fleet.unacceptable_proposal` for a repository Fleet does not serve |
| `keep_drafts.changed` (event, Bridge only) | Fleet publishes | `KeepDraftsChanged` `{ manifest_id }` and no draft. Raised once a proposal is kept, once his Approve saves it and once his Dismiss drops it; a refused Approve raises nothing |
| `propose_workflow_change` (`POST /workflows/changes/propose?manifest_id=`, `Yes`) | An agent sends | `ProposeWorkflowChange` `{ workflow_id, source, definition }`. Answers the `WorkflowChange` kept |
| `set_session_workflow`, `skip_session_step` | `Yes` now | An agent calls them on its own Job; the answer is a receipt (`workflow`, `workflow_step`, `skipped_steps`) |
| `JobLifecycles.rules`, `JobLifecycles.changes` | Fleet writes | The rules the Job's copy holds, which are the ones that run for it, and what was changed in the copy, for the lanes to mark. Both are left out when empty |
| `SessionRecord.workflow_copy` | Fleet writes | The Job's own `WorkflowSummary` once it changed a step. The rail and the lanes read it in place of the repository's |
| `TimelineKind` `session.copy` | Fleet writes | `says` names the change (`step added: Dispatch after Approve plan`, `rule dropped: deploy_watch`), `detail` is `by the Job` or `by you` |
| `ChangeOrigin` `{ kind: "job" }` | Fleet writes | A drafted change a Job proposed, with no model behind it |

**The Job is the one the connection places**, as a watch's is, and the person's request names it.
A call through the agent door that places no Job is refused 422 `fleet.job_copy_unplaced`; every other caller is the owner and `session_id` names the Job.
**A workflow is copied at the first change to its steps**, as the file the Job followed read then, so the Job keeps that text while the repository's changes; until then it follows the repository's.
A lifecycle or a rule is changed in the copy the Job froze on start (`job_copies`); 422 `fleet.job_holds_no_copy` for a Job started before copies were kept.
A step is edited as text and the result is read by the loader again, so an edit that does not fit is refused with the loader's own sentence (422 `fleet.job_workflow_unfit`, `fleet.job_copy_step_refused`, `fleet.job_copy_piece_refused`) and the Job keeps what it had.
The rail keeps a step's state by its index, so an edit remaps what it kept by step id: a skip stays on its step when one is added before it, and a step taken out is forgotten.
Switching the workflow gives up the copy and the marks with it.

**A mark is kept with who made it**, `job` or `you`, on `job_copy_marks`: a step is marked as `workflow` with its id, a rule as `rule` with its id, and a lifecycle by its own id with no item.
A step or a rule that was dropped has no node to draw, so its mark is listed under the lanes and drawn on nothing.
`job_lifecycles.changed` is raised after each change, and `session.changed` carries `workflow_copy`, so a Bridge re-reads nothing of its own.

**Keep for every Job is the workflow half.** `keep_job_workflow` and `propose_workflow_change` keep a `workflow_changes` row `ready` at once, with a `draft` row on the Job, and the owner's Approve saves it as any drafted change.
`send_back_workflow_change` refuses it (422 `fleet.workflow_change_not_sendable`), since no model drafted it.
**A rule or a lifecycle is kept one piece at a time**, from its own row under the lanes: `keep_job_piece` proposes the piece the copy holds as a file in Kit within the Job's repository, the repository's own level.
`save_lifecycle_piece` writes at once and has no held form, so the proposal waits in the `workflow_changes` row a workflow's does: `piece` is `rule` or `lifecycle`, `source` is `kit`, `workflow_id` is the piece's id, and `before` and `after` are Kit's file now and the piece as a save would write it.
A `draft` row on the Job carries it, named `Rule <id>` or `Lifecycle <id>`, and the owner's Approve on that draft is the save, through `save_lifecycle_piece`'s own: it replaces Kit's file only where the row began from it (409 `fleet.workflow_changed_since` where Kit's changed since), and his Dismiss drops the row.
**It is approved from the Job's draft and not from the Workflows page**: `list_workflow_changes` never lists a piece, and `approve_workflow_change` and `dismiss_workflow_change` answer 404 for one.
The Workflows page marks the pieces that wait, with the clock, and draws one the repository does not hold from the draft's text, so it reads `list_keep_drafts` for its repository and reads it again on `keep_drafts.changed`.
The event names the repository and carries no draft: it is raised from the three places that move a proposal, never from an agent's tool call, a few bytes at the owner's pace, so it adds nothing to the unbounded sink of the event stream, and a Bridge that missed one reads the list on the resync after a `Missed`.
A lifecycle drawn only from a draft has no file, so the page offers no Name and no Delete for it.
A piece the Job dropped is marked `dropped` on its `JobCopyChange` and is refused (422 `fleet.job_copy_piece_refused`), as is one as the Job froze it and one Kit already holds.
Fleet writes nothing to a tracker or a forge for any of this.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.

## Workspaces, lifecycles and rules, saved: `save_lifecycle_piece`, `delete_lifecycle_piece`, and `commit_to_repository` for pieces

Since 30 Oct 2026. The design is *Workspaces, lifecycles and rules*; `crates/ipc/src/piece_saving.rs`.
**Additive only**: two operations and two routes, and a field and four variants on types that exist, all optional or new. Nothing is removed or renamed.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is the mirror in `packages/protocol/src`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `save_lifecycle_piece` (`POST /lifecycles/pieces/save`, Bridge only) | Bridge sends, Fleet answers 200 | `SaveLifecyclePiece` `{ piece, overwrite? }`, `piece` tagged by `kind`: `lifecycle` `{ lifecycle }` (a `LifecycleDefinition`), `rule` `{ rule }` (a `LifecycleRule`), `workspace` `{ workspace: { id, name, repositories[] } }` or `autonomy` `{ autonomy: { id, scope?, defaults } }`. Answers `LifecyclePieceSaved` `{ kind, id, scope, file, replaced }`. 422 `fleet.piece_unfit`, `fleet.piece_exists`; 500 `fleet.piece_unwritable` |
| `delete_lifecycle_piece` (`POST /lifecycles/pieces/delete`, Bridge only) | Bridge sends, Fleet answers 200 | `DeleteLifecyclePiece` `{ kind, id, scope? }`; answers `LifecyclePieceDeleted` `{ kind, id, scope, file }`. 422 `fleet.piece_not_kept`; 500 `fleet.piece_unwritable` |
| `PieceToCommit` (`commit_to_repository`) | Bridge sends | Gains `lifecycle` `{ id, scope? }`, `lifecycle_rule` `{ id, scope? }`, `workspace` `{ id }` and `autonomy` `{ id, scope? }`; `scope` is a `PieceScope`, everywhere where absent |
| `CommitToRepository.remove` | Bridge sends | Optional, false where absent: delete the piece from the repository instead of copying it in |
| `fleet.commit_refused` | Fleet answers | Now also for a piece committed to a repository its scope does not belong to, a Workspace committed to a second repository, `remove` of a kind that cannot be removed, and `remove` of a piece main does not hold |
| `fleet.share_refused` | Fleet answers | Now also for these four kinds, which Share does not carry yet |

**The DTOs a read answered are the input.** Bridge sends what it was sent, changed; Fleet writes the file syntax, so no client holds a second copy of the format.
`fleet::piece_saving::text` makes the text from structs shaped like the file, and `config::piece_text` is the only place YAML is produced.
**The piece's own parser reads that text before anything is written** (`lifecycle`, `rule`, `workspace` or `autonomy` in `config`).
A piece it refuses is a 422 `fleet.piece_unfit` carrying the parser's own sentence with every fault and its key, and nothing is created: not the folder, not a staged file.
A test round-trips the parsers' own fixtures through the DTO, the writer and the parser, so the writer cannot drift from the format.
**A piece is its kind, id and scope, and not its id.** `ticket` everywhere and `ticket` within a repository are two files that rank against each other.
The file that holds one is found by the id and scope it declares, whatever it is called (`.json`, `.yml` or `.yaml`), as `save_workflow` finds a definition.
A Workspace has no scope.
Without `overwrite` a save of a held piece is a 422 `fleet.piece_exists` naming the file; with it the file that declared the piece is replaced in place and any second file declaring it is removed.
A new file is `<id>.yml`, or `<id>.<scope>.yml` within a scope, and never takes the name of a file that declares something else: the name gains `-2`, `-3` and on.
Kit's folder is made where absent, and nothing outside it is touched.
**Reads need nothing re-read or published**: the three repository reads read the files each time.

**Commit takes the four kinds the way it takes a workflow.** Kit's file is found by the id and scope it declares and copied into `.armada/<folder>/` under its own name, in place of the file on main that declares the piece, with `overwrite` only, and never over a file that declares something else.
**Where it may be committed follows its scope.** A piece scoped within a Workspace goes to the Workspace's home, the served repository whose `.armada/workspaces/` holds the Workspace file, and a commit to any other names the home.
A Workspace only Kit holds has no home yet, so a piece scoped within it is refused until the Workspace is committed.
A piece scoped within a repository's Manifest id goes to that repository.
A Workspace is committed to one repository, since two holding one id leave both set aside.
**`remove` is built for these four kinds.** It deletes the file on main that declares the piece, in a Job of its own with a pull request, as a copy does.
Any other kind, or a piece main does not hold, is a 422 `fleet.commit_refused`.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
Were a pair to meet anyway, a Fleet without the routes answers 404, and one without `remove` refuses a commit body that carries it, since `CommitToRepository` denies unknown fields, so a removal is never read as a copy.
**No event.** Nothing is published: a save is read by the next read.

## A document asks for his approval: `answer_document`, `document` on `WaitingSource`, and `asked`, `revision`, `kept_at` and `settled` on a document

Since 30 Oct 2026. `crates/ipc/operations/answer_document.toml`, `crates/fleet/src/session_host/document_answers.rs`.
**One operation, one enum value and keys of a document's `detail`**: a new route, its body and one enum; no event kind and no field on an existing type.
The protocol ID moves with `crates/ipc/src`, the operation file and the route table, and with `packages/protocol/src` for Bridge's half.
Bridge's half is `AnswerDocument` and `DocumentHow` in `packages/protocol/src/sessions.ts`, written by hand, and `document` on its `WaitingSource`.

| Where | Side | Carries |
| --- | --- | --- |
| `answer_document` (`POST /sessions/document/answer`, `agent_access = "No"`) | Bridge writes | `AnswerDocument` `{ session_id, document, revision, how, words? }`: `document` the item id `document:<path>`, `revision` the `detail.revision` he read, `how` `approved` or `sent_back`; answers the `SessionRecord`. 409 `fleet.document_revised` for a revision that is not the newest; 422 `fleet.document_not_open`, `fleet.document_answer_empty` and `fleet.document_answer_unnamed`, with nothing written or sent |
| `detail.asked`, `detail.revision`, `detail.kept_at` on an artifact of form `document` | Fleet writes, Bridge reads | `asked` `"true"`, `revision` the newest revision's number as text, `"1"` and on, and `kept_at` when it was kept, as `settled_at` is written; all three on every newly kept revision only. Bridge places the card by `kept_at`, since `changed_at` moves on an answer. A document kept before this has none and asks nothing |
| `detail.settled`, `detail.settled_at`, `detail.words` | Fleet writes, Bridge reads | How he answered, `approved` or `sent_back`, when, and his words. Gone when a new revision is kept, since the store replaces a row's detail whole |
| `WaitingSource` (`WaitingItem.source`) | Fleet writes, Bridge reads | Gains `document`: id `document:<path>`, text the page's title, act `message` on the Job. Derived on every read for each document with `detail.asked` and no `detail.settled`; never kept |
| `dismiss_waiting` on it | Bridge writes | No new field. The Job is sent `Owner dismissed "<title>"`, and a new revision lifts the dismissal |
| `answer_waiting` on it | Bridge writes | Refused 422 `fleet.session_waiting_empty`, and the item stands: `answer_document` answers it |
| `ends_on` | A workflow's file | Gains `document_approved`: he approved the newest revision of a page the Job wrote, read off the kept artifact and never off the thread |

**The Job is told as his message**, as a sketch's answer is: `Approved document: <title>, revision <n>` and `Go on from it as written.`, his words on the line after, or `Sent back document: <title>, revision <n>`, his note, and `Revise the page at <path> and write it again.`
The revision is settled only once the message is sent.

**Skew.** As for `answer_sketch`: the IDs differ and the lifeboat serves.
**The event stream.** No new event: an answer publishes `session.changed`.

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
