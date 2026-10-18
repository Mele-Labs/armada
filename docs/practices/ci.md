# CI

**Kind:** practice. **Covers:** `.github/workflows/checks.yml` and `.github/ci/`, the
workflow that runs this repository's Checks and `cargo xtask verify-foundations` on
GitHub, and with pull requests it replaces the local merge line. Why, and what was
measured: `.claude/decisions/2026-10-06-ci-and-pull-requests-replace-the-merge-line.md`.
The workflow reports and gates nothing itself: a ruleset on `main` that requires
`ci` does the gating. The repository allows merge commits only and auto-merge is
enabled. The same ruleset holds a merge queue: merge method MERGE, grouping
ALLGREEN, up to five entries built and merged together, and 60 minutes for `ci`
to answer a group. Set 7 Oct 2026; the free plan allows it on a public
repository. The owner, as admin, bypasses the ruleset.

## What runs when

| Trigger | For | Runs |
|---|---|---|
| `pull_request` into `main` | An early warning before the branch joins the queue | Of what `armada covers` names for the diff, the Checks that compile: `build`, `typecheck` and `bridge_build` |
| `merge_group` | The queue's test of a branch merged onto `main` | What `armada covers` names for the group, every one |
| `push` to `main` | Seeing a red `main` | Every Check in CI |
| `workflow_dispatch` | Asking for everything by hand | Every Check in CI |

**The suite runs once, in the merge queue.** A pull request's run keeps the Checks in `QUICK` in `.github/ci/plan.py` and names the rest in its job summary as left to the queue; `ci` passes on those.
The queue's run is the one that decides, so a pull request that also ran the whole suite paid for it twice, back to back: 8 to 20 minutes on the pull request, then 6 to 16 in the queue, measured over the 60 runs before 17 Oct 2026. A branch that breaks a test is now caught in the queue and dropped from it, and Fleet tells its holder (`docs/concepts/fleet.md`, *Telling the owner of a pull request*); the other entries of its group wait one run more.
Decided by the owner, 17 Oct 2026, keeping the Checks that compile on the pull request so most breakage stops before the queue.
`workflow_dispatch` on a branch runs everything, for a change that wants the whole answer first.

**A pull request runs a Check when `armada covers` matches what the branch changed or what landed on `main` since it was cut, either side.** It is the rule the merge line uses (`docs/capabilities/merge-line.md`, *Choosing what reruns*).
The plan checks out the merge GitHub tests, takes `git merge-base HEAD^1 HEAD^2`, and joins the diff from there to the branch with the diff from there to `main`.

**A branch whose own change hits no Check runs none.** Its merge into `main` leaves `main`'s code as it was, and `main` was checked when that code landed, so rerunning what `main` changed proves nothing new.
The `main` side is joined in only when the branch's own change hits at least one Check, which keeps the case the rule exists for: a change on `main` that breaks the branch only in combination.
Decided by the owner, 6 Oct 2026, after a docs-only pull request ran the Rust and desktop Checks because `main` had moved.

**The event's own base commit is not used.** It is the commit `main` had when the pull request was opened and does not move. Measured.

A merge group's diff runs from its base commit to its head commit.

`foundations` runs on every trigger in the table, whatever `armada covers` names.

A pull request run is cancelled by the next push to it. A merge group and a push
to `main` are keyed by their own commit and never cancelled.

## How a Check is chosen

```
changed paths ──> armada covers ──> plan.py ──> checks (JSON array of keys) ──> root name: a static job, `if:` its name is in it
                  reads armada.yml     sorts keys    matrix                  ──> workspace key: one `workspace_check` matrix entry
                  `when:` lists        fails on one                          ──> desktop_test of apps/desktop/unit: one Linux job
                                       it cannot place
                                                                          ci  (needs plan + every Check job + the matrix)
```

**A Check is a key.** A root Check is its bare name (`build`, `test`, `acceptance`, `typecheck`, `hooks_test`, `preview_test`, `sync_mod_test`, `sentences`).
A workspace Check is `<dir>:<name>` (`bridge_build` of `apps/desktop`, `storybook` of `packages/components`), as `armada covers` prints it and `armada check <key>` takes it.

**The plan job builds `armada` and pipes the changed paths into `armada covers`.**
It is the same answer the merge line and Fleet's gate ask
(`docs/capabilities/merge-line.md`, *Choosing what reruns*), so a path hits the
same Checks here as there. A change that hits no Check runs none and `ci` passes.

**A plan that cannot be made fails the job.** `armada covers` failing, or the
build of `armada` failing, is red, never an empty plan.

**`.github/ci/plan.py` places every key, and fails the plan on one it cannot.** A key is run, or named in `EXCLUDED` with its reason, which the plan writes to the job summary.
A Check added to a manifest that is neither makes `plan` red, so it is never skipped by omission.
Its tests run in `foundations` with the others in `.github/ci/`.

**A change under `.github/` hits the Checks that exercise every job shape in the workflow:** every root Check, and `apps/desktop` and `apps/desktop/unit`, which reach the macOS shards, the unit job and the matrix.
`.github/**` is in each root `when` and in those two manifests' `depends_on`.
The other workspaces would repeat the matrix job.

