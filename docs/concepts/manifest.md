# Manifest

**What it is:** How Armada runs a repository, so an agent does not guess: the Commands and ports it knows, the Checks a change must pass where the repository declares any, and which Skills, sub agents and secrets it may use.
Backed by an `armada.yml`, which a repository may not have. Bridge names it Armada configuration.
Extends or restricts Kit defaults.

---

**Kind:** Entity.

Defines Manifest — the per-project/repo definition of how Fleet and agents may interact with a given codebase. Companion to the main Armada brief and `../contracts/configuration.md`.

## What it is

A Manifest defines a project's shape and boundaries — checks it must pass to land code, commands available to run against it, and which Skills/Agents/secrets it can use.
It is backed by an `armada.yml` file.

It is not the same shape as either tier above it. [Kit](kit.md) is the tool set you bring and holds defaults a Manifest may extend or restrict; [Machine](machine.md) is how this installation behaves and has no project-level counterpart at all.

## Armada remembers how a repository runs

**A Manifest exists so an agent stops guessing how to run a repository's tests and commands.** Checks are one part of it and gate a change only where the repository declares them.
A repository needs none to start a Job: Fleet serves a folder with no `armada.yml` under an implicit Manifest, with no Check and no Command ([Fleet](fleet.md), *Fleet starts with none*).

| Source | What Armada keeps |
|---|---|
| Jobs | Each command a Job ran, its exit and the ports it listened on |
| Past agent sessions | The same, read from the sessions an agent CLI kept in the folder |
| Setup | Lines each traced to a file Scan read, written to `armada.yml` |

> **Rule.** Setup is offered and never required.
> Why: a Job starts on any folder, and what Jobs run is remembered without it.

What passed is kept for each workspace and a command whose every run failed is left out.
A dropped line is never offered again.
The next Job in the repository is told what was kept, in its system prompt; `get_manifest` does not serve remembered runs, and none becomes a Check.
[Kit](kit.md), *What a repository runs*, owns the rules, the four places it is kept, how a Job is told and what Bridge shows of it.

## Kit's copy

**A person can keep a repository's `armada.yml` in [Kit](kit.md) instead of committing one**, as `~/.armada/manifests/<owner--name>.yml`.
It is keyed the way the repository's run memory and tiles are: the remote's `owner--name`, or the path where there is no remote.
It is for a repository whose team does not use Armada, and nothing is written to the repository for it.

| The repository has | What runs |
|---|---|
| No `armada.yml` | Kit's copy is the Manifest |
| An `armada.yml` | The repository's, with each Check and Command Kit's copy names replaced by Kit's of the same name, each one only Kit's copy names added, and Kit's `tracker` and `ci_gate` in place of the repository's |

**A person's own wins over the repository's, and the answer says so.** The Manifest's summary carries `kit`: Kit's file, each name read from it, and `hides` on each one the repository declares differently.
A copy that is the same hides nothing.
A Check or Command Kit's copy names that the repository does not declare is added to the Manifest the person's Jobs run against, and is listed in `names` as `added`, hiding nothing.
**An added Check is available and does not gate unless Kit's copy marks it `gate: true`.**
It is listed, `armada check <name>` runs it and Bridge's Checks show it, and `every_manifest_check` and the gate `armada check` runs over a diff leave it out.
The `kit.names` entry carries `gates` for it, so a person sees which of their own Checks protect their Jobs.
A Check Kit's copy replaces by name gates as the repository's did, and `gate` on it is refused, as it is on any Check a repository declares: `gate` means nothing where there is nothing to add.
A Command Kit's copy adds is available to a Job as any Command is.
**Kit's `tracker` and `ci_gate` replace the repository's whole**, so a person whose issues are in Jira says so without committing it; nothing of the repository's block is kept in Kit's.
A copy that is not the whole file holds only `checks`, `commands`, `tracker` and `ci_gate`.
The rest of the repository's file stands, its id, base and policies among it.

**A Kit copy that will not merge is left out and named, and the repository's own stands.** Fleet says it at start, and the summary carries the sentence as `left_out`.
A repository's own file that will not load is refused as it always is, whatever Kit holds.

