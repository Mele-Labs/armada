# Spike: How often do two Jobs compile the same crate at the same state, and what would one schedule save?

**Answer: almost never across slots, so one build per crate state saves about 1% of compile CPU.**
In 9 hours, 912 of 91,303 compile CPU seconds were the same unit built again in another slot's target, and none of those overlapped in time with the first.
What does repeat is inside each slot: 38% of compile CPU rebuilt sources that slot had already compiled.
16% was Cargo compiling a unit again with the same sources and flags in the same target.
15% was the same sources compiled under other flags, mostly a check-only `cargo check` from rust-analyzer or an agent followed by `armada check`'s full build.
A schedule that owns every build could take that 31% off by running one build mode per state and never invalidating a fresh one; a shared store between slots adds the 1%.
The Mac was not short of cores while this ran: a median of 7.6 of 18 cores were busy at a fifteen-minute load of 27, with runnable work held to one cluster by something other than the QoS clamp (*The cores*).

Measured from 19:51 EST on 27 Nov to 04:50 EST on 28 Nov 2026, 9.04 hours, on the owner's 18-core M5 Pro, with the live Jobs of that night.
`shared-builds/sampler.py` read `ps` every 2 seconds and wrote one record per `rustc` and per `cargo` it saw; `shared-builds/analyze_duplicates.py` matched them.
The raw records are in `shared-builds/spike-1/`: `rustc.jsonl.gz`, `cargo.jsonl.gz`, `load.jsonl.gz`, every compile with its verdict in `compiles.csv.gz`, and the analysis output in `analysis.txt`.
Compiles in the worktrees spikes 2 and 3 built under `/tmp/armada-red/` are left out.

## Method

Two compiles are the same unit when the crate, its kind (library, binary, test harness, build script), what it emitted (check-only or a full build), its `-C metadata` hash, opt-level and debuginfo agree.
For a workspace crate the sources must agree too: the sampler hashed every workspace crate's directory in the compile's worktree when it first saw the `rustc`, and the match takes the crate's own hash, every workspace crate it depends on (from `cargo metadata`) and `Cargo.lock`.
A third-party crate's sources are fixed by its version, which its metadata hash carries.
CPU is the `rustc` process's own CPU at its last sighting; wall is its elapsed time.
Who started a compile is read from its parent chain: `armada check` from an agent, Fleet's own builds (preview check, walk builds), bare `cargo` from an agent's shell, rust-analyzer's check on save, the restart and preview scripts, or the sccache server, which hides the chain.

## The night

| | |
|---|---|
| Compiles seen | 3,160, in 37 target directories |
| Compile CPU | 91,303 s, 2.8 cores on average; by hour 0.8 to 3.9 |
| Fifteen-minute load | 15.6 at its lowest, a median of 26.7, 44.4 at its highest; at or over 24 for 79% of the night |
| `rustc` alive at once | a median of 6, at most 24 |

| Who started it | Compiles | CPU s | Share |
|---|---|---|---|
| `armada check` from an agent | 580 | 45,683 | 50% |
| Fleet's own builds | 508 | 23,343 | 26% |
| bare `cargo` from an agent | 423 | 11,306 | 12% |
| rust-analyzer | 567 | 6,267 | 7% |
| restart and preview scripts | 40 | 3,559 | 4% |
| sccache server, unattributed | 1,042 | 1,144 | 1% |

| Crate | Compiles | CPU s | Share |
|---|---|---|---|
| `bridge_native` | 213 | 40,924 | 45% |
| `fleet` | 253 | 30,613 | 34% |
| `ipc` | 256 | 11,026 | 12% |
| every other crate | 2,438 | 8,740 | 10% |

## What repeated

| Kind of repeat | Compiles | CPU s | Share of compile CPU |
|---|---|---|---|
| The same unit in another slot's target | 633 | 912 | 1.0% |
| of which third-party, through the sccache server | 546 | 601 | |
| of which overlapping in time with the first | 0 | 0 | |
| The same sources compiled again, any flags | 636 | 34,826 | 38% |
| the same unit again in the same target | 282 | 15,048 | 16% |
| other flags in the same target | 199 | 13,998 | 15% |
| other flags in another target | 155 | 5,779 | 6% |

