# Subagents

**A session hands each piece of work to the most specific subagent that does it,
on the cheapest model the work allows.** The session rules
(`crates/adapters/src/session_rules.md`, `docs/contracts/agent-prompt.md` 5f)
tell every hosted session this, in Armada and in any repository Armada works
in. This doc is the fleet those rules point at.

## Where a type comes from, in order

| Order | Where | Named | Reaches |
|---|---|---|---|
| 1 | The repository's `.claude/agents/` | `rust-engineer` | That repository only. Its expertise is written down here |
| 2 | The Armada mod, `plugins/armada/agents/` | `armada:mechanic` | Every repository where the mod is installed |
| 3 | The harness's built-ins | `Explore`, `Plan`, `general-purpose` | Everywhere |

A plan's group names the type the session will dispatch (`record_plan`'s
`agent`), so the person sees on the plan who does each group.

## Which model

| Tier | Model | The work |
|---|---|---|
| Find and report | `haiku` | Locate code, run a command and report what failed, read and summarise |
| Decided change | `sonnet` | A rename or move, a merge or conflict, a doc or registry edit, a test for a named behaviour, a fix whose cause is known |
| Undecided change | the session's own | Design, visual work, a change across the Fleet-to-Bridge seam or any process boundary, anything the brief cannot spell out |

**Pass `model` on the Agent call whenever the work's tier differs from the
type's own.** A type's `model:` frontmatter is its default, not a ceiling: the
Agent tool's `model` wins over it. The owner asked for the split on 2 Oct 2026
(`work-issue`, *Pick the model by the work*); haiku was added on 17 Oct 2026.

## The mod's fleet

Portable: each reads the repository's own `CLAUDE.md` or `AGENTS.md` first, and
that file's rules beat the agent's.

| Type | Model | Takes |
|---|---|---|
| `armada:implementer` | session's | A change whose edits are still to be worked out |
| `armada:mechanic` | `sonnet` | A change already decided, applied everywhere it applies |
| `armada:test-writer` | `sonnet` | Tests for a named behaviour, each shown red first |
| `armada:ci-fixer` | `sonnet` | One red check, reproduced, root-caused, fixed |
| `armada:docs-writer` | `sonnet` | A document whose content is decided |
| `armada:test-runner` | `haiku` | Runs a command and reports only the failures |

## Armada's own

| Type | Model | Takes |
|---|---|---|
| `rust-engineer` | `sonnet` | `crates/`, `xtask/` |
| `bridge-engineer` | `sonnet` | `apps/`, `packages/` |
| `protocol-engineer` | `sonnet` | Anything crossing the Rust/TypeScript wire |
| `docs-writer` | `sonnet` | `docs/`, `.claude/skills/`, issue bodies |
| `fixture-author` | `sonnet` | One testkit NDJSON fixture |
| `reserved-auditor` | `sonnet` | Reading v1; the only type that may |
| `judge` | `sonnet` | Fleet's, never a plan group's |

The three engineers default to `sonnet`; design or seam work on them is
dispatched with the session's model passed by name.

## Adding a type

One file in `.claude/agents/` (this repository's) or `plugins/armada/agents/`
(every repository's), with `name`, a `description` that says what work it takes
and when, and `model` when the work has one tier. A mod change bumps
`MOD_VERSION` and `plugin.json` together.

## Open questions

- A plan's group names a type but not a model, so the person cannot see or mark
  which tier a group runs on.
