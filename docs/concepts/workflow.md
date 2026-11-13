# Workflow

**What it is:** The template a Job may follow: an ordered set of steps that sit in the agent's prompt and in the view as a soft rail, each carrying its mechanical checks, Judge checks and evidence scope.
Fleet reads the current step off what it observed and tells the agent nothing of it; nothing blocks.

---

**Kind:** Entity.

How a workflow is structured and where a definition comes from. [Judge](judge.md) owns the semantic tier and [Job](session.md), *The rail*, owns how a Job follows one; this document owns the definition.

## Core principle

**Self-report is a signal, not a source of truth.** The step a Job is on is read from what Fleet observed — the plan, the build record, the Job's own Check runs and commits — and never from the agent's word.

## Workflow schema

Every workflow is a `WorkflowDef`: an ordered set of steps, and nothing else declares its shape.
The repository or Kit may replace a carried workflow, so `workflow_id` alone does not identify what a Job follows; `source` says which place it came from.

**The full field-by-field schema is `crates/core-model/domain/workflowdef-fields.toml`.** It carries every top-level field and every field nested under `steps[]`, with its type, whether it is required and the reasoning behind it.
One row per field, nested the way the schema nests: the top-level rows are the `WorkflowDef`'s own fields, and everything under `steps[]` is a field of a step.

## The workflows

### How a Job gets one

**You do not have to pick.** The [Proposer](job-proposer.md) reads the request a Job is dispatched with and proposes which workflow the work should run under, or none.
Picking one by hand stays available and is the override, not the path.
That document owns the call.

A Job may follow one workflow, which the proposer picks at dispatch or a person sets.
No step gates it.
A step may carry `instructions`, the words the workflow's page shows for it. Fleet does not send them to the agent (5 Nov 2026).
A step with `asks_delivery: true` first asks the owner how the work is delivered, unless the first message says or the Job's Delivery, set before Start, settled it; `bug` and `feature` set it on their first step.
At the delivering step the agent delivers by the answer it was given.
Fleet reads the Job's current step off what it observed, and a step with `judge_checks` has step judges, which are advisory and never block.
[The rail](session/the-rail.md) owns how a step is placed; [Judge](judge.md), *Step judges*, owns them.
**An owner can switch the workflow system off** (`features.workflows`), and every Job is then a plain one.
**One workflow can be switched off** too, for the machine or for one repository (`workflows.off`, Settings, Workflows): it is not offered, a Job cannot be given it, and a Job already on it runs plain until it is switched back on.

### `feature` and `bug`

The order is the owner's, and a mock is walked before any test is written.
The last column is what the Workflows page draws under each step (`rail_ends.rs`).

**`feature`**

| Step | The agent | Ends when |
|---|---|---|
| `propose` | Asks what is unclear, then puts its proposal to him: a plan, with a sketch inside where it helps, or a sketch, draft, document or rough walk alone | you approve the plan, or any one of those |
| `walk` | Builds the mock in the real Bridge on fixtures and shows it as a walk, or several; writes no test | you approve every walk |
| `refine` | Wires it for real and writes the tests | it is pushed |
| `deliver` | Delivers the way he chose | it is merged |

`walk` is passed over when nothing would be seen, or when a rough walk he approved on `propose` was it.

**One approval moves the Job on** (2 Nov 2026).
After he approves anything `propose` lists, the plan the agent then records stands with no card.
A version recorded after one he approved is put to him again, so a task added on `refine` or `deliver` waits on his approval (4 Nov 2026).
The agent prefers one proposal; a second stays open beside the first, and he answers each.

**`bug`**

| Step | The agent | Ends when |
|---|---|---|
| `find_the_cause` | Reproduces the bug, names the cause, records the fix as tasks | you approve the plan |
| `fix` | Makes the fix and shows it as a short walk straight to the place; writes no test | you approve every walk |
| `tests` | Writes the test that fails without the fix and passes with it | it is pushed |
| `deliver` | Delivers the way he chose | it is merged |