`test` and `build` run `armada check <name> --changed`, so the Manifest's own
`narrow` applies: `run-check` sets `ARMADA_SOLE_TENANT=1`, which keeps the reading
of every crate the paths name, where an agent's terminal gets only the tests the
change touched. Every other Check runs whole. A workspace Check runs as `armada check <key>`, whole, in its directory.

## What `ci` and `desktop_test` are

| Name | Passes when | Failed by |
|---|---|---|
| `ci` | The plan succeeded and no Check job or `foundations` failed or was cancelled | A failed Check, a cancelled one, a failed plan, a new failing foundations line |
| `desktop_test` | The plan succeeded and the unit job did not fail or get cancelled | A failed unit job, a failed plan |

`ci` needs `workspace_check`, the matrix job, whose entries are named by their key.

**A Check the plan skipped counts as passed.** `ci` is the one name the ruleset
requires for the Checks below. `desktop_test` is to be added.

## Where each Check runs

| Check | Runner | Command | Cache |
|---|---|---|---|
| `build` | `ubuntu-latest` | `armada check build --changed`, whole on `main` and dispatch | rust-cache |
| `test` | `ubuntu-latest`, four jobs `test 1/4` to `test 4/4` | `armada check test --changed`, whole on `main` and dispatch, each job a quarter of it: a `cargo` ahead on `PATH` adds `--partition slice:<n>/4 --no-tests=pass` to `nextest run` | rust-cache, cargo-nextest |
| `acceptance` | `ubuntu-latest` | `armada check acceptance` | rust-cache, cargo-nextest |
| `typecheck` (root) | `ubuntu-latest` | `pnpm typecheck` | node_modules |
| Every workspace key except the two desktop Checks below and the excluded: `typecheck`, `bridge_build` of `apps/desktop`, `storybook` and `components_test` of `packages/components`, `screens_test` of `packages/screens`, a surface's `test` | `ubuntu-latest`, one matrix entry per key | `armada check <key>`, with the `armada` the plan built | node_modules, and Playwright where a browser opens |
| `hooks_test` | `ubuntu-latest` | `python3 .claude/hooks/test_guard_merge.py` | none |
| `preview_test` | `ubuntu-latest` | `python3 scripts/test_preview.py` | none |
| `sync_mod_test` | `ubuntu-latest` | `python3 scripts/test_sync_mod.py` | none |
| `sentences` | `ubuntu-latest` | `python3 scripts/test_sentences.py`, then `scripts/sentences --check` over `docs/*.md` but `docs/OPEN.md` | none |
| `xtask_test` of `apps/desktop` | `ubuntu-latest`, job `xtask_test` | `cargo nextest run -p xtask --test-threads $WIDTH`, direct, since the plan's `armada` is not built there. Runs the xtask tests that read `apps/` and `packages/` | rust-cache, cargo-nextest |
| `foundations` | `ubuntu-latest` | `cargo xtask verify-foundations`, on the candidate and on `main`'s tip, read as a delta | rust-cache, `main`'s reading per commit |
| `desktop_test` of `apps/desktop/unit` | `ubuntu-latest`, job `desktop_unit` | `vitest run --maxWorkers=$WIDTH --project 'desktop*'`, direct. `armada check` runs the same from the manifest | node_modules, Playwright |

`$WIDTH` is the runner's core count and stands for armada.yml's `${width}`.
`run-check` also sets `ARMADA_SOLE_TENANT=1`, so `armada check` resolves `${width}` to every core on a runner and not the half a person's machine gets (`docs/concepts/manifest.md`, *How wide a Check runs*).
Measured 7 Oct on the 4-core runner, whole runs of `test`: width 2 median job 451s, tests 282s; width 4 two runs, job 375s and 368s, tests 197s and 193s, 4768 passed, no failure or retry.
Contention at width 4 beyond those two runs is not measured.
Every
job prints a `MACHINE` line and writes its Check's wall time to the job summary.
The root's Node Check and the desktop unit job run their command directly. The matrix
runs `armada check` so a workspace's own `armada.yml` is the command, and it uses
the binary the plan job built, uploaded as an artifact, so no matrix entry builds Rust.

**`desktop_test` is outside `ci`** until it stops flaking. Its aggregate job
reports on its own, so a red unit job never blocks a change that `ci` passes.

**The smoke tests are not a Check.** The owner removed `app_smoke` on 13 Oct 2026, after one kept turning main red and nobody was going to maintain it.
`apps/desktop/vitest.config.ts` still has the `smoke` projects, and `pnpm --dir apps/desktop exec vitest run --project 'smoke*'` runs them by hand, as the walk replays do.
No manifest, gate or CI job runs them.
`apps/desktop/unit/armada.yml` lists what `desktop_test`'s tests read; `docs/practices/bridge.md` says what is in each project.

## Foundations

`cargo xtask verify-foundations` is the gate for what no Check covers: the document rules, and the generated files.
The merge line ran it on every turn, so CI runs it on every change, docs included.
`foundations` is a job of its own, not a Check: it is not in armada.yml and `armada covers` does not choose it.

