# Spike: Would one combined build and test of several Jobs' branches cost less than one each, and how often would it be red?

**Answer so far: the red rate is measured and it is high; the build and test cost is not measured yet, because the owner held every timed build on 28 Nov.**
Of 228 merge-queue attempts that gave a verdict in the last 4 days, 59 were red, so p is 0.26 per attempt.
Under independence a batch of 4 is red 70% of the time and a batch of 8 is red 91% of the time.
The wall, CPU and crates-compiled tables below are empty on purpose: no measured run has finished, and nothing here is a guess in their place.
This file is the interim write-up; it is finished by the same rows in `shared-builds/spike-2/runs.csv` when the runs resume.

Measured on 28 Nov 2026 (UTC) on an 18-core Mac that also carried about 20 live Jobs.
The 15-minute load average read 22 to 44 throughout, and Fleet ran Checks one at a time while it was at or over 24.
Raw logs, the CI query outputs and the scripts are in `docs/spikes/shared-builds/spike-2/`.

## The two sets

Set M is the 8 most recent consecutively merged pull requests that change `crates/`, `xtask/` or `Cargo.*`.
No pull request between them was skipped: every merge from #4104 to #4109 changes one of those paths.
The base B0 is `5a0c99c26f`, the first parent of #4104's merge commit.

| PR | Head | Rust files | Crates touched |
|---|---|---|---|
| 4104 | `3ab09e20b4` | 4 | fleet 2, adapters 2 |
| 4101 | `8c00e141e3` | 8 | bridge-native 4, fleet 2, config 2 |
| 4105 | `6579d33e9b` | 7 | bridge-native 6, xtask 1 |
| 4100 | `0ef8f8d454` | 8 | fleet 7, adapters 1 |
| 4102 | `652fc9961c` | 10 | fleet 4, ipc 3, adapters 2, adapter-traits 1 |
| 4107 | `1feb7337de` | 1 | bridge-native 1 |
| 4106 | `48375e271d` | 58 | fleet 37, ipc 8, api 7, bridge-native 3, store 2, acceptance 1 |
| 4109 | `577e0aaee2` | 13 | fleet 5, api 5, ipc 3 |

Merging a head into B0, as the brief says, was wrong for two of them.
#4107 and #4109 were cut from a main later than B0, so their heads carry #4102, #4100, #4106 and others with them; the "separate" tree of #4109 would have been nearly the whole batch.
Their separate trees are their own non-merge commits cherry-picked onto B0 (`2311602bdd`, `a6998f87dc`); both apply cleanly.
The other six are merged as written.
The combined tree, all 8 merged in queue order, has no conflict and has the same tree hash as `bc6018a03f`, `main` at #4109 (`b09776ce77`).
So Set M's combined run builds exactly what the queue merged.

Set L is the live Job branches ahead of `origin/main` (`2408337a02`, fetched 28 Nov) with commits under `crates/`, `xtask/` or `Cargo.*`.
Seven branches qualified; their tips are pinned in `set-L-branches.txt`.
Uncommitted work in a slot was ignored, as the brief says.

| Slot | Branch | Rust files | Own commits |
|---|---|---|---|
| 7 | operator-thread-acts | 36 | 3 |
| 8 | session-82e19254 | 76 | 9 |
| 10 | clash-agreements | 40 | 6 |
| 11 | session-d95c66b9 | 15 | 1 |
| 12 | session-b11aa821 | 25 | 1 |
| 14 | note-claims | 41 | 3 |
| 22 | walk-from-branch | 41 | 6 |

Each merges into `origin/main` alone.
Together, slot 8 and slot 14 conflict in `crates/store/src/lib.rs`; every other pair merges clean (`set-L-pairwise-conflicts.txt`).
So the batch of all 7 is red before anything compiles, and the batch the combined run uses is the 6 without slot 14.
That is one of the 7 branches costing the whole batch, on a conflict git finds in a second.

## Red rate

From `gh run list --workflow checks --event merge_group --limit 300` (23 Nov 22:07 to 28 Nov 01:07 UTC) and the last 100 merged pull requests.

| | Count |
|---|---|
| Merge-queue runs | 300 |
| Green | 169 |
| Red | 59 |
| Cancelled (a group rebuilt behind a failure) | 70 |
| p, red over red plus green | 59 / 228 = 0.259 |
| p counting a cancelled run as not green | 129 / 298 = 0.433 |
| The 100 merged pull requests that failed the queue at least once | 27 |
| Red `ci` on a merged pull request's final head, on the pull request | 0 of 100 |

| Batch of N | Red if each member is red with p = 0.26, independently |
|---|---|
| 1 | 26% |
| 2 | 45% |
| 4 | 70% |
| 5 | 78% |
| 8 | 91% |

Independence is assumed and is probably wrong in both directions: a queue attempt already stacks up to five entries, so p is a rate for small batches and not for one branch; and a batch of Jobs touching the same crate fails together more than apart.
What the 59 red runs failed on, by the job that was red (`ci-failure-jobs.out`):

| Failed job set | Runs |
|---|---|
| A Rust unit-test shard | 26 |
| `native` | 12 |
| `acceptance` | 7 |
| Only the `ci` roll-up | 5 |
| `foundations` | 4 |
| A shard and another job | 4 |
| Desktop or node | 1 |

Two readings follow.
A red pull request-time `ci` is nearly absent (0 of 100), because a pull request runs only the Checks that compile; every red here is the queue's, which is the run a combined batch would replace.
The 8 pull requests of Set M were queued 2, 5, 2, 5, 2, 1, 2 and 1 times, with a red run for #4100, #4102 and #4106.

