# Workflow

**What it is:** The template a Job may follow: an ordered set of steps that sit in the agent's prompt and in the view as a soft rail, each carrying its mechanical checks, Judge checks and evidence scope.
Fleet reads the current step off what it observed and nudges when one is skipped; nothing blocks.

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
A step may carry `instructions`, the words the agent follows there, sent when the Job enters the step.
A step with `asks_delivery: true` first asks the owner how the work is delivered, unless the first message says or the Job's Delivery, set before Start, settled it; `bug` and `feature` set it on their first step.
At the delivering step the agent delivers by the answer it was told.
Fleet reads the Job's current step off what it observed and nudges the agent about a step skipped, and a step with `judge_checks` is read by a judge on the person's press, advisory and never blocking.
[The rail](session/the-rail.md) owns how a step is placed; [The judges](judge.md) owns the judge.
**An owner can switch the workflow system off** (`features.workflows`), and every Job is then a plain one.

### `feature` and `bug`

The order is the owner's, and a mock is walked before any test is written.
The last column is what the Workflows page draws under each step (`rail_ends.rs`).

**`feature`**

| Step | The agent | Ends when |
|---|---|---|
| `propose` | Shows what it took the request to mean as a sketch, or a rough walk for a look, then stops | you reply |
| `discuss` | Settles the scope with sketches; builds a rough walk when the idea is about the look | a plan is recorded |
| `approve_plan` | Records the plan, with a walk named for each piece he would see | you approve the plan |
| `walk` | Builds the mock in the real Bridge on fixtures and shows it as a walk; writes no test | you approve the walk |
| `refine` | Wires it for real and writes the tests | it is pushed |
| `deliver` | Delivers the way he chose | it is merged |

`walk` is passed over when nothing would be seen.

**`bug`**

| Step | The agent | Ends when |
|---|---|---|
| `find_the_cause` | Reproduces the bug, names the cause, records the fix as tasks | you approve the plan |
| `fix` | Makes the fix; writes no test | a walk is shown |
| `short_walk` | Shows the fix as a short walk straight to the place; writes no test | you approve the walk |
| `tests` | Writes the test that fails without the fix and passes with it | it is pushed |
| `deliver` | Delivers the way he chose | it is merged |

`short_walk` is passed over when nothing would be seen.

### Walks and replies

Three step keys place a Job around the owner.
`crates/core-model/domain/workflowdef-fields.toml` carries them.

**`ends_on`** lists what the owner does that ends the step, any one of them: `replied`, `walk_approved`, `sketch_approved`, `draft_approved` and `idea_picked`.
The agent stops until one holds.
`feature` sets it on `propose`, without `replied`: Propose ends on an approval, never on prose.
`idea_picked` counts only an option the agent's own question gave a preview; Fleet's questions never count.

**`awaits_reply: true`** is `ends_on: ["replied"]`.

**`walks: true`** is the step where he is shown the work and approves it before any test is written.
It is current from the plan's approval until he approves a window the Job showed, and it needs a recording step before it.

**A walk is expected** when the plan said `seen: true`, when it named a `walk`, or when the Job showed a window.
`seen: true` says the person would see the change, and holds nothing.
A `walk` names a page shown before the plan, and holds it.
With none of the three, the walks step is passed over once the work is pushed.

A test edited after the plan is approved and before an expected walk is approved draws one nudge, and nothing blocks.
Where a workflow has a walks step, a rough mock edited before the plan is approved is no skip, but a push or a test then is.
[The rail](session/the-rail.md) owns the nudges.

### Skipping a step

**A person can skip any step of a Job's workflow**, the current one or one still to come, from an icon on the step's header or from its right-click menu, and can undo it.
Fleet then counts the step as passed, so the Job's current step moves past it.
The step shows as skipped, and the timeline enters `skipped: <step label>`.

The agent is sent one message saying the step is not wanted, so it neither does the step nor asks about it, and another when the skip is undone.
**The delivering step cannot be skipped**; Fleet refuses it with `fleet.session_step_not_skippable`.

**This is not the skipped-step nudge.** That nudge is Fleet noticing from what it observed that the agent skipped a step on its own.
A step the person skipped is the person's call and never draws it.

### Where a definition comes from

**Three places, merged by `workflow_id`, a person's own winning**: the set Armada carries, then the repository's own `.armada/workflows/`, then [Kit](kit.md)'s Workflows at `~/.armada/workflows/`.
A repository with no workflows of its own uses the carried set, and one file in either later place replaces a carried definition by id.
**Kit's wins over the repository's**, so a teammate's commit never replaces what a person set up for themselves.
Where Kit's hides a repository definition of the same id that says something else, `list_workflows` carries `hides: "repository"` on it, and Bridge marks its row with the eclipse glyph; two copies that are the same carry nothing.
`config::Catalogue` holds the rule. #425.

