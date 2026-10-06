# CI

**Kind:** practice. **Covers:** `.github/workflows/checks.yml` and `.github/ci/`, the workflow that
runs this repository's Checks and `cargo xtask verify-foundations` on GitHub. It reports them and gates nothing: a
ruleset that requires its names does the gating, and none exists yet.

## What runs when

| Trigger | For | Runs |
|---|---|---|
| `pull_request` into `main` | Seeing a branch's Checks before it lands | What `armada covers` names for the diff |
| `merge_group` | The queue's test of a branch merged onto `main` | What `armada covers` names for the group |
| `push` to `main` | Seeing a red `main` | Every Check in CI |
| `workflow_dispatch` | Asking for everything by hand | Every Check in CI |

**A pull request runs a Check when `armada covers` matches what the branch changed or what landed on `main` since it was cut, either side.** It is the rule the merge line uses (`docs/capabilities/merge-line.md`, *Choosing what reruns*). The plan checks out the merge GitHub tests, takes `git merge-base HEAD^1 HEAD^2`, and joins the diff from there to the branch with the diff from there to `main`.

**A branch whose own change hits no Check runs none.** Its merge into `main` leaves `main`'s code as it was, and `main` was checked when that code landed, so rerunning what `main` changed proves nothing new. The `main` side is joined in only when the branch's own change hits at least one Check, which keeps the case the rule exists for: a change on `main` that breaks the branch only in combination. Decided by the owner, 6 Oct 2026, after a docs-only pull request ran the Rust and desktop Checks because `main` had moved.

**The event's own base commit is not used.** It is the commit `main` had when the pull request was opened and does not move. Measured.

A merge group's diff runs from its base commit to its head commit.

`foundations` runs on every trigger in the table, whatever `armada covers` names.

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
| `ci` | The plan succeeded and no Check job or `foundations` failed or was cancelled | A failed Check, a cancelled one, a failed plan, a new failing foundations line |
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
| `foundations` | `ubuntu-latest` | `cargo xtask verify-foundations`, on the candidate and on `main`'s tip, read as a delta | rust-cache, `main`'s reading per commit |
| `desktop_test` | `macos-latest`, sharded | `vitest run --shard=N/4 --maxWorkers=2` | node_modules, Playwright |

`$WIDTH` is the runner's core count and stands for armada.yml's `${width}`. Every
job prints a `MACHINE` line and writes its Check's wall time to the job summary.
Node Checks run their command directly because the wrapper adds a toolchain and a
build of `armada` they do not need.

**`desktop_test` is outside `ci`** until it stops flaking. Its aggregate job
reports on its own, so a red shard never blocks a change that `ci` passes.

## Foundations

`cargo xtask verify-foundations` is the gate for what no Check covers: the size limits on a source file and on a comment block, the document rules, and the generated files. The merge line ran it on every turn, so CI runs it on every change, docs included. `foundations` is a job of its own, not a Check: it is not in armada.yml and `armada covers` does not choose it.

```
HEAD (the merge GitHub tests) ──> generators ──> verify-foundations ──┐
HEAD^1 (main's tip) ─────────────> verify-foundations, cached per commit ──┴─> only what HEAD has more of is red
```

**Read as a delta, the way the merge line reads it** (`docs/capabilities/merge-line.md`, *How a `verify-foundations` run is read*). The reading is `.github/ci/foundations_delta.py`, kept in step by hand with `crates/armada/src/land/gate.rs`, and its tests run in the job first.

| Read | Why |
|---|---|
| Only `FAIL` and `missing:` lines | A warning does not fail `main` either |
| Line numbers normalised out of the subject | A line inserted above an old failure renumbers it |
| Counted, not collected into a set | A second violation of one rule in one file reads like the first |
| A non-zero exit naming no rule is red | A branch that breaks `xtask` prints one compile error |
| A `main` whose run names no rule fails the job | There is nothing to compare against |

**The new lines are written to the job summary.** A line `main` already fails is listed there under its own heading and does not count.

**`main`'s reading is cached per commit**, keyed on that commit alone, since its tree and its `xtask` are both fixed by it. A pull request restores it from `main`; a push to `main` saves its own run under its own commit, which is what the next pull request restores. A run that names no rule is never cached.

**A stale generated file fails the job and says how to fix it.** The line ran `cargo xtask verify-docs --write` and `cargo xtask verify-tokens --write` and committed the result. CI cannot commit onto a pull request, so it runs both, reads the tree with them applied, then fails with the two commands and the files they changed. The author runs them locally and pushes.

**Two things differ from a checkout on a developer's machine.**

| Difference | Handled by |
|---|---|
| `v1-final` is not in a shallow checkout, so every bare path that means v1 read as a path that exists nowhere | `git fetch --depth=1 origin tag v1-final` |
| The privacy rule bans the name the gate runs as, which on a runner is `runner`, an ordinary word in many files | `USER` unset for the job. The rule's paths convention still runs |

**On a merge group, and on `workflow_dispatch`, main's tip is `HEAD^1`**, the commit before the one tested. In a queue of several, that is the previous entry rather than `main`.

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
| `foundations`, whole job | 40 cold, 28 to 32 warm | Main's own run adds 8 when its reading is not cached. Setup and checkout are most of the rest |
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

## What `foundations` measured and what it did not

| Claim | Status |
|---|---|
| A change with no new failing line passes `foundations` and `ci` | Measured on the trial pull request |
| A 1250-line source file is red, named in the summary, and turns `ci` red | Measured |
| A stale `packages/tokens/tokens.css` fails the job and names both commands and the file | Measured |
| A failure already on `main` is not red | Measured, while `main` held privacy and path failures under the two defects above |
| A line moving or a second violation reading as new | Self-test only, not on a runner |
| A merge group's reading | Inferred, not run |

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