**It is read where the repository is served, and on each re-read of its `armada.yml`.** An edit to Kit's copy alone is read on the next restart.
**It is kept in the place the repository's tiles are kept in**: this Mac, a synced folder, or a bucket ([Kit](kit.md), *Flight controls*).
It is `manifests/<owner--name>.yml` under Kit's folder, under the synced folder, or under the bucket's prefix, and it is read and written there.
**Where the repository's place is This Mac, or none was chosen, it stays in `~/.armada` on this Mac.**
**A place that cannot be read reads as no Kit copy, and says so**: a synced folder that is not mounted, a bucket that refuses or is out of reach.
The repository's own `armada.yml` stands, and the summary's `left_out` carries the sentence naming the place.
A repository with no `armada.yml` and a place that cannot be read is not served for the Manifest, and Fleet's log says why.
The summary's `kit.file` is the copy's address: a path on this Mac, or the bucket's `s3://`, `r2://` or `gs://` address.
**A setup Session keeps it with `save_kit_manifest`**, which writes the file whole at that place after the person has said it is theirs and confirmed it (`docs/practices/protocol.md`, *A setup Session keeps the person's own armada.yml*).
**Commit to repository** copies Kit's file to `armada.yml` at the workspace root on the Job's branch, and Kit's copy stays ([Kit](kit.md), *Commit to repository*).
A copy that names only the Checks and Commands it changes is not a whole Manifest, and is refused.
`armada check` lays it over the repository's file too, so a Job's Check on this Mac runs what its Manifest was served with.
A machine with no Kit copy, CI among them, runs what the repository declares.

## Workspace mapping

**One Manifest per workspace.** Even inside a large monolith with multiple independent components, each workspace gets its own `armada.yml`, same as a standalone repo.
Components typically share one lockfile at the repo root under pnpm/yarn/npm-style workspaces.
Only the package manifest (e.g. `package.json`) is truly per-workspace; the shared lockfile is evidence common to all of them, not a per-workspace artifact.

### Root `armada.yml` (monorepos only)

A monorepo may additionally have a root-level `armada.yml`, separate from any workspace's own.
It is created only when Setup detects multiple workspaces — a simple, single-workspace repo's Manifest *is* the root, and no separate file exists.

**The root holds commands** — bootstrap/install, for example — **and Checks for the artefacts the root itself owns.** Root Checks exist because the root owns things nothing else gates:

- A change to a root test or CI command touches no workspace, so no workspace's Checks apply and the highest-blast-radius change in the repo has an empty mechanical tier.
- The shared lockfile is written by any workspace's change and read by every workspace's Checks, with no owner.
- A repo-wide Check — *do all workspaces build together* — belongs to no workspace by construction.

**Root Checks run only when the diff touches root paths.** Why: without it the root becomes a place to put a Check that gates every change in the repo, and one slow Check there taxes every workspace.
A root Check is scoped to root-owned artefacts, never promoted to a repo-wide gate.

**Which paths the root owns — nearest ancestor wins.** The nearest `armada.yml` up the directory tree owns a path; where no workspace claims it, the root does.
Root paths are therefore exactly the paths no workspace claims.

A `/tooling` directory belonging to no workspace is a root path, gated by root Checks, and ungated only where the root declares none — the same state as a workspace with no Checks.

**Ownership is not exclusivity.** The shared lockfile is owned by the root under this rule, but *owned* means only *gated by root Checks*.
Nothing reserves write scope, so two concurrent Jobs can still both write it.

**Per-workspace Checks are unchanged.** Every workspace still declares its own in its own `armada.yml`; root Checks are additional, not a replacement, and a workspace's Checks are never hoisted to the root.

Still open: whether the root *owns* the lockfile as opposed to merely being able to gate it — narrowed rather than closed by the nearest-ancestor rule, which settles gating and not exclusivity (see Open questions).

**The root gives repo-wide build-orchestration evidence a place to be cited from** — a root `Makefile`, `turbo.json`-style task runners — without duplicating it into every workspace's file.

**Armada only records *how to invoke* a tool.** It never replaces or centralizes the tool's own config resolution: ESLint resolving its own root `.eslintrc` up the directory tree is invisible to, and irrelevant for, Armada.

**A workspace declares a dependency on a root command explicitly**, as `setup.requires: [bootstrap]` in its own `armada.yml`.
It is a plural list, since a workspace may require more than one root command.

**Fleet tracks whether a required root command has already run for a given worktree**, and re-runs it when **either** the worktree is new, or the evidence backing the root command — the lockfile/manifest Scan traced it from — has drifted since it last ran.
That is the same signal Verify's drift detection computes.

### Workspace gating

Decided 2 Oct 2026. **A manifest gates a change when the diff touches a path it owns, or a path matching one of its `depends_on` globs.** Within a gating manifest each Check still applies its own `when`, and runs in that manifest's directory.

```yaml
# packages/a/armada.yml
depends_on: ["packages/lib/**", "pnpm-lock.yaml"]
checks:
  test:
    run: pnpm test
    when: ["src/**"]
```

- **`depends_on` is a list of repository-relative globs in the `when:` dialect**, which the Configuration contract states once.
  A pattern from another dialect is refused at load.
  An empty list is refused, as `when: []` is.
- **It is not transitive.** If `a` depends on `lib` and `lib` depends on `core`, a change to `core` gates `lib` and not `a`.
  Why: a transitive closure makes the gating set depend on every manifest's list at once, so a line edited in one file changes what an unrelated file gates, and nobody can read a manifest's reach from the manifest.
  A manifest that is affected by `core` says so itself.
- **`when:` in a workspace file is relative to that workspace's directory.** A pattern like `src/**` in a workspace file means `src/**` under that workspace's directory.
  `depends_on` is the one key written repository-relative, because it names paths the manifest does not own.
- **A `depends_on` hit runs the Checks that declare no `when`.** A workspace's `when` is read from its own directory, so it cannot match a path the manifest does not own.
  A Check meant to run when a dependency changes writes no `when`.
- **A root Check's path condition is implicit.** It runs when the diff touches a root-owned path, which is a path no workspace claims, and the root's own `when:` narrows that further.
  Root `when:` is repository-relative and is intersected with root ownership; there is no key to write the ownership half.
- **A workspace may name a root Command in `setup.requires`.** The name resolves against the workspace's own Commands first and then the root's.
  Where only the root declares it, it runs in the root, once per worktree, and not in the workspace.
- **`setup.worktrees` and `setup.seed` are refused in a workspace file.** Each is read from the root, so a second value would be one nothing reads.
- **An empty diff gates nothing.** Decided 6 Oct 2026. A change that touched no files runs no Checks, in every repository, and `config::gating` returns no manifest for it.
  A Check with no `when` still reads as always on a non-empty diff; the two are kept apart by asking the gating set first and never handing an empty path list to `Covers::reach`.
- **A workflow resolves over a gating set.** `ResolvedWorkflow::resolve_gated` expands `every_manifest_check` once per manifest it is given, in the order given, and each frozen Check carries the directory of the manifest that declared it (empty for the root).
  A step that names a Check by name still resolves it against the root.
  `resolve` is the same over the root alone, so a repository with no workspaces freezes what it always did.
  Fleet resolves every workflow over the root and every workspace it loaded, at start and each time the workflow folders are read again.
- **Fleet loads the workspaces when it sets a repository up.** It walks as Scan does, to the same depth and by the same package-file rule, and reads each directory's `armada.yml` as a workspace file.
  A file that will not load is left out, the repository is served and its other workspaces stand; at start the console names the file and why, and after a root re-read the same is published as a reading of that file, held with the root's.
  Workspaces are read again whenever the root's `armada.yml` is re-read and adopted; a refused root leaves them as they were.
  That does not resolve a workflow again, so a workspace added after start gates but declares no Check in an already-resolved workflow until the workflow folders are next read.
- **It reads the changed paths.** Where workspaces exist, `config::gating` over the diff says which manifests gate, and an empty diff gates none.
  A repository with no workspaces gates as it always did, so an empty diff there still runs a Check that declares no `when`.
- **A Check runs in its manifest's directory**, with its prerequisites and its `one_test`.
  Its `when` and its narrowing read the paths that manifest owns, relative to it, so a root Check never sees a workspace's path.
- **A Check is keyed by its manifest and name.** `ResolvedCheck::key` is the bare name for the root's and `<dir>:<name>` for a workspace's, and every row, live entry, past duration and reused answer is found by it.
- **A manifest the change does not gate skips its Checks.** The workflow resolves once, over every manifest, and a run skips the Checks whose manifest the diff does not reach.
- **A workspace Check's runner `dir` is omitted, and means that workspace.** A narrowed run of a Check naming `runner: {name: vitest}` puts `.` in the template's `{dir}` and the paths the manifest owns, relative to it, in `{files}`: `pnpm --dir . exec vitest related src/a.ts ...`, run in the workspace directory.
  A root Check with no `dir` has none to put there and runs whole, as before.
  `armada check --changed` is not read through a runner at all, only through `narrow`.
- **`armada check` takes the key.** `armada check a:test` runs `a`'s `test` in `a`'s directory, with its prerequisites, its `one_test` and its `--changed` narrowing all read there.
  The bare name is the root's, and the root's is run from the root.
  `armada run a:gen` does the same for a Command.
  A key whose directory is no workspace reads as a bare name, and is refused as one.
- **`armada covers` prints keys.** It reads the changed paths on stdin and answers with the keys of the Checks that cover them: the root's first, then each gating workspace's in directory order.
  Each manifest is asked about what it owns of the change, relative to it, so a root Check never sees a workspace's path.
  A repository with no workspace `armada.yml` answers as before, and its empty diff still names a Check with no `when`.
  Where workspaces exist, an empty diff names nothing, as above.
- **The merge line sets up in the root, then in each gating workspace.** Decided 6 Oct 2026. Once a Check is about to run it runs the root's `setup.requires`, then, for each workspace that has a Check in the set, that workspace's own, in directory order.
  **A Command a workspace names that only the root declares runs in the root**, once per worktree, and a Command already run is not run again.
  A workspace's own Command runs in its directory.
  A workspace's Checks run in its directory whichever Command prepared it.
  The same setup runs on `main`'s rerun of a Check that failed.
- **`armada check` passes no manifests, so it gates nothing.**
- **A workspace Verify reads its file against the root's**, so its `setup.requires` may name a root Command.
  The Verify still runs that entry in the workspace's directory; running it once in the root directory is the merge line's.

### Seeding a worktree's build

**`setup.seed.paths` names the build directories a new slot is cloned with**, copy-on-write.
A slot on a volume that cannot clone starts cold and says why — a seed is never copied in full, and never shared.
A repository that declares no seed gets none.

### How many worktrees a repository leases

**`setup.worktrees` is the size of the repository's pool of warm worktrees**, which `armada worktree lease` hands out — [Fleet](fleet.md), *Worktree slots*.

```yaml
setup:
  worktrees: 4
```

- **Absent means eight.**
- **Zero, a negative number and anything that is not a whole number are refused at load.**
- **Read from the root `armada.yml` at every lease**, so a change applies to the next one. Lowering it leaves the slots above the new number on disk and unleased.
- **A machine's own pool overrides it.** Once a person adds or removes a slot, from Cleanup or `armada worktree add|remove`, that machine's list of slots stands in for this number, and a change here no longer moves it — [Fleet](fleet.md), *Worktree slots*.
- **It is enough on its own**: `setup` with `worktrees` and nothing else needs no `requires`.

## Registries

Separate registries, not the same thing tagged two ways:

| Registry | Purpose | Invoked by |
| --- | --- | --- |
| Checks | Mandatory — must pass to land or advance code | Fleet, as part of mechanical verification |
| Commands | Optional, general-purpose — migrations, doc generation, builds, formatting, servers | An agent, and you directly via Bridge |
| Ports | Names a port a workspace needs, so Armada can place it | Nothing invokes it — Fleet reads it at claim time |

Checks are what `armada check` runs, for an agent in its slot and for the merge line — see [Workflow](workflow.md) for the steps that name them.

Commands cover anything project-specific. Armada doesn't reimplement those tools; it gives Fleet, agents and you one consistent way to invoke them, through the same named registry either way.

### Command approval

A Command can be flagged destructive.
That flag gates **agent** invocation — pauses for your approval, same as any other risky allowlisted op.
Your own **manual** invocation via Bridge doesn't require a second approval step, since you're already the one directly triggering it.

**A declared teardown Command is the escape hatch** for a process handed to another supervisor — `launchctl`, a systemd unit, a deliberate double-fork.
Everything Armada spawned that stayed in its own tree is killed by process group with no declaration needed, and containers are handled by the Docker adapter, so a declared Command covers only what neither reaches.

### Commands that keep running

A Command with `serve` is a server: it stays up until something stops it.
The shape follows Tilt's [`local_resource`](https://docs.tilt.dev/api.html#api.local_resource), which draws the same line between a command expected to exit and one expected to stay up.

```yaml
ports:
  storybook: {}

commands:
  storybook:
    serve: pnpm -C packages/components exec storybook dev -p ${port.storybook} --no-open --ci
    ready: curl -sf http://localhost:${port.storybook}
    links:
      - url: http://localhost:${port.storybook}
        name: Storybook
```

| field | type | required | default | meaning |
|---|---|---|---|---|
| `run` | string | no | — | Runs first; the server starts only if it exits zero |
| `serve` | string | yes, for a server | — | Starts the server, held until stopped |
| `ready` | string | no | serving once started | Exits zero once the server answers |
| `links` | list | no | empty | Addresses offered as buttons, each a `url` and optional `name` |

`${port.NAME}` resolves in every field from the claim the server runs under — see Ports, below. [Fleet](fleet.md) holds each server: one instance per checkout.

> **Rule.** A server that exits on its own has failed, whatever its exit code.
> Why: staying up is the whole of what a server is for.

> **Rule.** An agent starts a server only through Fleet's MCP tool, never from its shell.
> Why: Fleet has to know a server is running to hand it to the next request and to stop it.

### Check prerequisites

**A Check may require Commands to run before it.** `checks.<name>.requires` is an ordered list of **Command names** from this Manifest's own Commands registry.

An end-to-end Check that needs `migrate` and `seed` to have run first has no other way to say so: `setup.requires` (see Root `armada.yml`) is per *worktree* and runs once, so it cannot express per-Check ordering.

Rules that follow:

- **Names, never command strings.** A prerequisite points at a Commands entry, so what actually runs is written in exactly one place and stays in step when the Command is edited.
- **Ordered.** The list runs in the order given; two Commands where one seeds what the other migrated are not interchangeable.
- **A failure is attributed to the prerequisite.** A Check whose prerequisite failed reports the prerequisite as the failure, not the Check — otherwise a broken `migrate` reads as a broken test suite.
- **A prerequisite may name a root Command**, the same as `setup.requires`.
- **A prerequisite that has already run in the same execution context is skipped.** Fleet tracks it, so two Checks naming `migrate` run it once.

**Consequence for the Commands registry.** A Command earns its place by being something a Check depends on, not only something a person runs by hand.
That is also an argument against ever folding Commands into Checks.

**Per context, not per worktree.** Checks share the worktree today, so the two are the same thing and the skip holds.
A Check running in its own container is a different context: the prerequisite's effect is not there to inherit, so it must run again.

Stating the rule as per-context means per-container Checks need no exception written later — the tracking key is the context, and an isolated Check simply never finds a hit.

**Either way, `requires` guarantees *has run*, not *has just run*.** A Check needing genuinely fresh state resets what it needs in its own command.

In the Set Up a Project (Manifest) journey's proposal panel, a Check row shows its prerequisites in the same cell as its command.

### What a passing run of a Check looks like

**A Check may declare `expect_exit_code`, and absent means `0`.**

```yaml
checks:
  repro:
    run: cargo test --test regression -- --exact known_bug
    expect_exit_code: 1
```

Decided 8 Sep 2026. **The code lives on the Check, not on the step in a
workflow**, and it moved here from there — the same decision `when` records one
heading down, for the same reason. A Check that passes by failing knows that
about itself; a step naming it does not, and the number restated in `bug`,
`feature`, `refactor` and `revert` drifts the day the command changes.

- **Absent means zero**, which is what every Check written before the key
  existed meant.

### Which paths a Check covers

**A Check may declare `when`, a list of path patterns, and a change that touched none of them does not run it.**

```yaml
checks:
  storybook:
    run: pnpm -C packages/components build-storybook
    when: ["packages/**", "apps/desktop/**"]
```

Decided 29 Aug 2026. **The pattern lives on the Check, not on the step in a workflow.** The repository declares once what a Check covers and every workflow inherits it — the same glob repeated across `bug`, `feature`, `refactor` and `revert` would drift, and a Check that unexpectedly did not run would have two places to look.
There is no step-level override, and adding one is a change to this paragraph first.

Rules that follow:

- **Absent means always.** A Check with no `when` runs whenever it is asked for, which is every Check written before the key existed.
  An empty list is refused: `when: []` is a Check that can never run.
- **The dialect is one dialect**, and the Configuration contract states it in full. A pattern from another dialect is refused at load rather than matched literally.
- **The kind of change is not read.** A file deleted from `packages/` is a change to `packages/`, and a rename arrives as two paths — the old one deleted and the new one added — so either side covers the Check on its own.
- **Skipped is not passed, and not `did not run` either.** A run where every Check was skipped verified nothing, and the answer says which of the three happened rather than saying it passed.

`armada check` skips exactly what the merge line would. A run that ran a Check the merge line will not run would tell an agent its work failed something nobody is going to ask.

### How much of the tree a Check reads

**A Check may declare `narrow`, and an agent asking about its own change gets it run that way instead.** The Configuration contract holds the syntax and every key in it.

```yaml
checks:
  test:
    run: cargo nextest run --workspace --exclude acceptance
    narrow:
      run: cargo nextest run
      each: "-p {}"
      under: crates
      except:
        - acceptance
```

Decided 8 Sep 2026, after an agent spent a whole step polling the whole gate and submitted nothing.
It is the second question about paths a Check answers and `when` is the first: `when` decides whether the Check runs, `narrow` decides what it reads once it does.

Rules that follow:

- **Absent means whole.** A Check with nothing narrower to run declares nothing, and runs exactly as it did before the key existed. That is most Checks and always will be.
- **The derivation from a path to an argument is the Manifest's.** `under: crates` is this repository saying where its packages live.
  Nothing in Fleet or the runner knows it, which is what keeps the same key right for a repository that keeps its packages somewhere else.
- **A runner narrows a Check that declares no `narrow`**, from the `run_changed` of its shipped description.
  `{dir}` is the Check's runner `dir`, and a workspace Check that declares none gets its own manifest's directory, `.`.
  Where a root Check declares none and the template names `{dir}`, the Check runs whole.
- **A narrowed run gates nothing.** The report says in its own words that a narrowed pass is the smaller claim.
- **The merge line narrows its own way.** Only through `under`, over every crate the change reaches and every crate depending on one, and whole on any covered path it cannot name — unless the Manifest declares it `outside`, a path `narrow.run` already reads, which adds nothing and runs `narrow.run` alone where nothing else changed (owner, 4 Oct 2026); a verbatim `narrow` never narrows there.
  `git show ecfffb2c1ebd:docs/capabilities/merge-line.md`, *What a narrowed Check runs*.
- **`armada check <name> --changed` reads the change two ways, by who asks.** CI (`ARMADA_SOLE_TENANT=1`) and the merge line (`ARMADA_CHECK_PRIORITY=normal`) get the reading above: every member the paths name.
  Anyone else, which is an agent or a person at the default priority on a machine others share, gets **the tests the change touched** and nothing wider: a Check that declares both `one_test` and a `narrow` with `under` runs its `one_test` with the module paths of the test files among the changed ones (`crates/checks-runner/src/tests/slots.rs` is `tests::slots`), the inline `mod tests` of a changed source file, and the `#[test]` functions of a changed `tests/*.rs`, in the members they are in and no others, each spelled as `narrow.each` spells it.
  No dependents, no `narrow.run` and so no `xtask`.
  **A change that touches no test module runs nothing, says so and passes**, rather than falling back to the whole Check, and a file that only declares `#[cfg(test)] mod tests;` (`lib.rs`) is not a test.
  A Check without `one_test` or `under` has no such reading and is narrowed as above.
  Whole members and their dependents stay with CI and the gate.
  Decided 12 Oct 2026, with the machine at load 30 from three jobs' runs: one new test file had run 1,256 of one crate's 2,888 tests.
- **A Check that narrows to nothing the change touched is skipped, not passed.** The third answer, the same one a `when` skip records.
- **`except` restates an exclusion the whole run already makes.** Dropping `--workspace` so `-p` means something drops `--exclude` with it, and a narrowed `test` that pulled `acceptance` back in would show an agent a bar the merge line deliberately does not apply — the one case where a narrowed run is misleading rather than merely smaller.

### How many places a Check takes

**A Check may declare `places`, a positive integer, and it costs that many of the machine's places while it runs.**

```yaml
checks:
  screens_test:
    run: pnpm -C packages/screens test
    places: 3
```

Decided 14 Sep 2026 for #1102. The machine queue held every Check to one place each, so a browser suite starting a dozen Chromium processes cost what `format` costs, and several sharing the machine's places timed each other out.

Rules that follow:

- **Absent means one**, which is what every Check did before the key existed and what most Checks still cost.
- **Zero, a negative number and anything that is not a whole number are refused at load.**
- **A Check wider than `checks-at-once` still runs**: it takes every place the machine has rather than waiting for room that can never exist, clamped at the moment it asks — the limit can change while Fleet runs.

### How wide a Check runs

**A Check's command may write `${width}`, and Armada substitutes how many concurrent workers it may start.**

```yaml
checks:
  screens_test:
    run: pnpm -C packages/screens test -- --maxWorkers=${width}
    width: 2
```

Decided 18 Sep 2026 for #1444. Every test runner defaults to the width of the machine it finds, and Fleet runs several agents at once by design, so an agent reaching a test claimed the whole machine as many times over as there were agents at one.
On 17 Sep one agent's `desktop_test` started sixteen headless Chromium workers and took the load average to 73, and the owner found out because his machine stopped responding.

**A number handed to a runner, never a cap imposed on one.** Nothing here stops a process spawning what it likes; the repository spells the flag its own runner reads.
That is why the key is a number and not a mechanism, and why a runner that sizes itself from a config file rather than an argument reads `ARMADA_CHECK_WIDTH` from the Check's environment instead — the same second channel a port has.

Rules that follow:

- **Absent means the machine's own number**, which is half its cores divided by the concurrency cap, floored at two and capped at eight.
  Unlike `places`, absent is not a default written down here: nothing that reads the Manifest knows the machine.
- **A declared `width` only ever lowers it.** A repository asking for more of a machine than the machine allows is asking about a machine it does not own, which is the opposite of a timeout, where patience is the repository's to spend.
- **Zero, a negative number and anything that is not a whole number are refused at load**, as `places` is.
- **A command with no `${width}` is left alone**, which is every Check that sizes itself — `cargo fmt --all --check` has no width to be told.
- **Do not widen a timeout to compensate.** A bounded suite that no longer fits its budget is a budget to raise deliberately and say so.
- **A single-tenant machine takes every core.** `ARMADA_SOLE_TENANT=1` makes the number the machine's core count, uncapped, for a CI runner that nothing else shares.
  A declared `width` still lowers it.
  Half the cores is for a person's machine, where jobs share the CPU.
- **A compile is not capped by this number: it takes from the machine's one pool of compiler jobs.** See *One pool of compiler jobs for the machine*, below.
  A Check does not spell `--build-jobs` or `--jobs`, and cargo says so if one does.
- **The terminal resolves it too.** `armada check` substitutes the same way, so a Check a person runs is the Check the merge line measures a branch by.
  It reads the shipped concurrency bound rather than a saved one, and where those differ the merge line is the authority.

### One pool of compiler jobs for the machine

**Fleet holds one pool of compiler slots, sized to the machine's cores, and hands it to every build it starts**: Jobs, Helm, scouts and Checks.
Five builds share the cores and a lone build can use all of them.
Decided by the owner on 16 Oct 2026, and it replaces the per-Check width as the cap on a compile (#1444), which bounded each build and left the sum of them unbounded.

Measured on the owner's 12-core M2 Max (8 performance, 4 efficiency).
Five `cargo` builds ran at once, in four agent slots and the base warm-up, each held to 3 jobs: 15 compiler jobs on 12 cores, at a load average of 28. Earlier, three jobs at no cap were 54 rustc jobs on 18 cores (12 Oct 2026).

It is a GNU make jobserver on a named fifo, `build-slots.fifo` in Fleet's machine directory.
Cargo and rustc take a token before each job and give it back after.
A named fifo, not an inherited pipe, because Fleet starts agents detached and an agent's builds begin long after its spawn.
Fleet puts it in `MAKEFLAGS` for an agent, and for every Check it runs.

What was measured, on a 16-crate workspace with a `RUSTC_WRAPPER` that counts concurrent compiles and sleeps (no load), cargo 1.97.1, `jobserver` 0.1.35:

| Run | Most compiles at once |
|---|---|
| One cargo, `-j 4`, no jobserver | 4 |
| One cargo, fifo of 4 tokens | 5 |
| Two cargos at once, fifo of 4 | 6 |
| One cargo, fifo of 4, and `-j 2` or `-j 8` | 5, and cargo warns it ignored `-j` |
| One cargo, fifo of 4, `CARGO_BUILD_JOBS=2` | 5, and says nothing |
| `kill -9` on a cargo holding all 4 | the fifo then held 0 of 4 |

Rules that follow:

- **A build is one job of its own plus what it takes from the fifo, so the fifo holds four fewer than the cores.** Decided by the owner on 16 Oct 2026, after a fifo of `cores - 1` let five builds peak at 16 on 12 cores, more than the old widths' 15. `k` builds peak at `cores - 4 + k`: five at 13 on this machine, and a lone build gets 9, three times its old width.
  The pool bounds the work Fleet hands out and cannot take each build's own job away, so the headroom is what keeps several builds near the cores.
- **`-j`, `--jobs`, `--build-jobs` and `CARGO_BUILD_JOBS` are not ceilings.** Cargo ignores all four once it inherits a jobserver.
  `CARGO_BUILD_JOBS` is set only where there is no pool: a Fleet whose pool could not be made, `armada check` in a terminal outside a Fleet, and CI, where `${width}` is every core.
- **A killed build loses its tokens for good.** Tokens are anonymous bytes, so Fleet cannot say whose they were.
  Every 15 s it counts them; a pool short of its size while a `cargo`, `rustc` or `rustdoc` is running anywhere on the machine is left alone, and one short with none for two looks in a row has the difference written back.
  A token returned late to a fifo already refilled is dropped.
- **A fifo found at start is kept**, with what builds still running hold of it, because a Fleet that crashed leaves its agents building.
  One that is missing, or not a fifo, is made again, now or at a later look.
- **Nothing outside Fleet takes from it.** A build a person starts in a terminal is outside the pool, as it was outside every width.

### How many Checks run at once

**Every process that runs a Check takes the machine's Check slots first**: `armada check` and the merge line through it.
A Check takes as many as its `places`, and one that finds too few free waits and says so on stderr once — `waiting for a Check slot: 4 of 4 in use`.

Decided 30 Sep 2026. `places` and `${width}` bound what one Fleet starts and nothing else, so several agent jobs each running suites beside a Fleet took the load average to 19 on 18 cores, and a different test timed out on every run.

Rules that follow:

- **There are `checks-at-once`'s machine number of them**, half the cores from one to eight. A saved `checks-at-once` still bounds Fleet's own line; the slots bound the machine.
- **A slot is an `flock` on one of that many files in `~/Library/Application Support/Armada/check-slots/`**, beside `fleet.json`, so every clone and worktree shares them.
  The kernel lets go when the holder dies; there is nothing to reclaim.
- **All or none, and no queue.** A Check wanting three takes three or holds nothing while it waits, so two wide Checks never deadlock on halves; a narrow Check can pass a wide one that waits.
- **The merge line's Checks ask first.** The line sets `ARMADA_CHECK_AHEAD` on every `armada check` it starts, which holds `ahead` in the slot directory while it waits; any other ask that finds it held waits too, and says `waiting for a Check slot: the merge line asked first`.
  It jumps the wait and never a holder: a running Check keeps its slots.
  Decided 2 Oct 2026, because a turn holds every branch queued behind it.
  Priority on the CPU is the next section's.
- **A Check inside a Check runs under its parent's slots.** Each Check's command gets `ARMADA_CHECK_SLOTS_HELD`, so a suite that runs `armada check` on a fixture never waits on itself.
- **A suite run bare takes no slot and no `${width}`.** Run it through `armada check <name>`, and one test through `armada check <name> <test>`.

### Who asked for a run

**Every Check run a surface shows says who asked for it**, as a `Requester` the surface can follow: `kind` is an opaque string and the ids each kind needs are beside it.

| `kind` | Who | Ids |
|---|---|---|
| `merge_line` | The merge line, for one branch | `branch` |
| `session` | A Job's agent running `armada check` in the slot it holds | `session_id`, `slot` |
| `outside` | A person's press or a Verify | none |

- **`outside` is a value and never an absence.** A record from before the field reads as it.
- **A bare `armada check` writes no record**, so there is nothing to name: only what Fleet shows is stamped.
  **One exception, 23.75**: run in a slot a Job holds, it tells Fleet at both ends and the run is kept as that Job's (`docs/concepts/session.md`, *A Job's Checks*).
  The merge line's rows carry the entry's branch, which Fleet reads from the line's own state.

### At what priority a Check runs

**Agent work runs beneath the merge line and the person using the machine.** A Check started by `armada check` or by Fleet is clamped to utility QoS, and everything it starts inherits the clamp.
The merge line's Checks and a run a person starts from Bridge keep normal priority.
The total work is the same; what lands and what the owner is using go first.

Decided 2 Oct 2026, with the machine at load 20 to 32 on 18 cores and most of it agents' own builds.

Rules that follow:

- **`ARMADA_CHECK_PRIORITY=normal` turns it off.** Any other value, or none, lowers. Set it in the environment of `armada check`, or of Fleet for its Checks.
- **A Command run on its own is never lowered.** `armada run <name>` keeps normal priority; the same Command run as a Check's prerequisite takes that Check's.
- **`ps -o pri,ni,pid,command` shows it** as priority 20 where normal work is 31. `ni` stays 0, because it is not `nice`.
- **Utility, not `nice` and not background**, measured on the owner's M5 Pro with 24 busy loops saturating it, a four-worker job at normal priority timed beside each:

| The saturating load ran at | Normal job under load | The same job at that priority, idle machine |
|---|---|---|
| normal | 3.3–5.1s | 1.2s |
| `nice -n 10` | 3.5–5.6s | 1.2s |
| `taskpolicy -c utility` | 1.1–2.0s | 1.1–1.2s |
| `taskpolicy -b` | 1.2–2.0s | 2.1–2.3s |

`nice` left the process at priority 31 and changed nothing.
Background QoS gave way as well as utility but nearly doubled a Check's time on a machine with room, because it holds work to the slower cores.

**An agent runs at the same clamp, so a build it runs itself yields too.** Fleet starts every Helm and scout, and a Job's keeper starts its agent, under `taskpolicy -c utility`, and everything the agent starts inherits it.
`ARMADA_CHECK_PRIORITY=normal` in Fleet's environment turns this off with the Checks' lowering.

- **Fleet's own launchd job is `ProcessType` `Interactive`**, so Fleet and a run the owner starts from Bridge are at normal priority, and only agents and their Checks are lowered.
  Decided 15 Oct 2026. Before, the job set none, and launchd puts a job without one under light resource limits: measured on a 12-core M2 Max that day, Fleet, its `gh` and `git`, every keeper, every agent, a child of an agent's shell and a run from Bridge all read 20, against 31 for processes outside Fleet.
- **Fleet's own polling is lowered too, and an act is not** (16 Oct 2026).
  Interactive put Fleet's own `git` and `gh` at 31 with the owner's runs, and `git status` in a large work repository's slots read 71% of a core each there.
  What Fleet reads on a poll goes through `checks_runner::Priority::Low` at the helpers they share: the pool's reads (`adapters::leasing::git::read`), the forge's reads (`adapters::delivery::read_in`, and the `gh` calls of the issue tracker and CI), the territory, clash and base-standing reads, and `headroom`.
  A merge, a push, a lease, a clone and a run the owner starts from Bridge go through `run_in` and `leasing::git::git` and stay at normal.
  Not lowered, and not a poll: `fleet::process`, `resources` and `orphans`, which read `ps`, `du` and `lsof` for a person's own question.
- **A build a Job runs for the owner yields under load too**, `scripts/preview --restart` among them, because it is an agent's command.
- Load on that machine, one-minute average: 22 to 27 with two Jobs each building the workspace cold at full width, before #2355 and #2356; 7 to 13 after, with no Rust build running.
  The two windows did not run the same work, so the drop is not a measured effect of either.

### Running one test by name

**A Check may declare `one_test`, and `armada check <name> <test>` runs just that test.** The Configuration contract holds the syntax.

```yaml
checks:
  test:
    run: cargo nextest run --workspace --exclude acceptance
    narrow:
      run: cargo nextest run
      each: "-p {}"
      under: crates
      except:
        - acceptance
    one_test:
      run: cargo nextest run -E test(/(^|::){}(::|$)/)
```

Decided 13 Sep 2026 for #999, after several running agents each fixed the same flaky test inside their own change.

Rules that follow:

- **Absent means a test cannot be run by name.** A Check with no `one_test` gives no way to run one test, so a test named under it is refused.
- **`{}` is the test's name, and a command without it is refused at load**, the way every template with nowhere to substitute is.
- **A Check that declares `narrow.under` compiles only the members that declare the test.** `armada check` reads `fn <name>` or `mod <name>` in each member's source and appends every one that does as `narrow.each` spells it, so `armada check test a_test` is `-p fleet` for a fleet test and no longer builds every test binary in the workspace.
  That is why this `run` has no `--workspace`: cargo reads it beside `-p` as the whole workspace.
  `narrow.except` bars `acceptance` here as it does there.
  **A name no member declares is refused, saying so, rather than run against all of them.** The match is over source text, so a member may be compiled for a mention of the name and never missed for the want of one.
  Decided 12 Oct 2026.
- **The name is the caller's.** `{}` takes it regex-escaped and always as one argument, whatever quotes it holds, so it cannot write its way out; a blank name runs nothing.
- **The run gates nothing.**
- **A person runs one the same way**, with `armada check <name> <test>`. A name the runner matched nothing on exits 1 rather than reading as a pass, and a name that matched several says how many ran.
- **Several names are one run**: `armada check test a b c` puts them into `{}` as one alternation and compiles every member that holds any of them, with one Check slot and one row in a Job's thread.
  One name no member declares refuses the whole run.
  Decided 17 Oct 2026, after a Job's agent ran `armada check test <name>` once per name for ten minutes, a build and a queue turn each, at machine load 20.

### Check timeout

A configured bound applies to every Check, and an optional per-Check field narrows it.

Both exist because a Check ranges from a seconds-long lint to a half-hour suite.
One number has to accommodate the slowest and so never fires on the fast ones that hang; a field alone means every repo writes it on every Check or gets no bound at all.

**A timeout stops the Check and its children.** `checks-runner` holds the process group and kills it whole, so a timeout does not orphan them.
The Check did not fail — it did not finish, and retrying re-runs the same hang.
Nor is it one of the did-not-run reasons, since those mean the Check was correctly skipped, and this one ran and got stuck.

Whether the per-Check field may raise the configured bound or only lower it is open (see Open questions).

## Ports

A fourth definition registry, alongside Checks, Commands and Evidence. It defines rather than narrows, which is why it sits at the top level and not under permissions.

**Ports are qualified by Manifest id where a workspace and the root both declare one.** The direction is stated here because the section cannot carry it: the permissions and knowledge sections exist so a setting inherits its direction from where it sits, and Ports is a third direction on a two-direction boundary.

Why: a port is knowledge rather than authority.
Injecting a port number grants no ability the agent lacked, since it could already bind any port, and the allowlist is blast-radius reduction rather than a sandbox.
Commands made the identical move once namespaced by Manifest id.

Each entry has a name and these fields:

| Field | What it is |
| --- | --- |
| the name | What `${port.NAME}` resolves, and what an assignment keys to |
| `container` | The container-side port — the join key Armada matches on |
| `env` | The variable name the repo's own stack already expects |

`container` is what Armada matches on when it rewrites a compose document, so it knows which published port is which.

**`env` exists because a command string covers one channel and there are more.** A `package.json` script Armada invokes but did not author, a compose stack reading a variable, and a shell all read the environment rather than a command line.

`ARMADA_PORT_<name>` is emitted alongside regardless, so a name with no `env` still gets a variable and anything generic has one guaranteed form.
Requiring `env` would be a coupling the repo has to accept in the one channel whose whole purpose is working with a repo untouched.

**Colliding `env` names across a repository's Manifests are rejected at claim time.** Not at config load, and not scoped to a root: two independent roots can live in one repo, and a root-scoped check never compares them.
The same check already exists for Manifest ids that mangle to one environment token.

Exact key naming and nesting is tracked in `../contracts/configuration.md`.

**The main checkout holds a span of its own while Fleet runs**, which servers draw from.
A workspace's own `ports:` are claimed when that workspace is verified and win over the root's for a name both declare; a name neither declares stays as written.

Nothing here asks a person to release a port — there is no such action, and adding one would offer a control for a decision they are already making elsewhere. [Fleet](fleet.md) owns the mechanism.

## Setup (Scan → Proposal → Write)

Carried forward from the v1 prototype, where the pattern worked well.

| Stage | What happens |
| --- | --- |
| Scan | Read-only evidence gathering across the workspace. Nothing is written yet |
| Proposal | A *possible* `armada.yml`, iterated on rather than approved or rejected |
| Write | Commits the current state of the proposal to `armada.yml` |

**Scan** reads lockfiles, package scripts, CI config, docker-compose services, pyproject tool sections and workspace globs.

**A dispatched Scan is agent work Fleet runs on its own**: it carries no worktree and no resolved toolset, the same category as the [Judge](judge.md) and the [Proposer](job-proposer.md).

**Proposal is not a single approve-or-reject draft.** For a clean, well-evidenced workspace, the proposal is often already the final file.
For a gappier one it is a starting point you iterate on — editing it yourself and/or working it through with an agent — before it is something you'd want committed.

**At every stage of iteration, every line stays strictly evidence-backed**: it traces to something an actual file already said. Nothing inferred or guessed.

**Write is not a mode chosen in advance.** There is no separate "Write path" decision made up front, for a single workspace or for a whole monorepo batch.
Write always writes whatever you've arrived at through Proposal/iteration, whether that took zero rounds of iteration or several.

### Monorepo onboarding

For a repo with multiple workspaces, Setup asks orientation questions once, before Scan runs — informational, guiding the flow, not written to `armada.yml`:

- **Which discovered workspaces do you want to set up now?** Onboarding isn't all-or-nothing — scope this batch to whichever subset you want, come back for the rest later.
- **Do any checks run at the root of the repo, outside any single workspace?** Feeds the root `armada.yml` (see Workspace Mapping) if so — both its commands and its Checks.

Proposals for the workspaces selected for this batch are presented together, each line traceable to something Scan actually found.

Iteration on any individual workspace's proposal — and Write for it — happens at whatever pace makes sense.
A workspace with rich evidence might Write immediately; one with thin evidence might sit as a partial proposal until you're actually working in it, same as reaching for Update later.

**Explicitly not asked at Setup:** whether workspaces are coupled or dependent on each other, or how you push code changes (batched vs. per-workspace).
That is a property of the *specific change* being made, not a fixed property of the repo.

## Verify

Run against an existing Manifest. Does both:

- **Drift detection** — re-scans and flags if `armada.yml` is out of sync with the current repo state (new scripts added, old ones removed), without changing the file
- **Live dry-run** — actually runs Setup and Checks once, to confirm they still work

## Update

Manual editing only — no re-scan involved. You edit the Manifest directly, same as any other setting. Setup and Verify both involve scanning; Update does not.

## Deletion

A Workspace can be removed and its `armada.yml` with it. Nearest-ancestor reverts those paths to the root automatically, so the file side needs nothing.

**The record side needs nothing either, because the record was never tied to the file.** A Manifest's row in the store is not deleted when its file is.
It stops resolving — nothing is found at its path — and keeps its `id` and its last known name.

Records that carry it still show what they ran against, and every Evidence row naming it stays readable, which is what append-only was for.
A surface renders the name and marks it as no longer present; nothing renders a bare `id`.

**So there is no deletion event to observe.** Nothing watches for an `armada.yml` disappearing and nothing needs to.
An unresolvable Manifest is a state rather than an occurrence, discovered the moment something tries to resolve it — Verify notices on a re-scan, and nothing else has to.

**Re-creating an `armada.yml` at the same path does not restore the identity.** `id` is explicit so that a Workspace can move without dangling its history, and a new file carries a new `id` unless it is written with the old one.
Carrying it deliberately is how a Workspace is restored rather than replaced.

## Secrets

**Agents never hold secrets directly.** That constraint is Fleet-side and belongs to [Fleet](fleet.md) and the Adapters spec, not to any config tier — an agent never holds a callable secrets tool at all.
Manifest only scopes *which* secrets and providers are available to this specific project; Fleet still brokers the access.

## Skills, Sub Agents, Allowlist

Extends or restricts [Kit](kit.md) defaults — Kit holds the default set, a Manifest declares its own, and nothing above the Manifest constrains it.
Applies to the allowlist, Skills, and **Sub agents** (project-specific definitions alongside Kit's global ones).

**How they actually merge is not decided.** No merge strategy exists in the Configuration Settings registry for Skills, MCP or Sub agents.

All three are currently placed by inferring the inheritance axis from the peer axis, which is not a decision (see Open questions).

Kit's global **Agent file** ("how I work" — personal, cross-project) has **no Manifest-level counterpart**.
A repo's own agent file just lives naturally in the repo itself, alongside this Manifest — Armada doesn't inject or manage it separately.

## Pull request owners

**`policy.pull_request_owners` puts this repository in the central store.** Its claims on a red main and its pull requests' registered owners go there, so a Fleet on another machine signed in to the same account sees them.
`git show ecfffb2c1ebd:docs/capabilities/merge-line.md`, *Fixing a red across machines*; [Account](account.md).

```yaml
policy:
  pull_request_owners: true
```

- **Off unless the file says `true`.** Absent or `false` claims nothing, registers nothing and draws nothing of another machine's. Anything else is refused at the key.
- **The root's to say.** A workspace's file that names it is refused, because the claim is on the repository and a second value would be one nothing reads.
- **Live.** It is read at each take, so turning it off is answered at the next one. `policy` holds this one key; any other is refused.

## What every change here carries

**`standing_rules: <path>` names a file in the repository, and every Judge brief carries it.** A Judge reads it as what this repository requires of every change, labelled as a standard rather than the work and placed after the request.
Without it, a Judge asked whether work stays inside the request reads a repository's own rules as scope expansion: one refused a plan for fixing the prose its change made wrong, in a repository whose gate refuses a change that leaves prose wrong.

| | |
| --- | --- |
| Absent | Every Judge brief is exactly what it was before the key existed |
| The path | One file inside the checkout, refused at the key otherwise — a deliverable's rules |
| Read from | The repository's own checkout, never the Job's worktree, so an agent cannot rewrite what its Judge is told. Read when the work is judged |
| Beside the Judge's own reading | A Judge can also read the same checkout for itself, since 2 Oct 2026, so a repository naming no file is not one its Judge is blind to. The file is still the repository saying it outright: quoted in every brief, where a reading Judge would have to think to look |
| Over the bound | Cut on a whole line at the `standing-rules-cap` setting, and the brief says it was cut |
| Not there | The brief says the file could not be read |

**The repository writes it, and Fleet never guesses.** A file found by convention would put whatever sat at that path in front of every Judge.

## `tracker` and `ci_gate`

Two top-level keys say where a repository keeps what the forge does not decide for it. Both are optional, and a repository that writes neither is read exactly as it was before they existed.

```yaml
ci_gate: build
tracker:
  provider: jira
  site: acme
  project: ABC, OPS
```

| Key | Absent | Meaning |
|---|---|---|
| `tracker` | The forge's own tracker | Which tracker holds the repository's issues. `provider` names the implementation and every other key is carried as written, for that provider to read and refuse. `concepts/tracker.md`, *Choosing a tracker* and *Jira*. For `jira`, `site` is required, and `project` (one key or several, comma separated), `jql` or both name what is listed: a block with neither is refused |
| `ci_gate` | `ci` | The name of the check on a pull request that decides its `ci`. Where a pull request carries a check of that name only that one decides; where it carries none, every check does. `git show ecfffb2c1ebd:docs/capabilities/merge-line.md` |

- **A key that is not text is refused at its own path** (`tracker.site`, `ci_gate`), and the rest of the Manifest still reads. A refused `ci_gate` reads as `ci`.
- **A `tracker` block the provider cannot use is not a refused Manifest.** The repository's issues answer with the reason, and nothing else about it changes.
- **Both are read when Fleet starts.** A save that changes either is reported as frozen, waiting for a restart, as the keys read at that time are.

## Still open

The engineer-facing walk from "add a new repo" to a working Manifest is designed as the Set Up a Project (Manifest) journey — tracked there, not as an open item here.
The full set of dials scoped to a Manifest is a row in the Configuration Settings registry; `../contracts/configuration.md` owns the tiering rule, which is why no list of them appears in prose here.

## Open questions

- **[manifest-root-lockfile-ownership]** Does the root `armada.yml` *own* a shared lockfile it can gate, as opposed to merely being able to gate it?
  The nearest-ancestor rule settles gating and not exclusivity.
- **[manifest-check-timeout-raise-or-lower]** May a per-Check timeout field raise the configured bound, or may it only lower it?
- **[manifest-kit-merge-rules]** What are the Kit→Manifest merge rules for Skills, MCP and Sub agents? No merge strategy exists in the Configuration Settings registry for any of the three.
