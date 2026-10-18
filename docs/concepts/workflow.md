# Workflow

**What it is:** The template a Job may follow: an ordered set of steps that sit in the agent's prompt and in the view as a soft rail, each carrying its mechanical checks, Judge checks and evidence scope. Fleet reads the current step off what it observed and nudges when one is skipped; nothing blocks.

---

**Kind:** Entity.

How a workflow is structured and where a definition comes from. [Judge](judge.md) owns the semantic tier and [Job](session.md), *The rail*, owns how a Job follows one; this document owns the definition.

## Core principle

**Self-report is a signal, not a source of truth.** The step a Job is on is read from what Fleet observed — the plan, the build record, the Job's own Check runs and commits — and never from the agent's word.

## Workflow schema

Every workflow is a `WorkflowDef`: an ordered set of steps, and nothing else declares its shape. A Manifest may shadow a built-in workflow, so `workflow_id` alone does not identify what a Job follows; `source` says which place it came from.

**The full field-by-field schema is `crates/core-model/domain/workflowdef-fields.toml`.** It carries every top-level field and every field nested under `steps[]`, with its type, whether it is required and the reasoning behind it. One row per field, nested the way the schema nests: the top-level rows are the `WorkflowDef`'s own fields, and everything under `steps[]` is a field of a step.

## The workflows

### How a Job gets one

**You do not have to pick.** The [Proposer](job-proposer.md) reads the request a Job is dispatched with and proposes which workflow the work should run under, or none. Picking one by hand stays available and is the override, not the path. That document owns the call.

A Job may follow one workflow, which the proposer picks at dispatch or a person sets. No step gates it. Fleet reads the Job's current step off what it observed and nudges the agent about a step skipped, and a step with `judge_checks` is read by a judge on the person's press, advisory and never blocking. `session.md`, *The rail* and *The judges*. **An owner can switch the workflow system off** (`features.workflows`), and every Job is then a plain one.

### Where a definition comes from

**Three places, merged by `workflow_id`, the most specific winning**: the set Armada carries, then [Kit](kit.md)'s Workflows at `~/.armada/workflows/`, then the repository's own `.armada/workflows/`. A repository with no workflows of its own uses the carried set, and one file in either later place replaces a carried definition by id. `config::Catalogue` holds the rule. #425.

**The carried set is compiled in**, and is judged against a repository with tests — Armada's built-in support is for code-with-tests repositories, the owner's decision. None of them names a Check: they declare `every_manifest_check`, `artifact_exists`, `plan_recorded`, `diff_nonempty` and Judge checks, so each resolves against any Manifest, and a repository declaring no Checks gets none. See The plan step, below.

**That is Armada holding an opinion it used to refuse.** An empty `.armada/workflows/` was refused because a repository declares how its own work is done, and a carried set is Armada saying how work is done by default. The trade is accepted, and overriding is one file so that the default does not become the only way.

**A definition that does not fit is left out, not refused, wherever it came from.** One that will not parse, will not resolve against the repository, or shares its id with another file in the same place, is set aside and named when Fleet starts, and the next place down answers for its id — the sentence says whose. Two files in one place sharing an id are both left out. The repository's own is held to the same rule as Kit's and Armada's: a file somebody wrote badly does not stop Fleet starting, and the others stand.

**Fleet reads the places a definition comes from again while it runs.** A definition saved or edited under the repository's `.armada/workflows/` or Kit's folder is held without a restart: `save_workflow` reads them before it answers, and a watch on both folders reads them after a hand edit, with the settle window `armada.yml`'s watch uses. **A re-read leaves out by the same rule as start**: a file that does not fit is left out with its reason and the others stand.

**Saving one is `save_workflow`.** The caller names the place, the repository's own or Kit's, and sends the definition whole. Fleet parses it against the models this machine offers and resolves it against the Manifest, the parse and the resolve a catalogue does, so what is saved is what would load; a definition that does not fit is refused with the loader's reason and nothing is written. A definition already holding the id in that place is replaced only where the call says `overwrite`, and the file that held it is the one replaced, whatever it is called. Where both Kit and the repository define an id, the repository's runs.

**A workflow says which place it came from.** It carries `source` — `armada`, `kit` or `repository` — beside its steps. Fleet also says it for every workflow when it starts, and `list_workflows` says it with the file and each definition of the same id a more specific place replaced, so a surface can mark the source and draw the replaced ones apart. `get_workflow` reads any one of them whole, as the text `save_workflow` takes back. Fleet's health says whether anything was left out and never how many.

**The full workflow catalogue is `crates/core-model/domain/workflows.toml`**, one row per workflow shape — the shape, gate profile and status, including policy variants and workflows still needing a step list, so there is one list rather than three. The instantiated `WorkflowDef` sits beside it as JSON in `crates/core-model/domain/workflow-samples/`, where a sample exists.

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

## What is not a workflow

**Onboarding/Setup is not a workflow**, and is deliberately absent from the catalogue. Setting up a project is configuration, and it already has a journey — Locate, Scan, Proposal, Write, Verify (see the Set Up a Project (Manifest) user journey).

A workflow gives a Job steps to follow while a person is elsewhere. During setup you are sitting there reading the proposal and editing it, so steps and nudges make no sense for a screen you are already looking at.

The Scan is agent work and could be dispatched, but that does not make the setup a workflow.
