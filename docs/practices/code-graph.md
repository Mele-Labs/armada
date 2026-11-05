# The code graph

**What it is:** How a change here is checked against GitNexus, the code graph of
this repository — what to ask it before an edit and before a commit, how to
build it, and what it cannot see.

---

Read this before changing a function, type or method that something else calls.

## What it is for

**A grep finds the name. The graph finds what depends on it.** Renaming a field
on a DTO reaches a Rust handler, the generated TypeScript type and a screen that
reads it. A text search finds the first and misses the rest.

| When | Ask | Answers |
|---|---|---|
| Before editing a symbol | `impact`, upstream | Its callers, the flows it sits on, and a risk level |
| Before renaming one | `rename` | Every reference, through the call graph rather than the text |
| Before committing | `detect_changes` | Which symbols and flows the diff touched |
| Finding where something happens | `query`, then `context` | The flow, then one symbol's callers and callees |

The skills under `.claude/skills/gitnexus-*` carry the procedure for each.
`gitnexus-guide` is the one to read first.

## Building it

```sh
pnpm gitnexus:index
```

**What it does:** reads every source file and writes the graph to `.gitnexus/`
at the repository root, which is ignored. It takes about 300 MB.

**One index per repository, in its main checkout.**
Fleet rebuilds it there each time main lands in that checkout, wherever `armada.yml` names the `code_graph` Command under `when_base_moves:`.
`../concepts/manifest.md`, *When the base moves*.
Run it yourself only in a clone no Fleet serves.

## Asking it from a worktree

**A slot asks the main checkout's index by path, and never builds its own.**
Every slot under `.armada/slots/` is a linked worktree of the main checkout, which is the directory holding the shared `.git`:

```sh
dirname "$(git rev-parse --path-format=absolute --git-common-dir)"
```

- **Pass that path as `repo`** on every tool call, as `gitnexus-guide`'s *Bind the repository first* says.
- **Pass your slot as `worktree` to `detect_changes`**, so the diff it reads is yours and not the main checkout's.
- **The index is main's, not your branch's.** A symbol your branch added is not in it, so read an empty answer about one with a text search.
- **An index of its own in a slot is a stale copy.** It answers for the commit the slot had when it was built, so delete it with `node .gitnexus/run.cjs clean --force` in that slot rather than rebuilding it; a bare `clean` only asks to confirm.

**What it needs:** Node and pnpm, and a network the first time — `pnpm dlx`
fetches the pinned version. Nothing else, and no global install.

**What its output means:** it ends by printing the symbol, relationship and flow
counts. A failure naming `@ladybugdb/core` or `tree-sitter` is an install
script pnpm refused to run; the script passes a flag allowing each one it needs.

**It never writes to `AGENTS.md` or `.claude/skills/`.** A bare `gitnexus
analyze` does both — it appends a block to `AGENTS.md` that fails the fifty-line
gate on `CLAUDE.md`, and overwrites the committed skills with whatever version
ran. The script passes `--skip-agents-md --skip-skills` so neither happens.

## The tools in a job

**`.mcp.json` declares the `gitnexus` server, pinned to the same version as the
script.** Claude Code asks once whether to enable it. Without it, every skill
names a CLI fallback, and `pnpm gitnexus <command>` runs any of them.

## What it cannot see

**`risk: UNKNOWN` is not low risk.** An empty caller set also means callers the
index could not resolve: a call through a plain object, dynamic dispatch, or the
seam between Rust and TypeScript, which crosses as JSON. Confirm with a text
search before treating the symbol as unused.

**A stale index answers for the code as it was.**
Main's index trails main by one rebuild, and it is not rebuilt while the main checkout is held on other work.
`node .gitnexus/run.cjs status` in the main checkout names the commit it was built at.

## Upgrading it

**The version is pinned in two places: `package.json` and `.mcp.json`.** Move
both in one commit, then run `pnpm gitnexus analyze --skip-agents-md` once
without `--skip-skills` to refresh the committed skills, and read their diff.

**A refresh reverts every stale-index line to a bare `analyze`.** The committed
skills point at `pnpm gitnexus:index` instead, and the upstream copy does not.
Put those lines back before committing, or the next agent that finds a stale
index rewrites `AGENTS.md`. `verify-foundations` refuses a skill that still
names one, with the line and the replacement.

**A refresh also undoes the local trims (#1253).** Put them back before
committing; no rule checks them, so read the diff for each.

| Skill | The trim to re-apply |
|---|---|
| `gitnexus-debugging`, `-exploring`, `-impact-analysis`, `-refactoring` | Each *Bind the repository first* block is a line linking to `gitnexus-guide`'s, plus what is particular to that skill. Each *Tools* section names only the tools the skill uses and points at the guide's *Tools Reference* |
| `gitnexus-guide` | Holds the one *Bind the repository first* section |
| `gitnexus-cli` | No Spring Actuator or AsyncAPI flags, and no claim that a hook notices a stale index: none is installed |