`fix` both follows the plan and walks, and its walk is passed over when nothing would be seen: Fix then ends once the plan's tasks are done, and Tests begins.
A separate walk step after it had him approve one walk twice (30 Oct 2026).

### `orchestrate`

A Job whose work is other Jobs: a batch of feedback, a list of issues or a milestone, split up and handed out.
It writes no code and opens no pull request of its own; it lands a report.
The order is the owner's, 29 Oct 2026.

| Step | The agent | Ends when |
|---|---|---|
| `assess` | Reads the pile of work and checks each item against main: done, partial or missing | a plan is recorded |
| `plan` | Records one group per Job | you approve the plan |
| `dispatch` | Starts a Job per group with `start_another_job`, two at a time, the rest queued with `after` | every group's Job is started |
| `oversee` | Follows the Jobs it started, steps in when one is stuck or fails, and may start more | every Job it started has merged or been ended |
| `summarise` | Writes a Document of what each Job landed, with its pull request, then follow-up work, each item filed or dispatched as you pick | you approve the report |

### `uat`

A Job that runs acceptance scenarios against a test environment and writes the results back to the sheet they came from. It writes no code.
The workflow declares `carries_no_code: true`: the Job is never asked for a branch, a worktree slot, tests, a push, a pull request or a merge, and Fleet sends it none of the notices about them.
Its commands and scripts run without a leased slot; only a write into the checkout, such as its own document, takes one.

| Step | The agent | Ends when |
|---|---|---|
| `setup` | Picks the environment: sites, test accounts, log source, allowed scripts and commands, and checks each is reachable | the page is written |
| `preflight` | Marks each scenario runnable, blocked with its reason, or needs you | you settle every one |
| `run` | Runs scenario by scenario with evidence per step; a question waits while the work carries on, and every change to the environment is recorded as it happens | the runnable scenarios are done |
| `review` | Shows the results and the changes made to the environment together | you approve, or send it back to `run` |
| `writeback` | Writes the results to the scenario sheet and ends the Job | you approve the one write |

### Walks and replies

Three step keys place a Job around the owner.
`crates/core-model/domain/workflowdef-fields.toml` carries them.

**`ends_on`** lists what ends the step, any one of them: what the owner does (`replied`, `walk_approved`, `sketch_approved`, `draft_approved`, `idea_picked`, `document_approved` and `you_approved`) or a lifecycle reaching a state.
The agent stops until one holds.
`feature` sets it on `propose`, without `replied`: Propose ends on an approval, never on prose.
**On a step that records the plan**, as `propose` does, the plan's approval ends the step too.
A plan recorded after a listed end holds stands with no card, and the step stays current until that plan is recorded, since the steps after it work from the tasks.
A plan already put to him stays his to answer, and a plan held behind a walk stands with his Approve of that walk.

**`plan_unasked: true`**, on a step that records the plan, means the plan stands as soon as it is recorded and never makes a card.
`design_plan` sets it: its plan is the Job's progress, and what he approves is the page.
`idea_picked` counts only an option the agent's own question gave a preview; Fleet's questions never count.
`document_approved` counts only his approval of a page's newest revision, read off the page Fleet kept.
A step gated `you_approved` ends on it too, and reads "you approve"; the rail holds the Job on the step until the gate has closed.
`you_approved` counts only his Approve on a `waiting_for` item the agent raised with `approves: true`; Fleet gives that item exactly two options, Approve and Ask for changes, and Ask for changes leaves the gate open.

