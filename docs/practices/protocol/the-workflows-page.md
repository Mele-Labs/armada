# The Workflows page: `get_workflow_steps`, `delete_workflow_copy`, `reset_workflow_copy`, `mark_not_a_drift` and the changes an agent drafts

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 27 Oct 2026. The System › Kit › Workflows page draws a workflow as its steps: the words, what ends each, and the Jobs Fleet nudged there.
**Four operations, and eight for the changes an agent drafts (below), and one new field on `WorkflowCopy`.** The protocol ID moves with `crates/ipc/src`, `crates/ipc/operations/*.toml`, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/setup.ts`, written by hand, field for field.

| Operation | Route | Side | Carries |
| --- | --- | --- | --- |
| `get_workflow_steps` | `GET /workflows/steps?workflow_id=&source=&manifest_id=` | Fleet answers | `WorkflowSteps`: the id, the place and `steps`, each a `StepReading`: `id`, `label`, `instructions?`, `ends_when?`, `ends_any?`, `passed_over_when?`, `nudges?` (`StepNudge`: `id`, `job_id`, `job_title?`, `fact`, `at`) |
| `delete_workflow_copy` | `POST /workflows/delete?manifest_id=` | Bridge sends `WorkflowCopy` (`workflow_id`, `source`, `file?`) | `WorkflowCopyDeleted`: the id, the place, the file and `runs_from?` |
| `reset_workflow_copy` | `POST /workflows/reset?manifest_id=` | Bridge sends `WorkflowCopy` | `WorkflowSaved`, since a reset is a save |
| `mark_not_a_drift` | `POST /workflows/not_a_drift` | Bridge sends `MarkNotADrift` (`nudge`) | `NotADriftMarked` |

**A route of its own, and not fields on `get_workflow`.** That read is the file's text and not a tree, so what an editor sends back to `save_workflow` is the string it was given.
The steps are read from the parsed file and resolved against the repository's Checks as a Job would freeze them.
One a more specific place replaced is resolved on the read, since the catalogue never does.
`ends_when` and `passed_over_when` are `rail_ends`' reading of the rules the rail places a Job by, so the page does not restate them.
`ends_any`, since 28 Oct 2026, is every end of a step, in order, as the page says them, where any one of several ends it; left out where empty.
Nudges are those `workflow_nudges` keeps for that step in that place's definition, less those marked not a drift.

**Refusals**, all 422 but the last:

| Code | Where |
| --- | --- |
| `fleet.no_such_workflow_definition` | `get_workflow_steps`, delete and reset, where that place holds none by that id; a definition that was left out is on `list_left_out_workflows` and is not here |
| `fleet.workflow_unfit` | `get_workflow_steps` of a replaced definition that names a Check this repository does not declare; reset, where Armada's does not fit here (nothing is written) |
| `fleet.workflow_is_armadas` | delete and reset with `source: armada` |
| `fleet.workflow_not_carried` | reset of a copy of an id Armada does not carry |
| `fleet.no_such_nudge` | `mark_not_a_drift` of an id Fleet kept none by |
| `fleet.workflow_unwritable` (500) | delete, where a file cannot be removed |

**Make my copy needs no route.** `get_workflow` with `source=armada` reads Armada's text and `save_workflow` with `scope: kit` and `overwrite: false` writes it under the same id; 422 `fleet.workflow_exists` where Kit already holds one.
In the Workspace's lanes the first change to one of Armada's makes the same copy, once, and the change is written to it; the column of steps keeps its row read-only.
It is checked against the Checks of the repository Armada's row was listed under, as every save is.

**Each copy is read and acted on under the Manifest Fleet listed it under**, the `manifest_id` on its `WorkflowSummary`, and not the pick's.
`list_workflows` answers every repository's whichever is picked.
With one picked Bridge keeps that repository's rows; on All it keeps Armada's and Kit's once and each repository's copies apart, and asks `list_left_out_workflows` of none.

**Delete and reset act on copies Fleet holds, or on one it left out by its file.** `WorkflowCopy.file`, Fleet's and Bridge's, is the file `list_left_out_workflows` names.
A copy that was left out for not fitting may declare no id Fleet can read, so it is found among those left out in that place by its file and not by its id.
A delete removes that one file.
A reset checks Armada's definition as a save does before it removes the file and writes Armada's in its place, so Armada not fitting here leaves what was there.
`workflow_id` is then Armada's id to reset to.
A `file` that is not left out in that place is 422 `fleet.no_such_workflow_definition`.
Fleet reads the three places again before answering, as `save_workflow` does. No store migration, no event.

**Agents.** The read is `Yes`, `get_workflow`'s terms; the three acts are `Bridge only`: they are the person's press.

**The event stream is untouched**, so this makes the unbounded sink neither better nor worse.

## The changes an agent drafts

Since 27 Oct 2026. He types a change, or a step drifts, and an agent drafts the whole new definition.
He sees it beside what it was and approves it, dismisses it or sends it back with notes.
**Nothing saves without his Approve.**

| Operation | Route | Side | Carries |
| --- | --- | --- | --- |
| `ask_workflow_change` | `POST /workflows/changes/ask?manifest_id=` | Bridge sends `AskWorkflowChange` (`workflow_id`, `source`, `text`) | `WorkflowChange`, `drafting` |
| `suggest_workflow_change` | `POST /workflows/changes/suggest?manifest_id=` | Bridge sends `SuggestWorkflowChange` (`workflow_id`, `source`, `step_id`) | `WorkflowChange`, `drafting` |
| `propose_new_workflow` | `POST /workflows/changes/propose_new?manifest_id=` | A Job sends `ProposeNewWorkflow` (`workflow_id`, `definition`, `scope`) | `WorkflowChange`, `ready`, `origin` `new`, `before` empty |
| `list_workflow_changes` | `GET /workflows/changes?workflow_id=&source=&manifest_id=` | Fleet answers | `WorkflowChanges`: the open ones, oldest first |
| `approve_workflow_change` | `POST /workflows/changes/approve` | Bridge sends `WorkflowChangeRef` (`change`) | `WorkflowChangeApproved`: the `change` and the `saved` |
| `dismiss_workflow_change` | `POST /workflows/changes/dismiss` | Bridge sends `WorkflowChangeRef` | `WorkflowChange`, `dismissed` |
| `send_back_workflow_change` | `POST /workflows/changes/send_back` | Bridge sends `SendBackWorkflowChange` (`change`, `notes`) | `WorkflowChange`, `drafting` |
| `build_workflow_support` | `POST /workflows/changes/build` | Bridge sends `WorkflowChangeRef` | `WorkflowSupportBuilding`: `job_id` |
| `propose_workflow_to_armada` | `POST /workflows/propose` | Bridge sends `ProposeToArmada` (`workflow_id`, `definition`) | `ArmadaProposed`: `workflow_id`, `session_id`, `send_failed?` |

`WorkflowChange` is `id`, `workflow_id`, `source` (`repository` or `kit`), `step_id?`, `origin` (`{kind: asked, text}`, `{kind: drift, nudges}` or `{kind: new}`), `before`, `after?`, `state` (`drafting`, `ready`, `approved`, `dismissed`, `failed`, `cannot`), `reason?`, `missing?`, `before_steps?`, `after_steps?`, `notes?` and `at`.
`ChangeStep` is `id`, `label`, `instructions?`, `ends_when?` and `passed_over_when?`, as `StepReading` says them.
Fleet keeps it in `workflow_changes`; no event carries it.

**Drafting is one short model run and not a Job.** The call is the proposer's: its client, budget and `models.proposer`.
The input is the definition's text and his words (or the step's words and what the Jobs that drifted did, plus any notes and the draft he sent back); the output is the whole new definition.
`prompts.workflowChange` is the question, section 5h of the agent prompt contract.
**Fleet holds the answer to what a save holds it to**: `check_workflow_file`, the parse and resolve `save_workflow` uses, then the same `workflow_id`, then a difference from what it was made from, and for a suggestion a difference in that one step's `instructions` and nothing else (`WorkflowDef::differs_only_in_words_of`).
An answer that does not fit is asked for once more with the loader's sentence and the answer that did not fit.
After that the change is `failed` and `reason` is the last sentence.
An answer unchanged from the definition fails at once and is not asked for twice.

**A change Armada can't make is `cannot`, since 28 Oct 2026.** What ends a step and when one is passed over are read from its fields, never its words, and the question says so and lists what the fields can say.
Where the request needs something they cannot say, even in part, the agent answers `CAN'T` and one line per missing capability.
Fleet keeps those lines as `missing`, asks nothing again, and lists the change beside ready and failed ones; Dismiss settles it, and send back does not take it.
A `CAN'T` that names nothing, or one answering a suggestion, leaves the change `failed`.

