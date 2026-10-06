# CI

**Kind:** practice. **Covers:** `.github/workflows/checks.yml`, the workflow that
runs this repository's Checks on GitHub. It reports them and gates nothing: a
ruleset that requires its names does the gating, and none exists yet.

## What runs when

| Trigger | For | Runs |
|---|---|---|
| `pull_request` into `main` | Seeing a branch's Checks before it lands | What `armada covers` names for the diff |
| `merge_group` | The queue's test of a branch merged onto `main` | What `armada covers` names for the group |
| `push` to `main` | Seeing a red `main` | Every Check in CI |
| `workflow_dispatch` | Asking for everything by hand | Every Check in CI |

**A pull request runs a Check when `armada covers` matches what the branch changed or what landed on `main` since it was cut, either side.** It is the rule the merge line uses (`docs/capabilities/merge-line.md`, *Choosing what reruns*). The plan checks out the merge GitHub tests, takes `git merge-base HEAD^1 HEAD^2`, and joins the diff from there to the branch with the diff from there to `main`.

**The event's own base commit is not used.** It is the commit `main` had when the pull request was opened and does not move. Measured.

A merge group's diff runs from its base commit to its head commit.

A pull request run is cancelled by the next push to it. A merge group and a push
to `main` are keyed by their own commit and never cancelled.

## How a Check is chosen

```
changed paths ──> armada covers ──> plan output (JSON array) ──> one job per Check, `if:` its name is in it
                  reads armada.yml                                          │
                  `when:` lists                                             v
                                                              ci  (needs plan + every Check job)
```

**The plan job builds `armada` and pipes the changed paths into `armada covers`.**
It is the same answer the merge line and Fleet's gate ask
(`docs/capabilities/merge-line.md`, *Choosing what reruns*), so a path hits the
same Checks here as there. A change that hits no Check runs none and `ci` passes.

**A plan that cannot be made fails the job.** `armada covers` failing, or the
build of `armada` failing, is red, never an empty plan.

`test` and `build` run `armada check <name> --changed`, so the Manifest's own
`narrow` applies. Every other Check runs whole.

## What `ci`, `needs` and `desktop_test` are

| Name | Passes when | Failed by |
|---|---|---|
| `ci` | The plan succeeded and no Check job failed or was cancelled | A failed Check, a cancelled one, a failed plan |
| `needs` | Fleet's check says so | Not defined here. Reserved: Fleet publishes it |
| `desktop_test` | The plan succeeded and no shard failed or was cancelled | A failed shard, a failed plan |

**A Check the plan skipped counts as passed.** `ci` is the one name a ruleset
requires for the Checks below, and a ruleset will require `ci`, `needs` and later
`desktop_test`.

## Where each Check runs

| Check | Runner | Command | Cache |
|---|---|---|---|
| `build`, `test` | `ubuntu-latest` | `armada check <name> --changed`, whole on `main` and dispatch | rust-cache, cargo-nextest |
| `acceptance` | `ubuntu-latest` | `armada check acceptance` | rust-cache, cargo-nextest |
| `format` | `ubuntu-latest` | `cargo fmt --all --check` | none |
| `typecheck`, `bridge_build` | `ubuntu-latest` | armada.yml's command, direct | node_modules |
| `storybook`, `screens_test`, `components_test` | `ubuntu-latest` | armada.yml's command, direct, `--maxWorkers=$WIDTH` | node_modules, Playwright |
| `hooks_test` | `ubuntu-latest` | `python3 .claude/hooks/test_guard_merge.py` | none |
| `desktop_test` | `macos-latest`, sharded | `vitest run --shard=N/4 --maxWorkers=2` | node_modules, Playwright |

`$WIDTH` is the runner's core count and stands for armada.yml's `${width}`. Every
job prints a `MACHINE` line and writes its Check's wall time to the job summary.
Node Checks run their command directly because the wrapper adds a toolchain and a
build of `armada` they do not need.

**`desktop_test` is outside `ci`** until it stops flaking. Its aggregate job
reports on its own, so a red shard never blocks a change that `ci` passes.

## Adding a Check

1. Declare it in `armada.yml` with a `when:` list.
2. Add a job to `checks.yml` named for the Check, gated with `if: contains(fromJSON(needs.plan.outputs.checks), '<name>')`.
3. Add the name to the allowed list in the plan job's *Which Checks* step.
4. Add the job to `ci`'s `needs`.

## Measured on a trial pull request

Seconds spent in each Check's own step on a 4-core, 16 GB `ubuntu-latest` runner
and a 3-core, 7.5 GB `macos-latest` one, with every cache warm. Each job adds
setup before the step: restoring caches, installing, building `armada`.

| Check | Seconds | Note |
|---|---|---|
| `plan` | 70 to 90 for the whole job | Cold cache: 100. Mostly `cargo build -p armada` |
| `build`, `test` | 0 to 55 | Narrowed to what the diff touched |
| `acceptance` | 24 to 30 | Whole |
| `format` | 2 to 4 | |
| `typecheck` | 45 to 48 | |
| `bridge_build` | 5 to 8 | |
| `storybook` | 32 to 35 | |
| `screens_test` | 49 to 55 | |
| `components_test` | 64 to 108 | |
| `hooks_test` | 1 | |
| `desktop_test` per shard | 68 to 275 | The slowest shard varied the most between runs |

**A whole `test` was not measured to completion.** It stopped at the first
failure, which was a Linux-only test failing on `main`.

## Not in CI

| Check | Why |
|---|---|
| `scripts_test` | Tests `armada land`, which this workflow replaces. Also fails on Linux: the non-macOS `clone_tree` in `cloning.rs` returns `NotCloned` |
| `desktop_test` in `ci` | Flaky on the macOS runner. Reported separately until it is stable |
| Any Check on a self-hosted runner | None exist, and a public repository does not use one |

`hooks_test` runs when `.claude/hooks/**` changes, since that is its `when:`.

## Safety posture

The workflow holds `contents: read` and nothing broader. It uses `pull_request`,
never `pull_request_target`, reads no secret and runs on GitHub's hosted runners.
Every third-party action is pinned to a full commit SHA with its version beside it.

## Owed at the cutover

| Owed | Where |
|---|---|
| Convert the append-only list files; GitHub ignores `merge=union` | `.gitattributes`, `docs/practices/list-files.md` |
| Fleet publishes the `needs` check | Fleet |
| A ruleset requiring `ci` and `needs`, and later `desktop_test` | Repository settings |
| The merge queue setting | Repository settings |
| Change agents' landing instructions | `.claude/hooks/guard_merge.py`, `docs/practices/running-locally.md`, the `work-issue` skill |
| Retire `scripts_test` and `armada land` | `armada.yml`, `crates/armada/src/land/` |
| `.github/**` matches no `when:` in armada.yml, so a workflow change is exercised only by the plan job | `armada.yml` |

## Known limits

| Limit | Status |
|---|---|
| About 5 concurrent macOS jobs on the free plan; the queue can reach minutes | Measured |
| Runner speed on macOS varies between runs | Measured |
| Linux queue is 2 to 5 seconds | Measured |
| An aggregate job started up to 13 minutes after the last job it needed finished, on runs with macOS shards in flight; the cause was not found | Measured |
| Caches hit fully on a repeat of the same lockfile and `.nvmrc` | Measured |
| A cache written on a pull request is read by that pull request and not by another | Inferred from GitHub's cache scoping |
| A merge queue run restores caches from `main` | Inferred from GitHub's cache scoping |
