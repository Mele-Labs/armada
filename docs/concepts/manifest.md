# Manifest

**What it is:** Per-project config, backed by an `armada.yml`: the Checks a change must pass to land, the Commands available to run against the repo, which Skills, sub agents and secrets it may use, and whether Fleet may dispatch work here at all. Extends or restricts Kit defaults.

---

**Kind:** Entity.

Defines Manifest — the per-project/repo definition of how Fleet and Drones may interact with a given codebase. Companion to the main Armada brief and `../contracts/configuration.md`.

## What it is

A Manifest defines a project's shape and boundaries — checks it must pass to land code, commands available to run against it, which Skills/Agents/secrets it can use, and whether Fleet may dispatch work in it at all. It is backed by an `armada.yml` file.

It is not the same shape as either tier above it. [Kit](kit.md) is the tool set you bring and holds defaults a Manifest may extend or restrict; [Machine](machine.md) is how this installation behaves and has no project-level counterpart at all.

## Workspace mapping

**One Manifest per workspace.** Even inside a large monolith with multiple independent components, each workspace gets its own `armada.yml` and is independently scoped on the [Job Board](job-board.md), same as a standalone repo.

**A Job's `owner_manifest_id` is always exactly one**, which is what lets the Board focus on a single repository's Jobs when one is picked — see [Job Board](job-board.md).

Components typically share one lockfile at the repo root under pnpm/yarn/npm-style workspaces. Only the package manifest (e.g. `package.json`) is truly per-workspace; the shared lockfile is evidence common to all of them, not a per-workspace artifact.

### Root `armada.yml` (monorepos only)

A monorepo may additionally have a root-level `armada.yml`, separate from any workspace's own. It is created only when Setup detects multiple workspaces — a simple, single-workspace repo's Manifest *is* the root, and no separate file exists.

**The root holds commands** — bootstrap/install, for example — **and Checks for the artefacts the root itself owns.** Root Checks exist because the root owns things nothing else gates:

- A change to a root test or CI command touches no workspace, so no workspace's Checks apply and the highest-blast-radius change in the repo has an empty mechanical tier.
- The shared lockfile is written by any workspace's change and read by every workspace's Checks, with no owner.
- A repo-wide Check — *do all workspaces build together* — belongs to no workspace by construction.

**Root Checks run only when the diff touches root paths.** Why: without it the root becomes a place to put a Check that gates every Job in the repo, and one slow Check there taxes every workspace. A root Check is scoped to root-owned artefacts, never promoted to a repo-wide gate.

**Which paths the root owns — nearest ancestor wins.** The nearest `armada.yml` up the directory tree owns a path; where no workspace claims it, the root does. Root paths are therefore exactly the paths no workspace claims.

A `/tooling` directory belonging to no workspace is a root path, gated by root Checks, and ungated only where the root declares none — the same state as a workspace with no Checks.

**Ownership is not exclusivity.** The shared lockfile is owned by the root under this rule, but *owned* means only *gated by root Checks*. Nothing reserves write scope, so two concurrent Jobs can still both write it.

**Per-workspace Checks are unchanged.** Every workspace still declares its own in its own `armada.yml`; root Checks are additional, not a replacement, and a workspace's Checks are never hoisted to the root.

Still open: whether the root *owns* the lockfile as opposed to merely being able to gate it — narrowed rather than closed by the nearest-ancestor rule, which settles gating and not exclusivity (see Open questions).

**The root gives repo-wide build-orchestration evidence a place to be cited from** — a root `Makefile`, `turbo.json`-style task runners — without duplicating it into every workspace's file.

**Armada only records *how to invoke* a tool.** It never replaces or centralizes the tool's own config resolution: ESLint resolving its own root `.eslintrc` up the directory tree is invisible to, and irrelevant for, Armada.

**A workspace declares a dependency on a root command explicitly**, as `setup.requires: [bootstrap]` in its own `armada.yml`. It is a plural list, since a workspace may require more than one root command.

**Fleet tracks whether a required root command has already run for a given worktree**, and re-runs it when **either** the worktree is new, or the evidence backing the root command — the lockfile/manifest Scan traced it from — has drifted since it last ran. That is the same signal Verify's drift detection computes, applied at dispatch time instead of on-demand.

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

- **`depends_on` is a list of repository-relative globs in the `when:` dialect**, which the Configuration contract states once. A pattern from another dialect is refused at load. An empty list is refused, as `when: []` is.
- **It is not transitive.** If `a` depends on `lib` and `lib` depends on `core`, a change to `core` gates `lib` and not `a`. Why: a transitive closure makes the gating set depend on every manifest's list at once, so a line edited in one file changes what an unrelated file gates, and nobody can read a manifest's reach from the manifest. A manifest that is affected by `core` says so itself.
- **`when:` in a workspace file is relative to that workspace's directory.** A pattern like `src/**` in a workspace file means `src/**` under that workspace's directory. `depends_on` is the one key written repository-relative, because it names paths the manifest does not own.
- **A `depends_on` hit runs the Checks that declare no `when`.** A workspace's `when` is read from its own directory, so it cannot match a path the manifest does not own. A Check meant to run when a dependency changes writes no `when`.
- **A root Check's path condition is implicit.** It runs when the diff touches a root-owned path, which is a path no workspace claims, and the root's own `when:` narrows that further. Root `when:` is repository-relative and is intersected with root ownership; there is no key to write the ownership half.
- **A workspace may name a root Command in `setup.requires`.** The name resolves against the workspace's own Commands first and then the root's. Where only the root declares it, it runs in the root, once per worktree, and not in the workspace.
- **`setup.worktrees`, `setup.seed`, `setup.auto_release` and `setup.auto_release_grace_minutes` are refused in a workspace file.** Each is read from the root, so a second value would be one nothing reads.
- **An empty diff gates nothing.** Decided 6 Oct 2026. A Job that changed no files runs no Checks, in every repository, and `config::gating` returns no manifest for it. A Check with no `when` still reads as always on a non-empty diff; the two are kept apart by asking the gating set first and never handing an empty path list to `Covers::reach`.
- **A workflow resolves over a gating set.** `ResolvedWorkflow::resolve_gated` expands `every_manifest_check` once per manifest it is given, in the order given, and each frozen Check carries the directory of the manifest that declared it (empty for the root). A step that names a Check by name still resolves it against the root. `resolve` is the same over the root alone, so a repository with no workspaces freezes what it always did. Fleet resolves every workflow over the root and every workspace it loaded, at start and each time the workflow folders are read again.
- **`Job::gate_manifests` is empty for a repository with no workspaces**, where the root gates every Job and nothing needs saying. Where workspaces exist it lists every manifest that gates, the root included. It is first written at dispatch from `write_targets`: undetermined gates every manifest, determined and empty gates none.

**What the step gate does where a repository has workspaces.**

- **Fleet loads the workspaces when it sets a repository up.** It walks as Scan does, to the same depth and by the same package-file rule, and reads each directory's `armada.yml` as a workspace file. A file that will not load is left out, the repository is served and its other workspaces stand; at start the console names the file and why, and after a root re-read the same is published as a reading of that file, held with the root's. Workspaces are read again whenever the root's `armada.yml` is re-read and adopted; a refused root leaves them as they were. That does not resolve a workflow again, so a workspace added after start gates but declares no Check in an already-resolved workflow until the workflow folders are next read.
- **It reads the changed paths, never the list alone.** `gate_manifests` is empty for a repository with no workspaces, where the root alone gates and nothing is read; where workspaces exist, `config::gating` over the diff says which manifests gate, and an empty diff gates none. A repository with no workspaces gates as it always did, so an empty diff there still runs a Check that declares no `when`.
- **A Check runs in its manifest's directory**, with its prerequisites, its `one_test` and the whole run alone that confirms a red. Its `when` and its narrowing read the paths that manifest owns, relative to it, so a root Check never sees a workspace's path.
- **A Check is keyed by its manifest and name.** `ResolvedCheck::key` is the bare name for the root's and `<dir>:<name>` for a workspace's, and every row, live entry, past duration and reused answer is found by it. A Drone naming a Check for a test broken on main names that key.
- **A manifest the change does not gate skips its Checks.** The workflow resolves once, over every manifest, and the gate skips the Checks whose manifest the diff does not reach. The Checks a Job froze at dispatch are the ones it runs, so a workspace's Check changed in its file waits for a new Job.
- **The gate replaces `gate_manifests` with what each gating manifest came to**, and writes the same to the Job's log. Dispatch wrote a placeholder, since no Check had run. Where a step declares no manifest Check, the list stays as it was.

| A gating manifest's Checks on the step | Outcome |
|---|---|
| none declared | did not run, not declared |
| any did not pass | ran and failed |
| any passed | ran and passed |
| all skipped, none covers what the manifest owns | did not run, path condition unmet |
| all skipped, one covers it | did not run, scope narrowed |