**A lifecycle state** is written `pr: approved`, `ticket: closed` or `job: landed`, and `any ticket: closed` for the first of several (29 Oct 2026).
Every instance of the lifecycle the Job holds must have reached the state, so `job: landed` waits for every Job this one started.
Fleet moves the Job on when a move it kept for an instance satisfies the end, and wakes the agent with one line.
It is the world's to bring about, so the step raises no wait for the owner; if he writes on the step or skips it, the Job moves on without the line.
A lifecycle or state the repository does not have is refused where the workflow is read, naming it, and a step that `walks`, `follows_plan` or `records_plan` takes none: put the wait on the step before or after it.
On a step that `delivers` the state gates the merge instead (Land waits on `pr: approved`): the agent opens the pull request, ends its turn and does not merge, and Fleet wakes it with one line when the state comes.

**`awaits_reply: true`** is `ends_on: ["replied"]`.

**`walks: true`** is the step where he is shown the work and approves it before any test is written.
It is current from the plan's approval until he has approved every window the Job showed since, and it needs a recording step before it.
A window replaced by a newer one of its page, or taken back with `withdraw_window`, waits on nothing, and a walk shown later puts the Job back on the step until he approves that one too (2 Nov 2026).
A plan that stood on a rough walk he approved counts as approved from when the first such walk went out, so that walk passes the step and any shown with it wait.
The step after it is entered the moment it is passed, so a Job is never sent back to show a walk he approved.

**A walk is expected** when the plan said `seen: true`, when it named a `walk`, or when the Job showed a window.
`seen: true` says the person would see the change, and holds nothing.
A `walk` names a page shown before the plan, and holds it.
With none of the three, the walks step is passed over once the work is pushed.

Fleet tells the agent nothing of a test edited before an expected walk is approved, and nothing blocks.

### Skipping a step

**A person can skip any step of a Job's workflow**, the current one or one still to come, from an icon on the step's header or from its right-click menu, and can undo it.
Fleet then counts the step as passed, so the Job's current step moves past it.
The step shows as skipped, and the timeline enters `skipped: <step label>`.

The agent is sent one message, since a person pressed it, saying the step is not wanted, so it neither does the step nor asks about it, and another when the skip is undone.
**The agent can skip a step of its own Job too** (`skip_session_step`); it is not sent a message about a skip it made, and the timeline says `by the Job`.
**The delivering step cannot be skipped**; Fleet refuses it with `fleet.session_step_not_skippable`.

**A person can jump to a step still to come** with Jump here on its header (`jump_session_step`, `POST /sessions/step-jump`).
Fleet skips every step from the current one up to the one before it, in one write, and the timeline enters `jumped to: <step label>`.
The agent is sent one message naming the step and the steps passed, and none of the per-step skip messages.
Fleet refuses a step at or before the current one with `fleet.session_step_not_ahead`, and a jump that would pass the delivering step with `fleet.session_step_not_skippable`.

Fleet sends nothing when the agent skips a step on its own: it tells the agent only what a person pressed.

### Changing one Job

**A Job's agent can change its own Job's workflow, lifecycles and rules with tools**, and the owner can do the same from the Job's lanes.
It adds, moves, rewords or drops a step, adds or changes or drops a rule or a lifecycle, skips a step, and switches to another workflow or to none.
The change applies at once, to that Job only, and what the repository and Kit keep, and every other Job, stay as they were.

The Job that needs other steps, such as one whose work is to dispatch other Jobs and oversee them, says so with a tool instead of being switched to no workflow by hand.
An agent that finds another workflow fits better switches to it; one that finds a workflow it read would serve the next Job better proposes a change to it (`propose_workflow_change`), and one that finds none fits proposes a new one (`propose_new_workflow`).
A proposal waits on the owner's Approve on the Workflows page, as a drafted change does.

| To | He |
|---|---|
| Add a step, a lifecycle or a rule to this Job | Presses `+` above the Job's lanes and chooses which |
| See what changed | The lanes mark the step, the rule's chip and the lifecycle's head, and *Changed on this Job* under them lists each change with who made it, the Job or him |
| Keep a change for the others | Presses *Keep for every Job* on a changed workflow, which proposes the Job's workflow as a change, or on a changed rule or lifecycle, which proposes that one piece for Kit within the repository; nothing saves until he approves it |