**The carried set is compiled in**, and is judged against a repository with tests — Armada's built-in support is for code-with-tests repositories, the owner's decision.
None of them names a Check: they declare `every_manifest_check`, `artifact_exists`, `plan_recorded`, `diff_nonempty` and Judge checks, so each resolves against any Manifest, and a repository declaring no Checks gets none.
See The plan step, below.

**That is Armada holding an opinion it used to refuse.** An empty `.armada/workflows/` was refused because a repository declares how its own work is done, and a carried set is Armada saying how work is done by default.
The trade is accepted, and overriding is one file so that the default does not become the only way.

**A definition that does not fit is left out, not refused, wherever it came from.** One that will not parse, will not resolve against the repository, or shares its id with another file in the same place, is set aside and named when Fleet starts, and the next place down answers for its id — the sentence says whose.
Two files in one place sharing an id are both left out.
The repository's own is held to the same rule as Kit's and Armada's: a file somebody wrote badly does not stop Fleet starting, and the others stand.

**Fleet reads the places a definition comes from again while it runs.** A definition saved or edited under the repository's `.armada/workflows/` or Kit's folder is held without a restart: `save_workflow` reads them before it answers, and a watch on both folders reads them after a hand edit, with the settle window `armada.yml`'s watch uses.
**A re-read leaves out by the same rule as start**: a file that does not fit is left out with its reason and the others stand.

**Saving one is `save_workflow`.** The caller names the place, the repository's own or Kit's, and sends the definition whole.
Fleet parses it against the models this machine offers and resolves it against the Manifest, the parse and the resolve a catalogue does, so what is saved is what would load; a definition that does not fit is refused with the loader's reason and nothing is written.
A definition already holding the id in that place is replaced only where the call says `overwrite`, and the file that held it is the one replaced, whatever it is called.
**Commit to repository copies one of Kit's onto a Job's branch** under `.armada/workflows/`, and Kit's stays ([Kit](kit.md#commit-to-repository)).

**A workflow says which place it came from.** It carries `source` — `armada`, `kit` or `repository` — beside its steps.
Fleet also says it for every workflow when it starts, and `list_workflows` says it with the file, each definition of the same id a more specific place replaced, and the repository definition it hides, so a surface can mark the source and draw the replaced ones apart.
`get_workflow` reads any one of them whole, as the text `save_workflow` takes back.
Fleet's health says whether anything was left out and never how many.

**The full workflow catalogue is `crates/core-model/domain/workflows.toml`**, one row per workflow shape — the shape, gate profile and status, including policy variants and workflows still needing a step list, so there is one list rather than three.
The instantiated `WorkflowDef` sits beside it as JSON in `crates/core-model/domain/workflow-samples/`, where a sample exists.

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
Each step shows its label, what ends it, when it is passed over, and the words the agent is told there.
The page carries none of the gate boxes, retries or passes the old editor offered, since nothing reads them.

| To | He |
|---|---|
| Change a step's words | Edits them where they stand, and the page saves them in the place that copy is |
| Change a workflow's shape | Types the change under *Tell an agent what to change*; an agent drafts the whole definition, the page shows it beside what it was, and he approves it, sends it back with notes, or dismisses it. Nothing saves without his Approve |
| Change one Armada carries | Presses Make my copy, which writes it to Kit under the same id for every repository on this Mac; or Propose to Armada, which starts a Job on Armada's repository that lands it as a pull request for every install. Armada's rows are read-only |
| See how a copy differs | Presses Compare with Armada's, and each step's words show beside Armada's |
| Go back to Armada's | Presses Reset to Armada's on a copy, which replaces the file with Armada's definition |
| Remove a copy | Presses Delete, which removes that one file. A copy that was left out is deleted by its file, since it may declare no id Fleet can read |

**Under each step are the Jobs Fleet nudged there**, each with what it did and Open Job.
Fleet keeps a nudge against the workflow and step it was sent for ([The rail](session/the-rail.md)).
Not a drift stops one counting and leaves it listed.

**A step that drifts twice in seven days gets a suggestion without being asked.**
An agent drafts new words for that step and nothing else.
The step shows a card with the words before and after, and Approve and Dismiss.
A step with drift and no suggestion has *Suggest a change*, which asks for one sooner.
Armada's own rows get none; a change to one is Make my copy or Propose to Armada.
Approve saves the drafted definition, as it does for a typed change.

## What is not a workflow

**Onboarding/Setup is not a workflow**, and is deliberately absent from the catalogue.
Setting up a project is configuration, and it already has a journey — Locate, Scan, Proposal, Write, Verify (see the Set Up a Project (Manifest) user journey).

A workflow gives a Job steps to follow while a person is elsewhere.
During setup you are sitting there reading the proposal and editing it, so steps and nudges make no sense for a screen you are already looking at.

The Scan is agent work and could be dispatched, but that does not make the setup a workflow.