- **A workspace Check's runner `dir` is omitted, and means that workspace.** A Drone's narrowed run of a Check naming `runner: {name: vitest}` puts `.` in the template's `{dir}` and the paths the manifest owns, relative to it, in `{files}`: `pnpm --dir . exec vitest related src/a.ts ...`, run in the workspace directory. A root Check with no `dir` has none to put there and runs whole, as before. `armada check --changed` is not read through a runner at all, only through `narrow`.
- **`armada check` takes the key.** `armada check a:test` runs `a`'s `test` in `a`'s directory, with its prerequisites, its `one_test` and its `--changed` narrowing all read there. The bare name is the root's, and the root's is run from the root. `armada run a:gen` does the same for a Command. A key whose directory is no workspace reads as a bare name, and is refused as one.
- **`armada covers` prints keys.** It reads the changed paths on stdin and answers with the keys of the Checks that cover them: the root's first, then each gating workspace's in directory order. Each manifest is asked about what it owns of the change, relative to it, so a root Check never sees a workspace's path. A repository with no workspace `armada.yml` answers as before, and its empty diff still names a Check with no `when`. Where workspaces exist, an empty diff names nothing, as above.
- **The merge line sets up in the root, then in each gating workspace.** Decided 6 Oct 2026. Once a Check is about to run it runs the root's `setup.requires`, then, for each workspace that has a Check in the set, that workspace's own, in directory order. **A Command a workspace names that only the root declares runs in the root**, once per worktree, and a Command already run is not run again. A workspace's own Command runs in its directory. A workspace's Checks run in its directory whichever Command prepared it. The same setup runs on `main`'s rerun of a Check that failed.
- **A Drone's own run reads the change the way the gate does.** Where the repository has workspaces it skips a manifest the diff, or the files the Drone named, does not gate. The proof run after a merge still runs everything, since it has no change to read. `armada check` still passes no manifests, so it gates nothing. A bare command handed to a Drone's harness is refused for a workspace Check.
- **A workspace Verify reads its file against the root's**, so its `setup.requires` may name a root Command. The Verify still runs that entry in the workspace's directory; running it once in the root directory is the merge line's.

### Seeding a worktree's build

**`setup.seed` names the build directories a new worktree starts from, and the Commands that fill them.** Fleet runs `warm` in the base checkout when the base moves and marks the seed only once every command has succeeded; it clones `paths` into each new worktree, copy-on-write, before `setup.requires` runs. A Job cut while the seed is warming, or on a volume that cannot clone, starts cold and says why — a seed is never copied in full, and never shared. A repository that declares no seed gets none.

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

### Pausing a parked Job for waiting work

**`setup.auto_release` lets Fleet pause a Job parked at a gate when other work is waiting for a slot and the pool is full**, and `setup.auto_release_grace_minutes` is how long a Job must have been still first — [Fleet](fleet.md), *A paused Job gives its slot back*.

```yaml
setup:
  auto_release: true
  auto_release_grace_minutes: 15
```

- **Absent means on, and fifteen minutes.** The owner's choice of 5 Oct 2026: a grace window and an off switch.
- **`auto_release` is `true` or `false`; the window is a positive whole number.** Anything else is refused at load.
- **Read when Fleet starts**, as the pool's size is. A change applies at the next restart.
- **Either is enough on its own**, as `worktrees` is: `setup` with them and nothing else needs no `requires`.

### Cross-Workspace Jobs

**A Job that writes in several Workspaces is still one Job.** It has one worktree on one branch, and every Drone it spawns works that one. One worktree per Workspace would mean either several Drones at once or one Drone straddling branches, and neither can produce a single commit.

**Every declared Workspace descends from a single root `armada.yml`.** One worktree cannot span two repositories and one pull request cannot touch two, so the bound is stated as one root rather than one repository — Armada knows roots and Workspaces. Work that has to cross roots is several Jobs, landing in order — see [Landing](landing.md).

**Where work is sequenceable, it is separate Jobs linked by `dependencies`** — land the API change, then update the consumer against what merged. Each runs fully within its own Workspace's Manifest: own Checks, own approval, own worktree, own pull request. The dependent stays `blocked_by_dependency` until the upstream reaches `completed_success`, then becomes dispatchable but is not auto-dispatched.

