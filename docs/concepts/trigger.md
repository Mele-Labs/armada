# Trigger

**What it is:** A Command, a skill or a prompt tied to a moment in a workflow's steps, named in a file a person wrote. Fleet loads, lists, saves and removes them; nothing runs one.

---

**Kind:** Concept.

**Built:** the model, the loader, the wire to Bridge, and Bridge's saved Triggers.

## What a Trigger is

| Field | Values | Notes |
|---|---|---|
| `name` | text | Part of its identity |
| `when` | `step_starts`, `step_passes`, `pr_opened` | `pr_opened` is the delivering step's entry |
| `workflow` | a `workflow_id` | Absent is every workflow |
| `step` | a step id | Absent is every step. Not allowed with `pr_opened` |
| `command`, `skill` or `brief` | a name, a name, a prompt | One of the three. `brief` is a prompt, and is refused naming the key when empty |
| `on_failure` | `block`, `repair` | Both default to false |

A `brief` Trigger has no name of its own to take from what it names, so Bridge names it for the first four words of the prompt.

## Where one is set

> **Rule.** The most specific level wins, and its copy replaces the others whole.
> Why: a field merge makes a Trigger nobody wrote.

| Level | Where | Notes |
|---|---|---|
| Armada | Compiled in | Ships none |
| Repository | `.armada/triggers/` | Read from `main` only |
| This machine | `~/Library/Application Support/Armada/machine/triggers/` | Machine, not [Kit](kit.md): it does not travel |

**Identity is when, step and name.** The workflow a Trigger applies to is not part of it.

> **Rule.** A repository's Triggers are read from `main` and never from a branch.
> Why: Fleet reads a repository's files from the base branch.

## What becomes of one on load

| Case | Result |
|---|---|
| Names a Command the repository declares | Loads |
| Names a Command it does not declare | Skipped, marked as such |
| The file does not parse | Left out with its reason, the others stand |
| Two files in one place share an identity | Both left out, named together |

## On the wire

Protocol 23.58, the four operations. `docs/practices/protocol.md`.

| Operation | What it does |
|---|---|
| `list_triggers` | What a repository holds, each with its level and file, what it replaced, and the files left out with why |
| `get_trigger` | One Trigger as YAML text, from any level that holds a copy |
| `save_trigger`, `remove_trigger` | Write or delete a file in the repository's folder or this machine's |

> **Rule.** A save is checked with the loader's rules before anything is written.
> Why: a Trigger that is saved is one that loads.

**A repository's save refuses a Command its Manifest lacks, and a machine's does not.** The loader skips a Trigger like that so the same file is right in the next repository.
A machine's applies to every repository, so refusing it for the one in front of a person would undo that.
Its answer says it is skipped here.

> **Rule.** A repository's Trigger takes effect once it is on `main`.
> Why: Fleet reads a repository's files from the base branch. A save writes the checkout and answers `waits_for_main`, and a removal there waits the same way.

Nothing is held, so a save is on the next `list_triggers`.

## In Bridge

| Where | What it draws |
|---|---|
| A workflow's canvas | A Trigger that applies to every workflow is a leaf off the step it names, on each workflow's canvas, marked with the every-workflow glyph. It opens the editor: When, Step, Applies to, Set in, Runs, and the two switches under If it fails |
| A workflow's step | The Triggers set for this workflow that name it, one line a moment (On start, On pass, PR opened) |

A copy a more specific level replaced is drawn struck through under the one that applies.
A repository's save is marked as waiting for `main`, and a machine Trigger on a Command the repository lacks is marked skipped.

The repository's files are read from the base branch by `adapters::triggers_on_base`, and this machine's by `armada::Locator`.
The loader is `config::TriggerCatalogue`, the type is `core_model::Trigger`, and the decision is `.claude/decisions/2026-10-07-a-trigger-runs-at-a-moment-in-a-job.md`.