**A workflow is copied at the Job's first change to its steps**, as the file it followed then, so the Job keeps following that text while the repository's changes.
A rule or a lifecycle is changed in the copy the Job froze when it started.
A rule or a lifecycle is kept one piece at a time, from its own row under the lanes, and a piece the Job dropped is not offered.
The proposal waits on the Job's draft, since the Workflows page holds a workflow's change and not a piece's.
The timeline enters each change as `session.copy`, naming what changed and who.

### Where a definition comes from

**A workflow is a piece, and takes the places every piece takes**: Armada's, carried in the binary; yours, in [Kit](kit.md), for every project or for one; and the repository's, committed under `.armada/workflows/`.
[Who sees each piece of configuration](../design/who-sees-each-piece-of-configuration.md) owns the model, and this section does not restate it.
**A new workflow is yours.** Nothing of it reaches the repository or anyone else until you press Commit to repository or Share ([Kit](kit.md#commit-to-repository)).
The four are merged by `workflow_id`, with **one rule: yours first, then narrower.** For each id a repository runs the first of these that holds it:

| Order | Place | Read from |
|---|---|---|
| 1 | Your copy for a project the repository belongs to | `~/.armada/workflows/projects/<project-id>/` |
| 2 | Your copy for every project | `~/.armada/workflows/` |
| 3 | The repository's own | `.armada/workflows/` |
| 4 | Armada's | Carried in the binary |

So a teammate's commit never replaces what a person set up for themselves, and a copy kept for one project is not lost to the one kept for every project.
A repository with no workflows of its own uses the carried set, and one file in any other place replaces a carried definition by id.
Yours are read from `~/.armada/workflows/` on this Mac, whatever place the repository's other pieces are kept at.
**Workflows and the pieces rank differently, and both are chosen.** A lifecycle or a rule applies by scope first: one scoped to the repository beats one scoped to a Project, which beats one that applies everywhere, whoever kept it, and only at the same scope does yours beat the repository's (`config`'s `resolving`).
A workflow is a person's own way of working, so it ranks by whose it is first: yours leads, and the narrower of yours leads among yours.
A repository's Project is the one that holds it; a repository in two Projects reads the folders of both.
**Two Projects that both hold the repository and both keep a copy of one id leave both out**, with a reason naming the two Projects, and the next place down answers for the id; keeping the copy for one of them settles it.
A copy kept for a Project that does not hold the repository is never read for it.
Where Kit's, or a Project's, hides a repository definition of the same id that says something else, `list_workflows` carries `hides: "repository"` on it, and Bridge marks its row with the eclipse glyph; two copies that are the same carry nothing.
`config::Catalogue` holds the rule. #425.

**The carried set is compiled in**, and is judged against a repository with tests — Armada's built-in support is for code-with-tests repositories, the owner's decision.
None of them names a Check: they declare `every_manifest_check`, `artifact_exists`, `plan_recorded`, `diff_nonempty` and Judge checks, so each resolves against any Manifest, and a repository declaring no Checks gets none.
See The plan step, below.

**That is Armada holding an opinion it used to refuse.** An empty `.armada/workflows/` was refused because a repository declares how its own work is done, and a carried set is Armada saying how work is done by default.
The trade is accepted, and overriding is one file so that the default does not become the only way.

**A definition that does not fit is left out, not refused, wherever it came from.** One that will not parse, will not resolve against the repository, or shares its id with another file in the same place, is set aside and named when Fleet starts, and the next place down answers for its id — the sentence says whose.
Two files in one place sharing an id are both left out.
The repository's own is held to the same rule as Kit's and Armada's: a file somebody wrote badly does not stop Fleet starting, and the others stand.

**Fleet reads the places a definition comes from again while it runs.** A definition saved or edited under the repository's `.armada/workflows/` or Kit's folder is held without a restart: `save_workflow` reads them before it answers, and a watch on both folders, and on the folders of each Project inside Kit's, reads them after a hand edit, with the settle window `armada.yml`'s watch uses.
**A re-read leaves out by the same rule as start**: a file that does not fit is left out with its reason and the others stand.

**Saving one is `save_workflow`.** The caller names the place, the repository's own, Kit's or one Project's (`{"project":{"id":"…"}}`), and sends the definition whole.
A Project's is refused, with nothing written, where Fleet holds no such Project (404 `fleet.workflow_project_unknown`) or where the repository is not one of its members (422 `fleet.workflow_project_not_member`), since the repository would never run it.
Fleet parses it against the models this machine offers and resolves it against the Manifest, the parse and the resolve a catalogue does, so what is saved is what would load; a definition that does not fit is refused with the loader's reason and nothing is written.
A definition already holding the id in that place is replaced only where the call says `overwrite`, and the file that held it is the one replaced, whatever it is called.
**Commit to repository copies one of yours into the repository** under `.armada/workflows/`, by a Job of its own whose pull request carries it alone, and yours stays ([Kit](kit.md#commit-to-repository)).

**A workflow says which place it came from.** It carries `source` — `armada`, `kit`, `repository` or `project` — beside its steps, and `project` naming the Project where it is that.
Fleet also says it for every workflow when it starts, and `list_workflows` says it with the file, each definition of the same id a more specific place replaced, and the repository definition it hides, so a surface can mark the source and draw the replaced ones apart.
`get_workflow` reads any one of them whole, as the text `save_workflow` takes back.
Fleet's health says whether anything was left out and never how many.

**The full workflow catalogue is `crates/core-model/domain/workflows.toml`**, one row per workflow shape — the shape, gate profile and status, including policy variants and workflows still needing a step list, so there is one list rather than three.
The instantiated `WorkflowDef` sits beside it as JSON in `crates/core-model/domain/workflow-samples/`, where a sample exists.

### Drafts and Publish

**Editing a workflow is a draft, then Publish** (the owner's ruling of 13 Nov 2026).
A draft is Fleet's and not a file: kept for one place and one id, read by no loader and followed by no Job, so half an edit changes nothing that runs.
Each change to it keeps the whole text after it, and the native editor steps back through them by putting an earlier text again, with ⌘Z and a list of changes that each has its own Undo; Fleet keeps the list and the editor does the stepping.
**Publish writes it** through the same save `save_workflow` uses, with overwrite, so the loader's refusals pass through and nothing is written on one; the draft is deleted once the file is.
It is refused as stale where the text published in that place is no longer the text the draft began from, because a hand edit or another save came in between.
A draft that would not load carries the loader's sentence as `unfit`, and is still a draft.
**A change an agent makes is a change to the draft, marked as the agent's.** An asked change begins from the draft's text where there is a draft, and its result goes onto the end of it when the draft is still what the change was made from.
Where the draft moved while the agent wrote, the change stays ready for Approve, as before drafts.
**A draft of one of Armada's own has no scope**, since the carried set is written nowhere: the copy is made on Publish, at the scope he picks, the repository's own, Kit's or one Project's.
Lifecycles, rules and autonomy defaults get drafts the same way, kept in Kit.
**`save_workflow` is unchanged and still writes at once**, as does Approve on a drafted change, since the Classic Bridge uses both.
`docs/practices/protocol/protocol-23-95-a-workflow-edited-as-a-draft-and-published.md` owns the operations.

### The plan step

**A workflow may name at most one step as its recording step**, the one
that keeps the Job's plan ([Job](session.md), *The plan*). It names
itself either of two ways: giving the step `evidence.submitted.type: "plan"`,
where the plan is the whole of its product, or giving it `records_plan: true`
beside another product — Code Review's `read` keeps `read.md` and records the
plan too. Either way it needs a `plan_recorded` mechanical check declaring
`min_tasks`, which resolves against any Manifest the same way
`every_manifest_check`, `artifact_exists` and `diff_nonempty` do, since a plan
is not manifest-specific. A second step recording a plan, whichever spelling
it uses, is refused.

**A later step follows the plan by declaring `follows_plan: true`.** It is
shown the plan in its brief and may add or update a task; a step without it
may read the plan but not change it. `follows_plan` is legal on the
recording step itself — Revert's one step both records the plan and works
it, since Revert has no earlier step to put a recording step on — and on
any step after it. It is refused on a step strictly before the recording
step, and in a workflow with no recording step at all.

**A step says which phase of the run it is in by declaring `phase:`** —
`setup`, `work` or `delivery`. A step declaring none is `delivery` where it
`delivers` and `work` otherwise, never read off its id or its place, and Fleet
serves the resolved phase so no reader spells the fallback again. Design
Plan's `present` declares `delivery`: it delivers what its workflow is for and
sends nothing out.

## The Workflows page

System › Kit › Workflows is where a person reads and changes workflows.
`docs/practices/protocol/the-workflows-page.md` owns the operations behind it.

**The list is three groups**: Armada's, the repository's and Yours, which is Kit's.
A glyph on a row says that copy replaces another.
The list's search reads *Find workflow* and takes `F`; `/` stays System's own search.
A copy Fleet left out for not fitting is reached by its file.

**A workflow is drawn as its steps**, top to bottom.
Where Fleet keeps a lifecycle, the steps stand over its lanes instead, read for the picked repository.
On All they are read for the copy's own repository, and Armada's and Yours for the first repository listed.
Each step shows its label, what ends it, when it is passed over, and the words the agent is told there.
The page carries none of the gate boxes, retries or passes the old editor offered, since nothing reads them.

| To | He |
|---|---|
| Change a step's words | Edits them where they stand, and the page saves them in the place that copy is |
| Change a workflow's shape | Types the change under *Tell an agent what to change*; an agent drafts the whole definition, the page shows it beside what it was, and he approves it, sends it back with notes, or dismisses it. On the native editor the drafted definition also lands in his draft, marked as the agent's, and is written by Publish ([Drafts and Publish](#drafts-and-publish)); on the Classic Bridge nothing saves without his Approve |
| Change one Armada carries | In the project's lanes, changes it where it stands: the first change makes his own copy, written to Kit under the same id for every repository on this Mac, and nothing says copy. In the column of steps Armada's rows are read-only, and he presses Make my copy, which writes the same file. Or he presses Propose to Armada, which starts a Job on Armada's repository that lands it as a pull request for every install |
| See how a copy differs | Presses Compare with Armada's, and each step's words show beside Armada's |
| Go back to Armada's | Presses Reset to Armada's on a copy, which replaces the file with Armada's definition |
| Remove a copy | Presses Delete, which removes that one file. A copy that was left out is deleted by its file, since it may declare no id Fleet can read |

**Under each step is its words and what ends it.**
Fleet no longer lists the Jobs that drifted at a step, and no longer drafts a suggestion from them (5 Nov 2026).
A change to a workflow is typed under *Tell an agent what to change*, or proposed by a Job.
A change to one of Armada's own lands in a draft with no scope, and the copy is made when it is published; it can also be made in the lanes, which makes a copy, or be Make my copy or Propose to Armada.
Approve saves the drafted definition at once, and refuses one of Armada's own, which has no file to write.

## What is not a workflow

**Onboarding/Setup is not a workflow**, and is deliberately absent from the catalogue.
Setting up a project is configuration, and it already has a journey — Locate, Scan, Proposal, Write, Verify (see the Set Up a Project (Manifest) user journey).

A workflow gives a Job steps to follow while a person is elsewhere.
During setup you are sitting there reading the proposal and editing it, so steps make no sense for a screen you are already looking at.

The Scan is agent work and could be dispatched, but that does not make the setup a workflow.
