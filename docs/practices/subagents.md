# Subagents

**A job hands each piece of work to the most specific subagent that does it,
on the cheapest model the work allows.** The job rules
(`crates/adapters/src/session_rules.md`, `docs/contracts/agent-prompt.md` 5f)
tell every hosted job this, in Armada and in any repository Armada works
in. This doc is the fleet those rules point at.

## Where a type comes from, in order

| Order | Where | Named | Reaches |
|---|---|---|---|
| 1 | The repository's `.claude/agents/` | `rust-engineer` | That repository only. Its expertise is written down here |
| 2 | The Armada mod, `plugins/armada/agents/` | `armada:mechanic` | Every repository where the mod is installed |
| 3 | The harness's built-ins | `Explore`, `Plan`, `general-purpose` | Everywhere |

A plan's group names the type the job will dispatch (`record_plan`'s
`agent`), so the person sees on the plan who does each group.

## Which model

| Tier | Model | The work |
|---|---|---|
| Find and report | `haiku` | Locate code, run a command and report what failed, read and summarise |
| Decided change | `sonnet` | A rename or move, a merge or conflict, a doc or registry edit, a test for a named behaviour, a fix whose cause is known |
| Undecided change | the job's own | Design, visual work, a change across the Fleet-to-Bridge seam or any process boundary, anything the brief cannot spell out |

**Pass `model` on the Agent call whenever the work's tier differs from the
type's own.** A type's `model:` frontmatter is its default, not a ceiling: the
Agent tool's `model` wins over it. The owner asked for the split on 2 Oct 2026
(`work-issue`, *Pick the model by the work*); haiku was added on 17 Oct 2026.

## The mod's fleet

Portable: each reads the repository's own `CLAUDE.md` or `AGENTS.md` first, and
that file's rules beat the agent's.

| Type | Model | Takes |
|---|---|---|
| `armada:implementer` | job's | A change whose edits are still to be worked out |
| `armada:mechanic` | `sonnet` | A change already decided, applied everywhere it applies |
| `armada:test-writer` | `sonnet` | Tests for a named behaviour, each shown red first |
| `armada:ci-fixer` | `sonnet` | One red check, reproduced, root-caused, fixed |
| `armada:docs-writer` | `sonnet` | A document whose content is decided |
| `armada:test-runner` | `haiku` | Runs a command and reports only the failures |
| `armada:explorer` | `haiku` | Finds where something is and how it works, graph first, and reports `path:line`; use it in place of the built-in `Explore` |

## The mod's skills

Each mod type preloads the skills its work needs (`skills:` in its frontmatter,
`plugins/armada/skills/`), so a rule lives in one skill and not in every agent
that follows it. A job can load any of them too, through the Skill tool.
Like the agents, each defers to the repository's own `CLAUDE.md` or `AGENTS.md`.

| Skill | Carries |
|---|---|
| `armada:reading-a-brief` | Find your place by search and line range, stay in the named paths, commit and push each passing piece, ask one `**QUESTION:**` line rather than decide |
| `armada:verify-before-done` | Run the repository's checks on what you touched, show a fix red before green, re-read your own diff, report what ran and what it read |
| `armada:root-cause` | Reproduce first, "flaky" is not a cause, never skip or disable a test, the smallest fix that addresses the cause |

| Type | Preloads |
|---|---|
| `armada:implementer` | `reading-a-brief`, `verify-before-done`, `root-cause` |
| `armada:mechanic` | `reading-a-brief`, `verify-before-done` |
| `armada:test-writer` | `reading-a-brief`, `verify-before-done` |
| `armada:ci-fixer` | `root-cause`, `verify-before-done` |
| `armada:docs-writer` | `reading-a-brief` |
| `armada:test-runner` | none: it runs one command and changes nothing |
| `armada:explorer` | none: it reads and reports |

A type names each skill in full, `armada:<skill>`. The harness matches a
`skills:` entry against skill names exactly before anything else, so a bare
`verify-before-done` would pick up a repository's own skill of that name, and
on an older harness, such as 2.1.42, would miss the mod's skill altogether.

## Armada's own

| Type | Model | Takes |
|---|---|---|
| `rust-engineer` | job's | `crates/`, `xtask/` |
| `bridge-engineer` | job's | `apps/`, `packages/` |
| `protocol-engineer` | job's | Anything crossing the Rust/TypeScript wire |
| `docs-writer` | `sonnet` | `docs/`, `.claude/skills/`, issue bodies |
| `fixture-author` | `sonnet` | One testkit NDJSON fixture |
| `judge` | `sonnet` | Fleet's, never a plan group's |

The three engineers run on the job's own model, so design and seam work
is safe when a dispatch names no model (owner, 18 Oct 2026). Mechanical work on
them passes `model: "sonnet"`, or goes to `armada:mechanic`.

## Adding a type

One file in `.claude/agents/` (this repository's) or `plugins/armada/agents/`
(every repository's), with `name`, a `description` that says what work it takes
and when, and `model` when the work has one tier. A mod change bumps
`MOD_VERSION` and `plugin.json` together. A rule two types share goes in a
skill under `plugins/armada/skills/`, named in each type's `skills:`.

## Open questions

- A plan's group names a type but not a model, so the person cannot see or mark
  which tier a group runs on.