**Nothing on the record says which of these a Job is.** A Job carries `owner_manifest_id` (exactly one, and the [Job Board](job-board.md)'s scoping key), `gate_manifest_ids[]` (may be empty) and `write_targets[]`. These replaced a single `manifest_ids` field, which answered four questions at once and whose entry count doubled as a shape discriminator. Nothing reads a shape, because there is none to read.

### A Job gated by several Manifests

**Counting Manifests says nothing about a Job.** A root change gated by twelve Workspaces is an ordinary single-target Job. What follows applies wherever `gate_manifest_ids[]` holds more than one.

**Each gating Workspace's Checks run and gate independently against its own Manifest**, and any one of them failing fails the Job's single `workflow_status`, with the standard gate-failure retry flow applying to the whole Job. Workspaces that cannot land independently do not fail independently either. There is **one shared `retry_count` per step**, not one per Workspace.

**Evidence carries a Manifest reference, and each gating Manifest an outcome** of ran-and-passed, ran-and-failed, or did-not-run-and-why. Without the first there is nowhere to record which Workspace's Checks failed; without the second a root Check correctly skipped because the diff touched no root path is indistinguishable from one silently misconfigured.

**One human gate over the combined diff.** It falls out of whole-Job failure: if any Workspace's Check failure fails the whole Job on one shared `retry_count`, a per-Workspace gate offers a choice that cannot be acted on, since rejecting one Workspace kills the Job anyway. Grouping the diff per Workspace for review is presentation, not gate scope.

**The [Judge](judge.md) reads the combined diff, once.** It follows the human gate rather than the mechanical tier: `acceptance_criteria` are written by the requester about the change, not about a Workspace, so judging per Workspace would evaluate a criterion about consumers against a types package containing none. The asymmetry is deliberate — Checks are per gating Manifest and attributed as such, while the Judge and the human gate are both combined. Cost accepted: a wide combined diff is where `max_context_size` bites first, and exceeding it escalates to a person rather than failing the step.

**Permissions intersect; knowledge unions.** Allowlist, secrets, MCP and Sub agents all resolve most-restrictive-wins — only ops allowed by *every* gating Manifest, only secrets every one grants, only servers and personas every one defines. Commands are namespaced by Manifest `id` and union, because `api:migrate` and `billing:migrate` are two commands rather than one name with two meanings. Skills and the Agent file union, because they are instructions rather than authority. Ports union, qualified by Manifest id — see Ports. The table lives on [Drone](drone.md).

**Policy splits along safety against resource.** Dispatch freeze, `auto_merge` and `review_gate` are most-restrictive-wins: any frozen Manifest freezes the Job, `never` beats `checks-pass` beats `always`, `human_always` beats `auto_if_judge_passes`. The **budget cap follows `owner_manifest_id`**, not the minimum — taking the lowest lets a small Workspace's cap, sized for its own work, kill a much larger Job for a reason unrelated to that Workspace. Over-caution on a safety setting costs a manual step; over-caution on a resource setting makes the work impossible.

How the [Job proposer](job-proposer.md) relates to [Helm](helm.md)'s planning assist is on [Job proposer](job-proposer.md).

## Registries

Separate registries, not the same thing tagged two ways:

| Registry | Purpose | Invoked by |
| --- | --- | --- |
| Checks | Mandatory — must pass to land or advance code | Fleet, as part of mechanical verification |
| Commands | Optional, general-purpose — migrations, doc generation, builds, formatting, servers | A Drone during a Job, and you directly via Bridge |
| Evidence | Declares the harness — how this repo shows what a change did | Fleet, on a step whose evidence type is `visual` |
| Ports | Names a port a workspace needs, so Armada can place it | Nothing invokes it — Fleet reads it at claim time |

Checks gate workflow advancement — see [Workflow](workflow.md).

Commands cover anything project-specific. Armada doesn't reimplement those tools; it gives Fleet, Drones and you one consistent way to invoke them, through the same named registry either way.

### Command approval

A Command can be flagged destructive. That flag gates **Drone** invocation — pauses for your approval, same as any other risky allowlisted op. Your own **manual** invocation via Bridge doesn't require a second approval step, since you're already the one directly triggering it.

**A declared teardown Command is the escape hatch** for a process handed to another supervisor — `launchctl`, a systemd unit, a deliberate double-fork. Everything Armada spawned that stayed in its own tree is killed by process group with no declaration needed, and containers are handled by the Docker adapter, so a declared Command covers only what neither reaches.

### Commands that keep running

A Command with `serve` is a server: it stays up until something stops it. The shape follows Tilt's [`local_resource`](https://docs.tilt.dev/api.html#api.local_resource), which draws the same line between a command expected to exit and one expected to stay up.

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

`${port.NAME}` resolves in every field from the claim the server runs under — see Ports, below. [Fleet](fleet.md) holds each server: one instance per Job, stopped when the Job ends.

> **Rule.** A server that exits on its own has failed, whatever its exit code.
> Why: staying up is the whole of what a server is for.

> **Rule.** A Drone starts a server only through Fleet's MCP tool, never from its shell.
> Why: Fleet has to know a server is running to hand it to the next Drone and to stop it when the Job ends.

### Check prerequisites

**A Check may require Commands to run before it.** `checks.<name>.requires` is an ordered list of **Command names** from this Manifest's own Commands registry.

An end-to-end Check that needs `migrate` and `seed` to have run first has no other way to say so: `setup.requires` (see Root `armada.yml`) is per *worktree* and runs once, so it cannot express per-Check ordering.

A re-run is where a red the contention made is cleared by a person's press. When it passes, the group's tasks that the red run had marked `failed` are marked `done` and the Judge is told so (4 Oct 2026, Job 3; `plan.md`, *failed*). The pass also closes that group's run, and where a group follows it the step is not advanced: the next group starts (5 Oct 2026, #1792, https://github.com/NickMele/armada/issues/1792; `plan.md`).

Rules that follow:

- **Names, never command strings.** A prerequisite points at a Commands entry, so what actually runs is written in exactly one place and stays in step when the Command is edited.
- **Ordered.** The list runs in the order given; two Commands where one seeds what the other migrated are not interchangeable.
- **A failure is attributed to the prerequisite.** A Check whose prerequisite failed reports the prerequisite as the failure, not the Check — otherwise a broken `migrate` reads as a broken test suite.
- **A prerequisite may name a root Command**, the same as `setup.requires`.
- **A prerequisite that has already run in the same execution context is skipped.** Fleet tracks it, so two Checks naming `migrate` run it once.

**Consequence for the Commands registry.** A Command earns its place by being something a Check depends on, not only something a person runs by hand. That is also an argument against ever folding Commands into Checks.

**Per context, not per worktree.** Checks share the worktree today, so the two are the same thing and the skip holds. A Check running in its own container is a different context: the prerequisite's effect is not there to inherit, so it must run again.

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
- **`after_merge` keeps it**, where it drops `when` and `narrow`. Those are a
  step's and a Drone's questions and there is no step after a merge; this one is
  the Check's, so a Check that passes by failing does not report as broken every
  time somebody merges.
- **A step may still write it, and must agree.** The step-level key still
  parses so that workflows written before the move go on loading, and a step
  that writes a *different* number is refused before dispatch. Reading either
  one silently would be Armada picking which file is right about a repository's
  own command.

### Which paths a Check covers

**A Check may declare `when`, a list of path patterns, and a Job that changed none of them does not run it.**

```yaml
checks:
  storybook:
    run: pnpm -C packages/components build-storybook
    when: ["packages/**", "apps/desktop/**"]
```

Decided 29 Aug 2026. **The pattern lives on the Check, not on the step in a workflow.** The repository declares once what a Check covers and every workflow inherits it — the same glob repeated across `bug`, `feature`, `refactor` and `revert` would drift, and a Check that unexpectedly did not run would have two places to look. There is no step-level override, and adding one is a change to this paragraph first.

Rules that follow:

- **Absent means always.** A Check with no `when` runs on every step that names it, which is every Manifest written before the key existed. An empty list is refused: `when: []` is a Check that can never run.
- **The dialect is one dialect**, and the Configuration contract states it in full. A pattern from another dialect is refused at load rather than matched literally.
- **The kind of change is not read.** A file deleted from `packages/` is a change to `packages/`, and a rename arrives as two paths — the old one deleted and the new one added — so either side covers the Check on its own.
- **Skipped is not passed, and not `did not run` either.** A step that advanced because every Check was skipped verified nothing, and the record says so: `check_runs` carries a `skipped` row per Check, naming the paths it covers, and the Drone is told which of the three happened rather than being told it passed.
- **It is frozen with the workflow.** A Job resolves `when` at creation alongside the Check's command, so editing `armada.yml` mid-Job changes the next Job rather than moving the gate under this one.

`armada check`, the asked run a Drone can ask for, skips exactly what the gate would. A rehearsal that ran a Check the gate will not run would tell a Drone its work failed something nobody is going to ask.

### How much of the tree a Check reads

**A Check may declare `narrow`, and a Drone asking about its own change gets it run that way instead.** The Configuration contract holds the syntax and every key in it.

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

Decided 8 Sep 2026, after a Drone spent a whole step polling the whole gate and submitted nothing. It is the second question about paths a Check answers and `when` is the first: `when` decides whether the Check runs, `narrow` decides what it reads once it does.

Rules that follow:

- **Absent means whole.** A Check with nothing narrower to run declares nothing, and runs exactly as it did before the key existed. That is most Checks and always will be.
- **The derivation from a path to an argument is the Manifest's.** `under: crates` is this repository saying where its packages live. Nothing in Fleet or the runner knows it, which is what keeps the same key right for a repository that keeps its packages somewhere else.
- **A runner narrows a Check that declares no `narrow`**, from the `run_changed` of its shipped description. `{dir}` is the Check's runner `dir`, and a workspace Check that declares none gets its own manifest's directory, `.`. Where a root Check declares none and the template names `{dir}`, the Check runs whole.
- **A Drone's narrowed run gates nothing.** The Drone asks for it mid-step, the gate takes its own reading regardless, and the report says in its own words that a narrowed pass is the smaller claim. `after_merge` drops `narrow` for the reason it drops `when`.
- **The step gate narrows the merge line's way.** Only through `under`, over every crate the step's own change reaches and every crate depending on one, and whole on any covered path it cannot name — unless the Manifest declares it `outside`, a path `narrow.run` already reads, which adds nothing and runs `narrow.run` alone where nothing else changed (owner, 4 Oct 2026, after Job 3); a verbatim `narrow` never narrows there. The ruling carries the command it narrowed to, and a red run again alone runs that command. Decided 4 Oct 2026 after Job 3, whose gate ran every Check whole until then. [Merge line](../capabilities/merge-line.md), *What a narrowed Check runs*.
- **A Check that narrows to nothing the change touched is skipped, not passed.** The third answer, the same one a `when` skip records.
- **`except` restates an exclusion the whole run already makes.** Dropping `--workspace` so `-p` means something drops `--exclude` with it, and a narrowed `test` that pulled `acceptance` back in would show a Drone a bar the gate deliberately does not apply — the one case where a narrowed run is misleading rather than merely smaller.
- **It is frozen with the workflow**, beside the Check's command.

### Where a Check runs

**A Check may declare `runs_at: gate` to stay out of a Drone's own run, or `runs_at: handoff` to run once before the work is handed off.** The Configuration contract holds the rules.

```yaml
checks:
  storybook:
    run: pnpm -C packages/components build-storybook
    runs_at: gate
```

Decided 14 Sep 2026 for #849. A slow Check that mirrors CI cost its whole run on every retry and on every ask. Absent is `everywhere`, which is what every Check did before the key existed.

Rules that follow:

- **A Drone's run leaves out a `gate` or `handoff` Check**, and its brief names each one and where it runs instead, so a clean run does not read as the whole bar.
- **A `handoff` Check runs on one step only**, the last one gating on every Check before handoff, and starts only once every other Check there has passed. A failure goes back to that step's Drone under its retry budget.
- **A narrowed run still gates nothing**, and neither does this: the gate settles every Check it holds, whatever a Drone asked for. It reuses a Check only from a whole asked run that passed it on the same attempt, with nothing changed since, and runs the rest fresh.

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
- **It is frozen with the workflow**, beside the Check's command, and `after_merge` keeps it: a proof still spends the machine for real.

### How wide a Check runs

**A Check's command may write `${width}`, and Armada substitutes how many concurrent workers it may start.**

```yaml
checks:
  screens_test:
    run: pnpm -C packages/screens test -- --maxWorkers=${width}
    width: 2
```

Decided 18 Sep 2026 for #1444. Every test runner defaults to the width of the machine it finds, and Fleet runs several Jobs at once by design, so a Job reaching a test step claimed the whole machine as many times over as there were Jobs at one. On 17 Sep one Drone's `desktop_test` started sixteen headless Chromium workers and took the load average to 73, and the owner found out because his machine stopped responding.

**A number handed to a runner, never a cap imposed on one.** Nothing here stops a process spawning what it likes; the repository spells the flag its own runner reads. That is why the key is a number and not a mechanism, and why a runner that sizes itself from a config file rather than an argument reads `ARMADA_CHECK_WIDTH` from the Check's environment instead — the same second channel a port has.

Rules that follow:

- **Absent means the machine's own number**, which is half its cores divided by how many Jobs may run at once, floored at two and capped at eight. Unlike `places`, absent is not a default written down here: nothing that reads the Manifest knows the machine.
- **A declared `width` only ever lowers it.** A repository asking for more of a machine than the machine allows is asking about a machine it does not own, which is the opposite of a timeout, where patience is the repository's to spend.
- **Zero, a negative number and anything that is not a whole number are refused at load**, as `places` is.
- **A command with no `${width}` is left alone**, which is every Check that sizes itself — `cargo fmt --all --check` has no width to be told.
- **Do not widen a timeout to compensate.** A bounded suite that no longer fits its budget is a budget to raise deliberately and say so.
- **It is frozen with the workflow**, beside the Check's command, and `after_merge` keeps it for `places`' reason: the machine is no wider for the merge having happened.
- **A single-tenant machine takes every core.** `ARMADA_SOLE_TENANT=1` makes the number the machine's core count, uncapped, for a CI runner that nothing else shares. A declared `width` still lowers it. Half the cores is for a person's machine, where Jobs and sessions share the CPU.
- **The terminal resolves it too.** `armada check` substitutes the same way, so a Check a person runs is the Check a Drone is measured by. It reads the shipped Jobs bound rather than a saved one, and where those differ the gate is the authority.

### How many Checks run at once

**Every process that runs a Check takes the machine's Check slots first**: `armada check`, the merge line through it, and Fleet's gate. A Check takes as many as its `places`, and one that finds too few free waits and says so on stderr once — `waiting for a Check slot: 4 of 4 in use`.

Decided 30 Sep 2026. `places` and `${width}` bound what one Fleet starts and nothing else, so several agent sessions each running suites beside a Fleet took the load average to 19 on 18 cores, and a different test timed out on every run.

Rules that follow:

- **There are `checks-at-once`'s machine number of them**, half the cores from one to eight. A saved `checks-at-once` still bounds Fleet's own line, which orders a Drone's run ahead of a gate; the slots bound the machine.
- **A slot is an `flock` on one of that many files in `~/Library/Application Support/Armada/check-slots/`**, beside `fleet.json`, so every clone and worktree shares them. The kernel lets go when the holder dies; there is nothing to reclaim.
- **All or none, and no queue.** A Check wanting three takes three or holds nothing while it waits, so two wide Checks never deadlock on halves; a narrow Check can pass a wide one that waits.
- **The merge line's Checks ask first.** The line sets `ARMADA_CHECK_AHEAD` on every `armada check` it starts, which holds `ahead` in the slot directory while it waits; any other ask that finds it held waits too, and says `waiting for a Check slot: the merge line asked first`. It jumps the wait and never a holder: a running Check keeps its slots. Decided 2 Oct 2026, because a turn holds every branch queued behind it. Priority on the CPU is the next section's.
- **A Check inside a Check runs under its parent's slots.** Each Check's command gets `ARMADA_CHECK_SLOTS_HELD`, so a suite that runs `armada check` on a fixture never waits on itself.
- **A suite run bare takes no slot and no `${width}`.** Run it through `armada check <name>`, and one test through `armada check <name> <test>`.
- **A Drone's `run_checks` is time-boxed as a whole**, the wait for a slot included: the per-command budget plus ten minutes of waiting. Past that Fleet stops the run, frees its slots and tells the Drone, naming the Check that was running or saying it was still waiting. The Drone is also told, once, when its run has to wait for a slot. Decided 5 Oct 2026, after a Drone waited 43 minutes on Job 3 with no result and no signal.
- **A run Fleet is still making tells the Drone where it stands every two minutes**: what is running and for how long, what waits for a slot, and what is done. It is a turn of its own and ends when the run does, so the Drone never has to ask.
- **A run's mark lasts as long as the run.** Fleet supervises the task that makes the run, so a task that panics or is cancelled takes the mark off, frees the slots and tells the Drone the run was lost, saying it is a fault in Fleet and not in the work. A Drone is never left refused with "already running" and nothing behind it.
- **A second ask while one is going is refused with where the first stands**: how long it has gone, the Check it is on, and whether it is waiting for a slot. It still says to wait for the report, which arrives as a later turn, and gives the Drone nothing to look at in the meantime.
- **An asked run is a record of its own, never a Check row.** It is written when the run starts and closed when it ends by any route; `running`, `passed`, `failed`, `stopped` (cut off by the time-box or a fault, so measured to no end) and `lost` (its task died, or Fleet restarted while it was `running`). It names who asked, the step, the attempt, the Checks, whether it was narrowed and each Check's log. It reads on `StepDetail.asked_runs` and `list_runs`, and the Record draws a row for each under Checks, signed by the Drone and never hued as a pass. The Job's log says when it started and when it ended, with the Drone, task and attempt on both.
- **A step's gate runs its own Checks one at a time**, fastest first, each still asking for its places. A Drone's own run is not the gate and keeps running several at once. Decided 4 Oct 2026 after Job 3, whose `desktop_test` ran beside its own step's `components_test` and whole Rust `test` and failed on timeouts three times; the merge line has run a turn's Checks one at a time since 2 Oct.

### Who asked for a run

**Every Check run a surface shows says who asked for it**, as a `Requester` the surface can follow: `kind` is an opaque string and the ids each kind needs are beside it.

| `kind` | Who | Ids |
|---|---|---|
| `gate` | A Job's step gate | `job_id`, `step` |
| `drone_task` | A Drone asking on a plan task | `job_id`, `step`, `task_id`, `drone_id` |
| `drone_step` | A Drone asking on a step with no task | `job_id`, `step`, `drone_id` |
| `merge_line` | The merge line, for one branch | `branch` |
| `outside` | A person's press, a Helm session's or a person's own agent's `start_run`, a Verify | none |

- **`outside` is a value and never an absence.** A record from before the field reads as it.
- **An agent's `start_run` names the Drone that asked** where a Drone made the call: `drone_task` or `drone_step`, with the step, task and Drone the caller is at. Fleet places the connection against the Drones it holds, never by anything the call says, so a caller no Drone holds reads `outside`.
- **A bare `armada check` writes no record**, so there is nothing to name: only what Fleet shows is stamped. The merge line's rows carry the entry's branch, which Fleet reads from the line's own state.

### At what priority a Check runs

**Agent work runs beneath the merge line and the person using the machine.** A Check started by `armada check` or by Fleet — a Drone's own run and its gate — is clamped to utility QoS, and everything it starts inherits the clamp. The merge line's Checks and a run a person starts from Bridge keep normal priority. The total work is the same; what lands and what the owner is using go first.

Decided 2 Oct 2026, with the machine at load 20 to 32 on 18 cores and most of it agents' own builds.

Rules that follow:

- **`ARMADA_CHECK_PRIORITY=normal` turns it off.** Any other value, or none, lowers. Set it in the environment of `armada check`, or of Fleet for its Checks.
- **The merge line sets it on every Check it runs**, explicitly rather than by inheritance, so `scripts/land` is never lowered by whoever called it.
- **A Command run on its own is never lowered.** `armada run <name>` keeps normal priority; the same Command run as a Check's prerequisite takes that Check's.
- **`ps -o pri,ni,pid,command` shows it** as priority 20 where normal work is 31. `ni` stays 0, because it is not `nice`.
- **Utility, not `nice` and not background**, measured on the owner's M5 Pro with 24 busy loops saturating it, a four-worker job at normal priority timed beside each:

| The saturating load ran at | Normal job under load | The same job at that priority, idle machine |
|---|---|---|
| normal | 3.3–5.1s | 1.2s |
| `nice -n 10` | 3.5–5.6s | 1.2s |
| `taskpolicy -c utility` | 1.1–2.0s | 1.1–1.2s |
| `taskpolicy -b` | 1.2–2.0s | 2.1–2.3s |

`nice` left the process at priority 31 and changed nothing. Background QoS gave way as well as utility but nearly doubled a Check's time on a machine with room, because it holds work to the slower cores.

### Running one test by name

**A Check may declare `one_test`, and it is what Fleet runs against a checkout of main before a Drone's `draft_fix` drafts anything, and in the Job's worktree before a red at the gate is ruled against the step** — Confirming a red, below. The Configuration contract holds the syntax.

```yaml
checks:
  test:
    run: cargo nextest run --workspace --exclude acceptance
    one_test:
      run: cargo nextest run --workspace --exclude acceptance -E test(/(^|::){}(::|$)/)
```

Decided 13 Sep 2026 for #999, after several running Jobs each fixed the same flaky test inside their own change. A Drone that says a test is broken on main is asking a question: Fleet runs just that test there, drafts the fix only where it fails there too, and claims the test for that fix so a second report drafts nothing.

Rules that follow:

- **Absent means a report cannot be confirmed.** A Check with no `one_test` gives Fleet no way to run one test, so a Drone naming a test under it is refused and nothing is drafted, and a red at the gate is ruled on as it stands.
- **`{}` is the test's name, and a command without it is refused at load**, the way every template with nowhere to substitute is.
- **The name is the Drone's.** `{}` takes it regex-escaped and always as one argument, whatever quotes it holds, so it cannot write its way out; a blank name runs nothing.
- **The run gates nothing.** A test that fails on main drafts a Job that waits for a person, and the Drone's own step is still decided by its Checks.
- **A person runs one the same way**, with `armada check <name> <test>`. A name the runner matched nothing on exits 1 rather than reading as a pass, and a name that matched several says how many ran.
- **It is frozen with the workflow**, beside the Check's command, and `after_merge` drops it for the reason it drops `narrow`.

### Confirming a red

**Before a red Check at the gate is ruled against the step, Fleet asks whether it is the work's or the machine's.** Each failing test runs alone by `one_test`, one at a time; where every one passes, the whole Check runs again alone and the step is ruled on that run. `crates/fleet/src/confirming.rs`.

Decided 3 Oct 2026, from Job 3 (`3-make-branch-inputs-searchable-comboboxes-w`). Its `implement` step failed `desktop_test` on all three attempts with 41 to 45 failing tests each, every one a timeout, a different set each time, in files its diff never touched, and the Job stopped out of retries over work that was fine. The gate then ran a step's Checks up to `checks-at-once` at a time (one at a time since 4 Oct, under *How many Checks run at once*), and that `desktop_test` ran beside the step's own `components_test` and full Rust `test`: 376 s against the merge line's 59 s in the same minutes. Its failing tests passed alone, and plain `main` failed the same way under the same load. Nothing told a red the gate's own contention made from a regression, so each was handed to the Drone as its own.

Rules that follow:

- **Only where every red can be confirmed.** Each must have exited with a code, declare `one_test`, and name its failing tests in the summary shape nextest or vitest prints, read with its colour off. A step with any other red fails whatever the rest would show, so nothing runs and the step is ruled on as it stands.
- **Alone is every place on the machine.** Each run asks for every place, which the clamp in `places` makes the limit in force, so no other Check of Fleet's starts beside it. A Check's prerequisites are not run again: the gate's own run met them in this worktree.
- **A test that fails alone, matches nothing or never runs is the work's**, and the red is handed back exactly as it was, against the retry budget.
- **A few failing tests run one by one, and past `ONE_BY_ONE` in that file none do**: only the whole Check runs alone. Each one-test run of a browser suite still starts the browser and collects every file, so past a handful the tests cost more than the run that decides; a regression usually names a few tests, and the contention named dozens.
- **The whole run alone is what decides.** A test passing alone advances nothing by itself, so a wrong name, a no-match read as a pass, or a test that only fails beside its neighbours is caught by that run. A `runs_at: handoff` Check the red held back runs with it.
- **The whole run alone runs what the gate ruled on**: narrowed where the gate narrowed, over the same reading of the same change. Decided 4 Oct 2026, with the gate's narrowing.
- **Once per ruling**, which is once per attempt. Nothing here loops.
- **The step's Check row reads running while it runs again**, with the live log of the run alone, never the red about to be overturned, and ends at what the run alone came to. A test run alone shows only that the Check is running; where one fails alone the row reads the gate's red again. Decided 3 Oct 2026, after "the checks are running but nothing showed that they were running."
- **It is written down.** The Check's row is the run alone, and the Job's log says under the step that the red was run again alone, how many tests failed and whether the run alone passed. A retro reads that line as Fleet's friction, not the Drone's.

### Proving what merged

**A Manifest may name Checks to run against the tree a merge left behind, and absent means none run.**

```yaml
after_merge:
  checks: [build, test]
```

Decided 7 Sep 2026, #474. A Job's gate proves the work in a worktree cut from a base that has since moved; Fleet then brings the repository up to what merged and, until this key, stopped there — so the last run that said anything about the code holding was a run against a tree that no longer existed.

Rules that follow:

- **Absent is off, and there is no value meaning *all of them*.** What this costs is a build and a test run on the machine somebody is working on, for an event nobody is waiting for. `../practices/rust.md` section 8 names a hook that rebuilt on merge as the cause of v1's four-minute cold build and says outright not to reintroduce one. So the repository names what is worth that, one Check at a time, or nothing runs.
- **The run belongs to the commit, not to the Job that noticed.** Two Jobs merging within a minute are two merges into one commit, and the result is keyed by the commit the repository ended up on. The second Job reads the first one's answer rather than running the suite again.
- **A Check that declares `requires` is refused here.** A prerequisite writes in the tree it runs in, and that tree is the repository a person is standing in. Fleet fast-forwards it only over a clean checkout; a run that then reformatted it would put back what those refusals exist to keep out.
- **A Check's own `when` is not consulted.** `when` answers *did this step touch anything I cover*, and after a merge there is no step to ask it of. The list is the filter.
- **Nothing depends on the answer.** The work is merged. A red cannot fail the Job, cannot reopen it — `completed_success` is terminal — and rolls nothing back; the response is a person filing a new Job pointing back through `subject`. See [Fleet](fleet.md).
- **It holds for both ways a merge lands.** Under `merge_by: forge` the forge names the base it merged into; under `merge_by: push` the push does, without waiting for the forge to catch up. Either way the main checkout is fast-forwarded to what the remote now holds, and the run is against that commit. See *How work lands*.
- **A merge Fleet declined to fast-forward proves nothing.** A dirty worktree or a checkout on another branch leaves no updated tree, and a run against whatever was there would be reporting on somebody's uncommitted work.
- **Its Checks see `${port.NAME}` too.** The proof run has no worktree of its own — it runs in the main checkout — so `${port.NAME}` resolves from that checkout's own span, the one [Fleet](fleet.md)'s Ports section describes as held while Fleet runs.

### Check timeout

A configured bound applies to every Check, and an optional per-Check field narrows it.

Both exist because a Check ranges from a seconds-long lint to a half-hour suite. One number has to accommodate the slowest and so never fires on the fast ones that hang; a field alone means every repo writes it on every Check or gets no bound at all.

**A timeout escalates. It does not fail the gate.** The Check did not fail — it did not finish, and retrying re-runs the same hang.

Nor is it one of the did-not-run reasons, since those mean the Check was correctly skipped, and this one ran and got stuck.

It is a condition the Drone cannot resolve. Killing it is Fleet's: `checks-runner` holds the process group and kills it whole, so a timeout stops the Check and its children rather than orphaning them.

Whether the per-Check field may raise the configured bound or only lower it is open (see Open questions).

## Evidence — how this repository shows its work

On a change whose point is not the code — a panel that should collapse, a screen that should render differently — the diff is the least useful thing on the screen, and it used to be the only thing offered. Reviewing meant reading a patch to infer an outcome you could have been shown.

Checks say whether work advances and Commands say what a Drone may run. Neither says how this project demonstrates that a change did what was asked. `evidence:` is that third thing.

```yaml
evidence:
  serve: pnpm dev --port 6006
  ready: curl -sf http://localhost:6006
  run: pnpm exec playwright test {}
  frames: .playwright/frames
  never:
    - /settings
```

`run` and `frames` are the common denominator — the command that reaches the state and the directory it leaves files in. A web application also starts a server first, which is what `serve` and `ready` are for; a repository with nothing to serve — a desktop app, a CLI, a library, a data pipeline — declares only the two required keys:

```yaml
evidence:
  run: node run-and-capture.js {}
  frames: .armada/frames
```

| Field | What it is |
| --- | --- |
| `serve` | Starts the thing being shown. Long-running: held for the run, ended after it. May reach a `ports:` declaration through `${port.NAME}`. Optional, and goes with `ready` |
| `ready` | Exits zero once `serve` is up. Optional, paired with `serve` — one without the other is refused — and there is no sleep to fall back on |
| `run` | Runs one spec. `{}` is where the spec's path goes, and a template without it is refused. Required |
| `frames` | Where the harness writes, relative to the repo root. Fleet reads it after the run and keeps what it finds. Required |
| `never` | Paths a spec must never visit. Optional, and Armada does not enforce it |

**Command lines and a directory, and not one of them is Armada's.** Nothing in Fleet knows what a browser is or which framework wrote the frames, the same way nothing in the Checks runner knows cargo from pnpm. Reaching a state is what an end-to-end test already does, so the repository's existing harness is the mechanism and Armada gains no capture stack of its own. A repository needing a pipeline writes a script and names the script — there is no shell, so `run` cannot pipe or chain.

`${port.NAME}` resolves in `serve`, `ready` and the substituted `run` from the Job's own claim, exactly as it does for a Command — see Ports, below. `ARMADA_PORT_<NAME>` reaches the same three as an environment variable, which is the only channel the Drone's own spec has to a port it did not read off its own command line.

**`ready` is a command and not a duration, where it is declared.** A number would be a guess against a machine somebody else is using, and what it produces is a frame of a blank page that looks exactly like a frame of a broken one.

**Starting a server is one row's detail, not the shape.** `serve` and `ready` describe a web application; a desktop app's driver launches its own process and owns it directly, a CLI's `run` is the whole invocation, and neither needs a port — so a repository with nothing to serve declares `evidence:` with `run` and `frames` alone. Armada's own `armada.yml` is the other shape: it serves Bridge on a mock Fleet in a browser, on the port its `ports:` section names, and photographs the app replaying recorded Jobs, rather than driving the Electron app itself.

### One run, in the Job's own worktree

Fleet runs `run` once, against the Job's own worktree, and reads `frames` after — there is no comparison against a checkout of the base commit. An earlier design served the base and shot the same spec at it so a reviewer saw before and after, but the pairing only worked where Armada could serve the old code on a port, which ruled out exactly the repositories `serve` and `ready` are now optional for. That code stays on `main` — `crates/fleet/src/basing.rs`, and the base-run method `crates/fleet/src/showing.rs` no longer calls — reachable by nothing, so it returns once something can tell `run` which tree it is aimed at, which is a design of its own.

**A spec captures and never compares.** `run` writes frames into the directory `frames` names and Fleet is what keeps them. A spec that reaches instead for its own runner's snapshot comparison — Playwright's `toHaveScreenshot()` and its kin — keeps baselines of its own and fails on a first run and on any difference, which answers a different question than the one a step whose evidence is what it looks like asks.

**`never` is honest about being a comment.** Nothing here drives a browser, so there is no navigation to intercept. What reads it is the block a Drone writing the spec is given, and the reviewer reading the spec against it. That is weaker than an interception, and the alternative is a key that reads as a guarantee.

**A section this repository does not declare is not a default.** A workflow with a captured step resolved against a Manifest with no harness is refused before anything is dispatched — so it reaches a person as a sentence rather than as a Job that cut a worktree, spawned a Drone and captured nothing.

### A Drone attesting to its own work

Every other evidence type is checkable by something other than the Drone. A screenshot of the wrong state looks exactly like one of the right state.

**The spec is what makes it checkable.** The Drone names a spec and never hands over a file; Fleet runs it and owns the frames. The spec is code, it lands in the diff next to the change, and a wrong frame comes from a wrong spec somebody can read. Weaker than a test, much stronger than an image with no provenance — and it is why a frame carries no caption. A sentence saying what a frame shows would be the attestation put back in a field nothing can check.

### A server to walk the work on

**`walk:` names one server, and a workflow step decides when it starts.** A
step declaring `evidence.walked` — Prototype's Build — stops for a person with
that server already starting in the Job's own worktree, and Bridge opens it in
its own window when the Job is opened. The person looks at the work by using it.

```yaml
walk: mock
```

The name must be a Command with `serve`, refused at load otherwise. **Optional,
and a repository that names none loses nothing**: the step stops exactly as it
did before, and anything it declares can still be started from the run sheet.
A desktop app, a CLI or a library has nothing to walk in a window, and leaves it
out.

## Ports

A fourth definition registry, alongside Checks, Commands and Evidence. It defines rather than narrows, which is why it sits at the top level and not under permissions.

**Ports union across a Job's declaring Manifests, qualified by Manifest id.** The direction is stated here because the section cannot carry it: the permissions and knowledge sections exist so a setting inherits its direction from where it sits, and Ports is a third direction on a two-direction boundary — see A Job gated by several Manifests.

Why: a port is knowledge rather than authority. Injecting a port number grants no ability the Drone lacked, since it could already bind any port, and the allowlist is blast-radius reduction rather than a sandbox. Commands made the identical move once namespaced by Manifest id.

Each entry has a name and these fields:

| Field | What it is |
| --- | --- |
| the name | What `${port.NAME}` resolves, and what an assignment keys to |
| `container` | The container-side port — the join key Armada matches on |
| `env` | The variable name the repo's own stack already expects |

`container` is what Armada matches on when it rewrites a compose document, so it knows which published port is which.

**`env` exists because a command string covers one channel and there are more.** A `package.json` script Armada invokes but did not author, a compose stack reading a variable, and a Pilot shell all read the environment rather than a command line.

`ARMADA_PORT_<name>` is emitted alongside regardless, so a name with no `env` still gets a variable and anything generic has one guaranteed form. Requiring `env` would be a coupling the repo has to accept in the one channel whose whole purpose is working with a repo untouched.

**Colliding `env` names across a Job's Manifest set are rejected at claim time.** Not at config load, and not scoped to a root: two independent roots can live in one repo, and a root-scoped check never compares them. The same check already exists for Manifest ids that mangle to one environment token.

Exact key naming and nesting is tracked in `../contracts/configuration.md`.

**A span outlives an interrupted Job and is released by the Job ending, not by anything about ports.** A claim lasts as long as its worktree; an interrupted Job holds both until a person answers its escalation and it reaches a terminal, after which retention sweeps the worktree and the span goes. The main checkout holds a span of its own while Fleet runs, which the proof run and servers started with no Job draw from. A workspace's own `ports:` are claimed when that workspace is verified and win over the root's for a name both declare; a name neither declares stays as written.

Nothing here asks a person to release a port — there is no such action, and adding one would offer a control for a decision they are already making elsewhere. Past a threshold Fleet surfaces the hold so accumulation is not discovered at exhaustion. [Fleet](fleet.md) owns the mechanism.

## Setup (Scan → Proposal → Write)

Carried forward from the v1 prototype, where the pattern worked well.

| Stage | What happens |
| --- | --- |
| Scan | Read-only evidence gathering across the workspace. Nothing is written yet |
| Proposal | A *possible* `armada.yml`, iterated on rather than approved or rejected |
| Write | Commits the current state of the proposal to `armada.yml` |

**Scan** reads lockfiles, package scripts, CI config, docker-compose services, pyproject tool sections and workspace globs.

**A dispatched Scan is not a [Job](job.md)**, even where it is agent work: it carries no worktree and no resolved toolset, and a Job's `owner_manifest_id` is always present, which a repo with no `armada.yml` cannot supply. It is agent work Fleet runs outside the Job model, the same category as the [Judge](judge.md) and the [Job proposer](job-proposer.md).

**Proposal is not a single approve-or-reject draft.** For a clean, well-evidenced workspace, the proposal is often already the final file. For a gappier one it is a starting point you iterate on — editing it yourself and/or working it through with an agent — before it is something you'd want committed.

**At every stage of iteration, every line stays strictly evidence-backed**: it traces to something an actual file already said. Nothing inferred or guessed.

**Write is not a mode chosen in advance.** There is no separate "Write path" decision made up front, for a single workspace or for a whole monorepo batch. Write always writes whatever you've arrived at through Proposal/iteration, whether that took zero rounds of iteration or several.

### Monorepo onboarding

For a repo with multiple workspaces, Setup asks orientation questions once, before Scan runs — informational, guiding the flow, not written to `armada.yml`:

- **Which discovered workspaces do you want to set up now?** Onboarding isn't all-or-nothing — scope this batch to whichever subset you want, come back for the rest later.
- **Do any checks run at the root of the repo, outside any single workspace?** Feeds the root `armada.yml` (see Workspace Mapping) if so — both its commands and its Checks.

Proposals for the workspaces selected for this batch are presented together, each line traceable to something Scan actually found.

Iteration on any individual workspace's proposal — and Write for it — happens at whatever pace makes sense. A workspace with rich evidence might Write immediately; one with thin evidence might sit as a partial proposal until you're actually working in it, same as reaching for Update later.

**Explicitly not asked at Setup:** whether workspaces are coupled or dependent on each other, or how you push code changes (batched vs. per-workspace). That is a property of the *specific change* being made, not a fixed property of the repo — see Cross-Workspace Jobs.

## Verify

Run against an existing Manifest. Does both:

- **Drift detection** — re-scans and flags if `armada.yml` is out of sync with the current repo state (new scripts added, old ones removed), without changing the file
- **Live dry-run** — actually runs Setup and Checks once, to confirm they still work

## Update

Manual editing only — no re-scan involved. You edit the Manifest directly, same as any other setting. Setup and Verify both involve scanning; Update does not.

## A Manifest a Job creates

A Job can author an `armada.yml` — adding a service, extracting a package. Two questions look like one and are not: *do the new Checks pass*, which is about this work and is answered now, and *who gates this path from here*, which is a durable policy statement.

**Its Checks run before the creating Job lands.** Fleet finds a newly created `armada.yml` in the diff and runs the Checks it declares once, at that Job's final mechanical verification, attributed to the new Manifest.

Nothing in a `WorkflowDef` declares this and no workflow opts in — the trigger is a property of the diff, so it applies to all of them. Without it a Job could add a service together with its test suite and land with that suite never having run.

**It does not gate the creating Job.** `gate_manifest_ids[]` is resolved at dispatch and does not gain a Manifest the Job authored; the new file starts gating from the next Job dispatched. That list says who gates a path, not what ran.

The same freeze holds everywhere else — the `WorkflowDef`, `acceptance_criteria` and Check definitions are all fixed before the work starts, precisely so the yardstick cannot move under it, and a Job adding its own file to its own gate list is that move.

**Nothing is unguarded in the meantime.** Nearest-ancestor owns those paths until the new file exists, so the root's or the parent Workspace's Checks gate the diff that creates it.

**A Job writing in several Workspaces needs no special handling here.** An extraction would otherwise have to add a gate mid-Job, and would need an `id` for a Manifest that does not exist yet. Neither arises: the gate list does not move, and the new `id` matters from the next Job onward.

**The adversarial version closes by construction rather than by approval.** A Drone weakening a Check that gates it is a live concern, and a Drone authoring one that would gate it is the same shape. Here it cannot, because nothing it writes joins the list mid-flight.

What its new Checks get instead is a single run, and a Judge reading those tests as part of the diff, where the gaming patterns already look for a tautological or weakened test.

## Deletion

A Workspace can be removed and its `armada.yml` with it. Nearest-ancestor reverts those paths to the root automatically, so the file side needs nothing.

**The record side needs nothing either, because the record was never tied to the file.** A Manifest's row in the store is not deleted when its file is. It stops resolving — nothing is found at its path — and keeps its `id` and its last known name.

Every historical [Job](job.md) carrying it as `owner_manifest_id` or in its gate list still shows what it ran against, and every Evidence row naming it stays readable, which is what append-only was for. A surface renders the name and marks it as no longer present; nothing renders a bare `id`.

**So there is no deletion event to observe.** Nothing watches for an `armada.yml` disappearing and nothing needs to. An unresolvable Manifest is a state rather than an occurrence, discovered the moment something tries to resolve it — Verify notices on a re-scan, and nothing else has to.

**A not-yet-started Job whose owner no longer resolves is refused at dispatch.** There is no configuration to resolve against, so it cannot run.

It needs no status of its own and no third `queued` reason: dispatch is human-gated, so the refusal reaches a person at the moment they approve it, naming the Manifest that is gone. Rejected: reassigning such a Job to the nearest surviving ancestor, which would make the record say the Job ran against something it did not.

**Re-creating an `armada.yml` at the same path does not restore the identity.** `id` is explicit so that a Workspace can move without dangling its history, and a new file carries a new `id` unless it is written with the old one. Carrying it deliberately is how a Workspace is restored rather than replaced.

## Secrets

**Drones never hold secrets directly.** That constraint is Fleet-side and belongs to [Fleet](fleet.md) and the Adapters spec, not to any config tier — a Drone never holds a callable secrets tool at all. Manifest only scopes *which* secrets and providers are available to this specific project; Fleet still brokers the access.

## Skills, Sub Agents, Allowlist

Extends or restricts [Kit](kit.md) defaults — Kit holds the default set, a Manifest declares its own, and nothing above the Manifest constrains it. Applies to the allowlist, Skills, and **Sub agents** (project-specific definitions alongside Kit's global ones).

**How they actually merge is not decided.** No merge strategy exists in the Configuration Settings registry for Skills, MCP or Sub agents, and Kit and [Drone](drone.md) disagree on Sub agents outright — Kit describes them as layered on top, Drone's resolution table puts them under intersection.

All three are currently placed by inferring the inheritance axis from the peer axis, which is not a decision (see Open questions).

Kit's global **Agent file** ("how I work" — personal, cross-project) has **no Manifest-level counterpart**. A repo's own agent file just lives naturally in the repo itself, alongside this Manifest — Armada doesn't inject or manage it separately.

### What's frozen vs. live

Skills/MCP/Agent-files/Commands are frozen into a Drone at spawn time, a process boot-time constraint. Allowlist and dispatch freeze are enforced live at every gated checkpoint. The budget cap is enforced live too, and since 8 Sept 2026 a Manifest states it — the dollar half then, both halves from 9 Sept — live at every tier, and frozen at none, because a Job past its cap is refused again at every admission until a number moves. See below.

Checks split: a Check that existed at spawn is frozen for the life of the Job, while a Check added mid-Job gates immediately — additive-only, and for a different reason than the boot-time constraint above. Full detail on [Drone](drone.md) and [Fleet](fleet.md).

## Budget — a Manifest states both caps

**A Manifest may state `drone.cost_cap_micros_per_job`**, in millionths of a
dollar, **and `drone.turn_cap_per_job`**, in turns. Each sits between
[Machine](machine.md)'s number and the Job's own. Stating nothing defers upward;
stating zero says no Job here starts anything, which is how a repository is held
without stopping the Fleet the other repositories run under.

**This section said the opposite until 8 Sept 2026**, and the sentence it argued
for was that there is one cap, so no precedence rule and no override. What
falsified it was a Job refused its last step at $5.28 against a $5 compile-time
constant, with no lever anywhere. An earlier Manifest-only row had been deleted
for good reasons that still hold — it declared a *full override* of the Machine
cap with no stated precedence, which is two independent caps rather than a tier.
What is here now is a tier: one order, written once, in `fleet::Allowance::at`.

**The turn cap took the same tier a day later, and for the same reason.** It
stayed Machine's alone on the argument that a repository able to raise its own
would be opting out of the one signal reading steadily across cache warmth. Job
`01M22TYSAE0023MADDP5ZQEYGW` is what falsified that: it finished, passed every
Check, and stopped at 393 turns against a 300 constant with a cheap final step
unrun and its branch committed by hand. A ceiling nobody can move is not a
policy. What keeps a repository from opting out is not the absence of a lever:
it is that a ceiling somebody had to raise is a ceiling somebody read.

**Two things the deleted row was carrying that are still true.** Verification
spend counts against the cap, because a cap that excluded the Judge would
understate what the Job cost; and a cap is a resource setting rather than a
safety one, so nothing takes the minimum across a Job's gating Manifests — the
owning Manifest's number is the one that applies. Peer polarity records that a
lower cap is the stricter one, for whatever consults it later.

## Dispatch freeze

A Manifest-level toggle to pause/freeze **all** dispatch for this project — during a release freeze, for example. It is independent of, and layered on top of, the existing per-Job approval gate.

**Cross-workspace interaction needs no special-case logic.** If a frozen Manifest's Job is blocking a dependent Job in another, unfrozen workspace, that dependent stays `blocked_by_dependency` until the frozen Manifest unfreezes and its Job actually completes — falling out of the existing DAG dependency status_reason, not a new mechanism. A Job whose members are Jobs is dependents all the way down, so the same reasoning reaches it.

**Most-restrictive-wins.** Any frozen Manifest in a Job's gate list freezes the whole Job. Why: a freeze means do not touch this project, and the Job would touch it.

**It is `freeze: true`, a top-level flag, and nothing else.** Absent or `false` is not frozen; any other value is refused at the key. There is no note of why: a key nothing reads is refused on principle. The Manifest form sets it through the same writer as every other key, and the file's own read carries it back.

**It is live, and it holds rather than refuses.** A save is adopted on the re-read Fleet already does, with no restart:

| Where the Job is | What a freeze does |
| --- | --- |
| Approved, never started | Stays `queued`, reading `frozen`, with `frozen_by` naming the Manifest |
| Running | Finishes the step it is on. When that step passes its gate the Job goes back to `queued` with the step advanced and the next not entered, and says so in its log |
| About to deliver | The delivering step is not entered, so nothing is committed, pushed or opened. It is entered, and the branch sent out, when the freeze lifts |
| At a gate under `auto_merge` | The sweep does not merge, and the Job's log says so once. The first sweep after the freeze lifts merges it |
| At a person's gate | The approve, restart or override is taken, and where a step follows the Job waits at `queued` as above. A merge press is taken and written in the Job's log, and nothing merges: the sweep after the freeze lifts carries it out. The press is held in memory, so a restart drops it and it is pressed again |

Lifting it admits on the next turn, onto the step after the one that passed. Nothing is re-run and nothing is lost. Both the queued row and the gate's row carry `frozen_by`; only a queued one reads `frozen`, because that label is `queued`'s. A person's act is never refused for it, for [Fleet](fleet.md)'s reason: admission is the only thing that starts a Drone.

That answers dispatch, and the table above answers a Job already running: freeze is enforced live at every gated checkpoint, not only at dispatch.

## Auto-merge and review gate

Both are per-Manifest, not global, and both use the same override pattern one level apart in the pipeline.

| Setting | Values | Behavior |
| --- | --- | --- |
| `auto_merge` | `never` / `checks-pass` / `always` | Enforced by Fleet before merge |
| `review_gate` | `human_always` (default) / `auto_if_judge_passes` | Whether a workflow's final review step requires a human |

A false `auto_merge` result routes to Inbox > Job Reviews rather than merging.

`review_gate` decides whether the final review step can advance on a Judge pass alone — see [Workflow](workflow.md), the `advance_gate` field.

**Across a Job gated by several Manifests, most-restrictive-wins for both**: `never` beats `checks-pass` beats `always`, and `human_always` beats `auto_if_judge_passes`. There is one PR, so the most cautious gating Manifest holds.

**Both are read at the question, never frozen onto a step.** A step declares `manifest_rule:auto_merge` or `manifest_rule:review_gate` and the record keeps the key rather than the answer — the settings rows call both policies *Live*, so a value written onto a Job at creation would go on stating a decision the repository had since changed. Fleet resolves `review_gate` at the advance gate and `auto_merge` on the sweep over an open pull request. **What both resolved to is kept on every run that reached a gate** (#1683), so the Record can say what a step was gated under after the file has changed. That includes a run its Checks, its Judge or its gaming check stopped, where `decided` is false because the rule never answered for it; the owner decided that on 2 Oct 2026 so the Record says what ran. That is history and is never read back to decide a gate. The run's `resolved` on `get_job` serves it, and a run from before it was kept has none.

**`checks-pass` is the forge's checks, not Armada's.** A Check named in `armada.yml` has already run at the gate the Job is holding at, and totalling the two would claim a gate had held that never ran. Only *every check passed* is a pass: a repository whose forge runs nothing has proved nothing, and a check that finished in a word Armada has no name for counts as not passed.

**`auto_merge` does not read an approval, and `always` means always.** Its three values are all about machines; a person approving on the forge is neither, and whether that becomes a fourth value or a policy of its own is undecided. A forge that requires a review refuses the merge, so branch protection is the backstop and it is the forge's.

### How work lands

**`merge_by` says how Fleet lands a Job's work once a person or `auto_merge` has said it may.** `auto_merge` decides whether; this decides how.

```yaml
merge_by: push
```

| Value | What Fleet does |
| --- | --- |
| `forge` (default) | Asks the forge to merge the pull request, `--merge`. Branch protection and the forge's own checks stay in the path |
| `push` | Makes the `--no-ff` merge commit itself and pushes the base, never forced, with the code `armada land` lands this repository with. The forge reads the pull request merged once its head is in the base |

**A declared need holds Fleet's press under both values** (#1059). Under `forge` it is still Fleet that asks the forge to merge, so Fleet refuses before it does while a need ahead of the Job's stands; a person pressing the forge's own button goes around it. `docs/concepts/fleet.md`, *Declared needs*.

**`forge` is the default, so a repository that says nothing lands exactly as it did before the key existed.** It is the only value that keeps a protected base and the forge's required checks in the path, and a repository that has those has them for a reason this file cannot see. `push` is for a repository where the forge adds a round trip and guards nothing, which is why `armada land` stopped merging through it: [Merge line](../capabilities/merge-line.md), *The merge*.

**Under `push`, only a branch that already holds the base lands, and only at a head whose tree the Job's Checks passed on.** The merge commit carries the branch's own tree. Fleet records the tree a gate's Checks passed on when they were every Manifest Check the workflow declares, and the push refuses a head carrying any other. Fleet's own merges move the head without running a Check: the sweep keeping a pull request current with its base, and a delivery catching its branch up. **Such a head is gated where it stands before the push**, with the Checks below asked of what changed since the tree they passed on and what the branch carries over the base. A red refuses as `fleet.merge_gate_failed` and pushes nothing. Nothing is put back, because Fleet did not make that head in this press and the remote branch already holds it. Decided 1 Oct 2026. **A base that has moved past the branch, or moves before the push, is brought in and gated again**, the way `armada land` answers it: Fleet merges the base the remote holds into the branch, in the Job's own worktree and never by rebasing, runs the Job's Checks over the merge, and pushes again. Decided 1 Oct 2026.

- **The Checks are every Manifest Check the Job's workflow declares, from every step.** The step a Job holds at before merging is a hand-off that declares none in every shipped workflow, so its own list would gate nothing; what the merge changes is the tree the earlier steps' Checks read. Each Check's `when` is asked of what either side changed, as the merge line asks it.
- **A red Check is asked of the base once before it is the branch's.** Green there, it runs once more on the branch and counts only if it fails again. Red there too, it is the base's and is recorded so. [Merge line](../capabilities/merge-line.md), *Fleet's line*.
- **A conflict refuses as `fleet.merge_conflicted` and a red as `fleet.merge_gate_failed`, and either puts the branch back.** A branch left holding the base would be landed unread by the next press, because it would then hold the base.
- **The base moving again goes round, up to the five rounds `armada land` allows**, and then refuses as `fleet.merge_base_moved` naming them.
- **While it runs, every other act at the gate is refused**, because each would change the worktree the Checks are reading. The Job stays at `awaiting_review` throughout, with each round a line in its log and the Checks shown running on the step it holds at.

**Live, and not folded.** It is read at the merge, so a saved change answers the next one. A Job lands in one repository, so that repository's word is the answer and there is nothing for several gating Manifests to resolve.

### A judgeless step under `auto_if_judge_passes`

A step may declare `manifest_rule:review_gate` and no Judge criterion — Code Review's `deliver` step does, and it is legal. **Where the policy then resolves to `auto_if_judge_passes`, the step holds for a person anyway.**

The gate names a tier the step never declared, so advancing would advance on the mechanical tier alone while reading as judged. That is what a workflow file spelling `auto_if_judge_passes` outright is refused for at parse time; it cannot be refused there through a policy, because the file does not say what the policy is. So it is refused where the resolution happens, and refusing means holding rather than failing: the step passed every tier it declared, and nothing a Drone can do from a worktree would fix a configuration mismatch. Fleet writes a line into the Job's log naming the file to fix.

## What every change here carries

**`standing_rules: <path>` names a file in the repository, and every Judge brief carries it.** A Judge reads it as what this repository requires of every change, labelled as a standard rather than the work and placed after the request. Without it, a Judge asked whether work stays inside the request reads a repository's own rules as scope expansion: one refused a plan for fixing the prose its change made wrong, in a repository whose gate refuses a change that leaves prose wrong.

**A Drone is not handed it**, because a Drone reads the repository itself. Its opening brief carried the file from 1 Oct 2026 and stopped on 2 Oct, when the block was found only to repeat what the Drone reads anyway (owner, 2 Oct 2026).

| | |
| --- | --- |
| Absent | Every Judge brief is exactly what it was before the key existed |
| The path | One file inside the checkout, refused at the key otherwise — a deliverable's rules |
| Read from | The repository's own checkout, never the Job's worktree, so a Drone cannot rewrite what its Judge is told. Read when the work is judged |
| Beside the Judge's own reading | A Judge can also read the same checkout for itself, since 2 Oct 2026, so a repository naming no file is not one its Judge is blind to. The file is still the repository saying it outright: quoted in every brief, where a reading Judge would have to think to look |
| Over the bound | Cut on a whole line at the `standing-rules-cap` setting, and the brief says it was cut |
| Not there | The brief says the file could not be read |

**The repository writes it, and Fleet never guesses.** A file found by convention would put whatever sat at that path in front of every Judge.

## Still open

The engineer-facing walk from "add a new repo" to a working Manifest is designed as the Set Up a Project (Manifest) journey — tracked there, not as an open item here. The full set of dials scoped to a Manifest is a row in the Configuration Settings registry; `../contracts/configuration.md` owns the tiering rule, which is why no list of them appears in prose here.

## Open questions

- **[manifest-root-lockfile-ownership]** Does the root `armada.yml` *own* a shared lockfile it can gate, as opposed to merely being able to gate it? The nearest-ancestor rule settles gating and not exclusivity.
- **[manifest-check-timeout-raise-or-lower]** May a per-Check timeout field raise the configured bound, or may it only lower it?
- **[manifest-kit-merge-rules]** What are the Kit→Manifest merge rules for Skills, MCP and Sub agents? No merge strategy exists in the Configuration Settings registry for any of the three, and Kit and Drone disagree on Sub agents outright — Kit describes them as layered on top, Drone's resolution table puts them under intersection.