Slots almost never hold the same sources.
Jobs change `ipc` and `core-model` near the bottom of the graph, and every crate above them then differs; at 20:10 on 27 Nov six slots held five different `ipc` trees (`git rev-parse HEAD:crates/ipc`).

Other flags in the same target is nearly all one pattern, 173 of its 199 compiles and 13,425 of its 13,998 CPU s: rust-analyzer or an agent's `cargo check` compiles a crate check-only (`--emit=dep-info,metadata`), then `armada check` compiles the same sources as a full build (`dep-info,metadata,link`) under another metadata hash, and Cargo cannot use the first for the second.
`fleet`'s test harness alone is 6,214 CPU s of it.

The same unit again in the same target has more than one cause, and only one was traced:

| Crate, who, target | Compiles | CPU s |
|---|---|---|
| `bridge_native`, Fleet's walk build under the support folder | 4 | 4,048 |
| `fleet`, `armada check`, slot 13 | 2 | 999 |
| `bridge_native`, `armada check`, slot 13 | 3 | 934 |
| `fleet`, Fleet's preview check | 68 | 804 |
| `bridge_native`, Fleet's preview check | 23 | 630 |

`fleet`'s 68 in the preview check are its build script: `crates/fleet/build.rs` rewrites `src/tests/mods.inc` and watches `src/tests`, so the build after any build compiles `fleet` again (spike 3, `shared-builds-warm-base.md`).
The walk build's four were not traced; the match ignores `RUSTFLAGS` and environment, so a change in either would show here as a repeat.

## The cores

The Check gate paused builds at a fifteen-minute load of 24 for most of the night, yet compiles used under 4 cores in every hour.
The load gate Job's samples (`spike-1/core-samples-from-load-gate-job.tsv`, 636 samples on 28 Nov, 04:41 to 06:41 UTC) say why load is no measure of busy cores here.

| | |
|---|---|
| Busy cores | a median of 7.6 of 18, at most 11.0 |
| Load at the same moments | 16 to 35 |
| Samples with one six-core cluster at 90% or more and both others at 40% or less | 46% |

A one-minute `ps` reading at 04:55 EST agrees: 6.7 cores busy, 3.0 of them `rustc`, at a one-minute load of 29.8.
Agent work runs under `taskpolicy -c utility` (`crates/checks-runner/src/priority.rs`), but the load gate Job measured that the clamp is not what holds work to one cluster.
On 28 Nov at about 04:50 UTC it raced six spinners under `taskpolicy -c utility` against six at normal priority, twice: each got 0.12 of a core either way, while `cpu0-5` sat at 8 to 9% and `cpu6-11` at 100%.
Six more utility spinners added no busy cores at all, and twelve added 0.2 and 1.1 cores with about 8 idle.
The cap is on where runnable work is placed, not on its priority, and the busy cluster moved between `cpu0-5` and `cpu6-11` from one sample to the next.
Whether the cap is per process tree or coalition (every agent's work descends from Fleet's launchd job) or machine-wide was not measured: the test, a spinner started outside the tree through `launchctl submit` raced against one inside, was refused by that Job's sandbox.
Memory pressure was at warn throughout, so memory stalls are not ruled out either.
Its tools are copied to `shared-builds/spike-1/`: `percore.c` reads each core's use and `spin2.py` is the spinner.

## Not measured

- Wall time a schedule would save a Job: the sampler sees compiles, not what each Job waited on.
- Memory: the sampler read no resident sizes. The load gate Job saw 23 GB compressed and 6.9 of 8 GB swap at 04:41 to 04:47 UTC, with each Check's `rustc` at 1 to 4 GB.
- Compiles shorter than 2 seconds, linking, and running tests.
- A working day: the window was a night, with the Jobs that ran that night.
- What confines runnable work to one cluster, when the QoS clamp does not.