**Each side step by step.** On a `ready` change Fleet reads `before` and `after` through the rail, resolved against the repository as a Job would freeze them, into `before_steps` and `after_steps`, so a step the draft adds says what ends it; a `cannot` change carries `before_steps`.
A side whose text does not parse or resolve leaves its field absent, and Bridge reads the text.

**Build support** starts a Job on `armada`, as Propose to Armada does, titled `Build workflow support:` and the first missing line, and sends it a fixed brief: the workflow's id, the request and the missing lines as material, and the ask to build that support in Armada with tests and land it as a pull request.
The change is then dismissed. A brief that could not be sent is a line on the transcript, and the Job stands.

**A read the page polls while one is `drafting`, and no event.** The event stream is untouched, so this makes the unbounded sink neither better nor worse.
A change left drafting for longer than three proposer budgets is read as `failed`, since a Fleet that stopped mid-draft will never finish it; sending it back drafts it again.

**Approve writes `after` through `save_workflow`'s path with `overwrite`**, in the change's place and the repository it began under.
It is refused, and nothing is written, if the file no longer holds `before`: Fleet reads the three places again first.
He dismisses the draft and asks again.

**A suggestion is begun on its own** when a nudge is kept and at least two still count inside seven days at that step in that place, the place is the repository or Kit, and the step has no open change.
A nudge marked not a drift does not count.
Armada's own definitions get none: they are read-only, and a change to one is Propose to Armada, Make my copy, or a first change in the lanes, which makes the copy.
Suggest a change asks for one sooner, from the nudges that count then.