```
HEAD (the merge GitHub tests) ──> generators ──> verify-foundations ──┐
HEAD^1 (main's tip) ─────────────> verify-foundations, cached per commit ──┴─> only what HEAD has more of is red
```

**Read as a delta, the way the merge line reads it** (`docs/capabilities/merge-line.md`, *How a `verify-foundations` run is read*).
The reading is `.github/ci/foundations_delta.py`, and its tests run in the job first.

| Read | Why |
|---|---|
| Only `FAIL` and `missing:` lines | A warning does not fail `main` either |
| Line numbers normalised out of the subject | A line inserted above an old failure renumbers it |
| Counted, not collected into a set | A second violation of one rule in one file reads like the first |
| A non-zero exit naming no rule is red | A branch that breaks `xtask` prints one compile error |
| A `main` whose run names no rule fails the job | There is nothing to compare against |

**The new lines are written to the job summary.** A line `main` already fails is listed there under its own heading and does not count.

**`main`'s reading is cached per commit**, keyed on that commit alone, since its tree and its `xtask` are both fixed by it.
A pull request restores it from `main`; a push to `main` saves its own run under its own commit, which is what the next pull request restores.
A run that names no rule is never cached.

**A stale generated file fails the job and says how to fix it.** The line ran `cargo xtask verify-docs --write` and `cargo xtask verify-tokens --write` and committed the result.
CI cannot commit onto a pull request, so it runs both, reads the tree with them applied, then fails with the two commands and the files they changed.
The author runs them locally and pushes.

**Two things differ from a checkout on a developer's machine.**

| Difference | Handled by |
|---|---|
| `v1-final` is not in a shallow checkout, so every bare path that means v1 read as a path that exists nowhere | `git fetch --depth=1 origin tag v1-final` |
| The privacy rule bans the name the gate runs as, which on a runner is `runner`, an ordinary word in many files | `USER` unset for the job. The rule's paths convention still runs |

**On a merge group, and on `workflow_dispatch`, main's tip is `HEAD^1`**, the commit before the one tested. In a queue of several, that is the previous entry rather than `main`.

## Adding a Check

1. Declare it in the manifest that owns it, with a `when:` list or none.
2. A workspace Check that is a Node command needs nothing else when its name is in `MATRIX` in `.github/ci/plan.py`; add the name there otherwise, and to `NO_BROWSER` if no browser opens.
3. A root Check needs a job in `checks.yml` gated with `if: contains(fromJSON(needs.plan.outputs.checks), '<name>')`, the name in `ROOT`, and the job in `ci`'s `needs`.
4. A workspace Check that is a Rust command needs a job like `xtask_test`, its key in `RUST` in `plan.py`, and the job in `ci`'s `needs`.
5. A Check CI should not run goes in `EXCLUDED` with its reason.

## Measured on a trial pull request

These were measured before workspace keys, with each command run directly. The
matrix runs the same commands through `armada check <key>` and has not been
measured.

Seconds spent in each Check's own step on a 4-core, 16 GB `ubuntu-latest` runner
and a 3-core, 7.5 GB `macos-latest` one, with every cache warm. Each job adds
setup before the step: restoring caches, installing, building `armada`.

| Check | Seconds | Note |
|---|---|---|
| `plan` | 70 to 90 for the whole job | Cold cache: 100. Mostly `cargo build -p armada` |
| `build`, `test` | 0 to 55 | Narrowed to what the diff touched |
| `acceptance` | 24 to 30 | Whole |
| `typecheck` | 45 to 48 | |
| `bridge_build` | 5 to 8 | |
| `storybook` | 32 to 35 | |
| `screens_test` | 49 to 55 | |
| `components_test` | 64 to 108 | |
| `hooks_test` | 1 | |
| `preview_test` | 10 | Measured locally, not on a runner |
| `foundations`, whole job | 40 cold, 28 to 32 warm | Main's own run adds 8 when its reading is not cached. Setup and checkout are most of the rest |
| `desktop_test` per shard (before the split) | 68 to 275 | The slowest shard varied the most between runs |

**A whole `test` was not measured to completion.** It stopped at the first
failure, which was a Linux-only test failing on `main`.

## Not in CI

| Check | Why |
|---|---|
| `scripts_test` | Dropped. It tested `armada land`, which this workflow replaces, and failed on Linux: the non-macOS `clone_tree` in `cloning.rs` returns `NotCloned` |
| `desktop_test` in `ci` | Flaky on the macOS runner. Reported separately until it is stable |
| Any Check on a self-hosted runner | None exist, and a public repository does not use one |

`hooks_test` runs when `.claude/hooks/**` changes, since that is its `when:`. `preview_test` runs when `scripts/preview`, `scripts/restart`, its own test or `armada.yml` changes.

## Safety posture

The workflow holds `contents: read` and nothing broader. It uses `pull_request`,
never `pull_request_target`, reads no secret and runs on GitHub's hosted runners.
Every third-party action is pinned to a full commit SHA with its version beside it.

## Owed at the cutover

| Owed | Where |
|---|---|
| The ruleset also requiring `desktop_test` | Repository settings |

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