## Bisect cost

Computed, not measured, because no combined run has finished.
For a batch of N with one bad branch and no interaction between branches, a halving bisect needs ceil(log2 N) further runs: 2 for N = 4 and 3 for N = 8.
Each further run is a combined build and test of half, then a quarter, then one branch, so the three together cost about as much as testing 7 of the 8 branches again (4 + 2 + 1).
With two bad branches the count roughly doubles.
A conflict like slot 8 and slot 14 needs no run to find, but it still costs a human or a rule a decision about which branch leaves.

## What it took to measure this, and what it found

These are measured facts about the Mac and the repository, found while building the harness.

| Finding | Evidence |
|---|---|
| A Check has a 900 second budget, and the cold build of `cargo build --workspace` ran past it under this load | `M/base-build.log` in the scratch tree: killed at 900 s, 1143 s real including the queue |
| The whole `test` Check does not finish in 900 s either: 5585 of 9137 tests ran by then | `M-basefull-r1-test.log` |
| A killed Check loses its CPU: `/usr/bin/time -l` reads 0.3 s user for a run that compiled for 900 s, because the rustc it started is not waited for | base-test attempts 2 and 3 |
| `crates/ipc/build.rs` writes one absolute `rerun-if-changed` path per operation file, so a target warmed at one worktree path is dirty at any other and `ipc` and everything above it rebuilds | `diag-ipc-fingerprint.log`; a branch that only touched `bridge-native` rebuilt 9 workspace crates in 14 min 27 s (`discarded/`) |
| nextest stops at the first failure, and one test fails on this Mac with no change: `adapters` `loadout_item::an_agent_and_a_style_are_their_markdown_file_by_file_name_or_front_matter` expects `terse.md` and gets `Terse.md` on a case-insensitive disk; a second whole run did not fail it, so it depends on what an earlier run left behind | `base-test-4.log` |
| `armada check test --changed` on a change that only touches `crates/bridge-native` is "narrowed to nothing it runs" in 0.5 s | `discarded/M-p4107-r1-test.log` |
| `armada check` prints nothing until the Check ends, so wall time is read from the process table: when the first cargo appears under the run, and when the run ends | `watch.py` |

Three of these change what the numbers mean when they come.
The 900 second budget caps any run that a Job would also see killed; the harness records a kill and its retry, and the CPU of a killed attempt is a lower bound.
Every run starts from a clone of a warm target restored at one fixed path, so a run pays for its own changes and not for moving the target.
Fail-fast is turned off in the scratch worktrees' `.config/nextest.toml`, which is not a file under `crates/`, so that a red test at the base does not end every run in a minute; the failing test names are kept, so red means a failure the base does not have.

## Method, as it will run

Set M runs in full: the combined tree 3 times, all 8 separate trees once, and #4106 (the largest) and #4107 (the smallest) 3 times each, so variance is known without 24 full runs.
Set L runs once each, with no repetitions, by the coordinator's decision of 28 Nov: 7 separate trees and the batch of the 6 that merge.
Both sets use one fixed worktree and target path (`M/base`, `L/base`) and restore the target from a pristine clone before every run.
That is a change from the brief's one worktree per run, forced by the `ipc` build script above.
Set M's #4107 and #4109 use cherry-picked own commits, as above.
Each Set L run starts only while the gate below is open; a run in progress when it shuts finishes.
Set M, once the gate first opens, runs without being gated again.

## The pause

The owner asked on 28 Nov for every timed build to be held, because the builds added to the load that kept nine Jobs' Checks queued.
The hold started at 04:35 UTC (`pause.txt`); the 15-minute load average was 25.4 and Fleet's check slots had 11 holders.
It resumes when the 15-minute load is under 20 and the check slots have one holder or none.
Builds measured after it ran on a less loaded Mac than those before, and the CSV's `load1` and `load15` columns say which.
Before the hold, the only finished measured run was thrown away (`discarded/`), so no timed row is on either side of it.

## Not measured

| What | Why |
|---|---|
| Combined against separate wall and CPU, either set | Held by the owner; the scripts are written and queued behind the resume gate |
| Repetitions of Set M | The same hold |
| Repetitions for Set L | None, by the coordinator's decision: one run each |
| Bisect cost by running it | No combined run has been red yet; the cost above is computed |
| Crates compiled and tests run per run | The same |
| A batch of all 7 Set L branches | It does not merge: slot 8 and slot 14 conflict, so there is nothing to build |
| The `crates/bridge-native` workspace | It is a workspace of its own that `armada check build` does not build, so changes to it cost the root Checks nothing; 3 of Set M's 8 pull requests touch it |
| Queue wait inside `armada check` | Read off the process table to the second, not from Fleet's own record |
| Whether Fleet's own batching would choose what this spike's trees choose | Not looked at |

## Worktrees left in place

| Path | At |
|---|---|
| `/tmp/armada-red/shared-builds/s2/M/base/src` | `5a0c99c26f`, the working tree every Set M run uses |
| `/tmp/armada-red/shared-builds/s2/M/prep` | the merged trees of Set M, as detached commits |
| `/tmp/armada-red/shared-builds/s2/L/prep` | the merged trees of Set L |
| `/tmp/armada-red/shared-builds/s2/M/comb-r1/src`, `M/p4107-r1/src`, `M/discarded-p4107-src`, `M/dbg/src` | left by the first attempts; their targets are removed |