**Propose to Armada** starts a Job on the repository Fleet serves as `armada`, the manifest id a retro's fix in Armada also looks for, with `start_session` and a brief that says to set the carried `.armada/workflows/` file for the id to the definition and land it as a pull request.
Nothing is written by Fleet.
Fleet holds no marker of which repository is Armada's, and a machine that serves it serves it under that id.

**Refusals**

| Code | Where |
| --- | --- |
| `fleet.workflow_is_armadas` (422) | ask, suggest, with `source: armada` |
| `fleet.no_such_workflow_definition` (422) | ask, suggest, where that place holds none |
| `fleet.workflow_change_empty` (422) | ask with blank words, send back with blank notes, propose with a blank definition |
| `fleet.workflow_change_no_agent` (422) | ask, suggest, send back, where no model could be asked |
| `fleet.workflow_step_not_here` (422) | suggest, a step the definition lacks |
| `fleet.workflow_nothing_drifted` (422) | suggest, a step no nudge counts at |
| `fleet.workflow_change_open` (422) | suggest, a step that has a change waiting; `change` names it |
| `fleet.workflow_armada_not_served` (422) | propose and build, where Fleet does not serve `armada` |
| `fleet.no_such_workflow_change` (404) | approve, dismiss, send back, build |
| `fleet.workflow_change_not_ready` (409) | approve of a change that is not `ready` |
| `fleet.workflow_changed_since` (409) | approve, the file no longer holds `before` |
| `fleet.workflow_change_settled` (409) | dismiss of an approved or dismissed change |
| `fleet.workflow_change_not_buildable` (409) | build of a change that is not `cannot` |
| `fleet.workflow_change_not_sendable` (409) | send back of a change that is drafting or settled |
| `fleet.workflow_unfit` (422) | approve, the checks refuse it now |
| `fleet.workflow_change_not_kept` (500) | the store would not answer |

**Agents.** `list_workflow_changes` is `Yes`, `get_workflow_steps`' terms. The seven acts are `Bridge only`: each spends a model run, writes a definition, or starts a Job, on his press.

## The switches: `features.workflows` and `workflows.off`

Since 29 Oct 2026. The page holds **Workflows for Jobs** at the top and a switch on each row.
**No operation, route or field is new.** Bridge reads both settings from the `get_settings` list it already holds and writes them with `save_settings`, the save every setting takes.

| Setting | Shape | Switch |
| --- | --- | --- |
| `features.workflows` | A boolean, on unless Fleet says off | Workflows for Jobs, the same setting as Settings › Features |
| `workflows.off` | A `Json` value, `{ "machine": ["<workflow id>"], "repositories": { "<manifest id>": ["<workflow id>"] } }` | A row's switch: the machine's list on All, the picked repository's list with one picked |

**The save is the whole value.** A row's press sends `workflows.off` with that one id added to or taken out of the list it concerns, and every other list as it was.
A repository whose last workflow is switched back on has no entry left, and a value that is absent, null or does not read is nothing off (`apps/desktop/src/renderer/src/workflows-off.ts`).
**A workflow off on the machine is off in every repository**, so with one picked its row says `Off on this machine` in place of a switch, and only the machine's switch turns it back on.

**Where it is edited.** Settings draws no row for `workflows.off`: a Json setting is never drawn by the generic form (`drawn` in `packages/surfaces/settings/src/sections.ts`), and the Workflows section that holds nothing else is not offered.
This page is the one place it is edited.

**What reads it.** The start card's Workflow picker leaves out a workflow that is off for the machine or for the Job's repository, and keeps None.
Fleet's half, the proposer, `set_session_workflow` and the rail, is `crates/config/settings.toml`, *Workflows switched off*.
