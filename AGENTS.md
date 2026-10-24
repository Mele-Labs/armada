# Armada

A macOS app that dispatches coding agents as Jobs against git repositories and
verifies their work before advancing them. Rust daemon, Electron app, one repo.

- **Read `cargo xtask verify-foundations` as a delta against `main`, not as a
  colour.** Red is legitimate — read what each line names, and never chase green.
- **The acceptance test is a milestone's own claim**, written before the code.
  `docs/practices/acceptance-tests.md`.
- **v1 is deleted.** A bare file path in a doc means `git show v1-final:<path>`.
- **Cite a beads issue by its id**, such as `armada-a3f2`: it has no web page.
  An old GitHub `#123` keeps its full link when cited to a person, so it opens
  from the terminal. A bare `#123` stays bare *inside* the repo — docs autolink
  and gates parse it.
- **Never edit the checkout at `main`.** Cut a worktree on a branch before the
  first edit, and land it as a pull request. `.claude/skills/agent-worktrees/`.
- **Ask the code graph what a change breaks, before the edit and before the
  commit.** `docs/practices/code-graph.md`.
- **Load `session-habits` before showing a walk, briefing an agent or offering
  follow-up work.** The owner's corrections live there, not in one machine's memory.
- **Name no other repository, or its internals, in an issue, pull request,
  commit or code** unless a human says to. `docs/instructions/every-change.md`.
- **Never drop or reverse a decision the owner made without asking him first**,
  however good the evidence. `.claude/skills/asking-a-person/`.

| Looking for | Go to |
|---|---|
| **What Armada is for, and what it is not** | **`docs/scope.md`** |
| A binding rule — architecture, errors, config, design, prompts | `docs/contracts/` |
| Rust, Bridge, or the seam between them | `docs/practices/` |
| What is open, and what is waiting on it | Grep `docs/OPEN.md` for a slug or topic; never read it whole |
| A value — tokens, icons, settings | `packages/`, `crates/config/settings.toml` |
| What was measured, and what was not | `docs/spikes/` |
| What is being built, and in what order | Beads issues (`bd list`), with milestones as epics. `docs/practices/joining.md` |
| What a thing is, and how it is used | `docs/concepts/`, `docs/journeys/` |
| Skills and subagents, and which model each runs on | `.claude/skills/`, `.claude/agents/`, `plugins/armada/agents/`, `docs/practices/subagents.md` |
| Landing a branch on `main` | `docs/practices/ci.md`, a pull request merged on a green `ci` |

Everything written down is in `docs/INDEX.md`, and the gate refuses one that is
not. `ARCHITECTURE.md` is the map. `CLAUDE.md` symlinks here — a copy drifts.

**This file routes. It does not explain.** Anything longer than a pointer
belongs in a contract, a practice doc or a skill.
