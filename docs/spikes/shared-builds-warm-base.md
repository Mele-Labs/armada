# Spike: Can a slot reuse one warm build of `main` across worktree paths, and what does it save?

**PARTIAL. E0, E1 and E2 are measured. E3, E4, E5 and E6 were not run, because the owner held every timed build at 23:37 EST on 27 Nov. See "The pause" and "Not measured".**

**Answer: Cargo reuses workspace-crate output across worktree paths, but today two build scripts stop it for the half of the workspace that matters.**
A target directory warmed at path A and used at path B kept every third-party crate and four of the workspace crates fresh (`core-model`, `adapter-traits`, `verification`, `checks-runner`) and rebuilt nine (`ipc`, `store`, `adapters`, `config`, `api`, `fleet`, `pocket`, `armada`, `testkit`).
The cause is named in the fingerprint log: `crates/ipc/build.rs` and `crates/store/build.rs` print `rerun-if-changed` with absolute paths built from `CARGO_MANIFEST_DIR`, so the path list a target keeps names the tree that built it, Cargo marks the script dirty at any other path, and every crate above `ipc` or `store` follows.

Measured on 27 and 28 Nov 2026 on an 18-core, 48 GiB Mac (`hw.memsize` 51539607552) shared with about 20 live Jobs, from `bc6018a03f` (`origin/main`).
Everything below is `cargo build --workspace --locked` unless a row says otherwise, with `RUSTC_WRAPPER=` set empty (no `sccache`; the `-v` logs name no wrapper) and `nice -n 10`.
`runs.csv` and one log per run are in `spikes/shared-builds/spike-3/`.

## Method, and two departures from the brief

- The `sccache` wrapper is not in the repository's `.cargo/config.toml`; it is in `~/.cargo/config.toml`. `RUSTC_WRAPPER=` overrides it.
- **`armada check build` was started and abandoned.** It honours `CARGO_TARGET_DIR` (the target landed where set), but it queued 26 minutes for a Check slot at a 15-minute load of 24 to 25 with 8 of 8 slots held, and the background task was stopped by its 30-minute limit before compiling (`E0-armada-check-queue-aborted.log`). Every run after it is plain `nice -n 10 cargo build`, which the brief allowed.
- **Wall time is not usable.** The load average was 10 to 60 during the runs, and the same 9-crate rebuild took 649 s at load 10 and 2248 s at load 27. User plus system CPU time held within 442 to 566 s. Tables lead with CPU seconds; wall is in `runs.csv` with the load beside it.
- `user_s` and `sys_s` are from `/usr/bin/time -l` on `cargo` and count only waited-for descendants. With `sccache` off, rustc is a child of cargo, so they count it.

## E0: the warm base

| Run | Crates compiled | CPU s (user+sys) | Wall s | Load1 |
|---|---|---|---|---|
| Cold build of `main` into an empty target | 295 (15 workspace) | 824 | 1612 | 19.7 |
| No-op rebuild, first | 2 (`fleet`, `armada`) | 20 | 27 | 18.5 |
| No-op rebuild, second and third | 0 | 0.4 | 0.5 to 0.7 | 15.3 |

The target is 9.9 GB after the cold build.
The first no-op rebuild is not a no-op: `crates/fleet/build.rs` writes `src/tests/mods.inc` and declares `rerun-if-changed=src/tests`, so the file it wrote changes the directory it watches, and the next build reruns it and recompiles `fleet` and `armada`.

## E1: one `CARGO_TARGET_DIR` for two paths

| Run | Crates compiled | CPU s | Wall s | Load1 |
|---|---|---|---|---|
| B (second worktree, same commit) into the target A warmed | 9, all workspace | 492 | 649 | 9.5 |
| Then A into the same target again | the same 9 | 566 | 2248 | 26.8 |

- `-C metadata` and `-C extra-filename` of every workspace crate are the same at A and B (`fleet` is `-3811231cca97f512` in both; one `libfleet-*.rlib` in the target afterwards). Cargo hashes a workspace member's path relative to the workspace root, and runs rustc with `crates/fleet/src/lib.rs` as a relative path.
- The fingerprint says why nine crates rebuild: `ipc` and `store` are `Dirty ...: the rerun-if-changed instructions changed` (`RerunIfChangedOutputPathsChanged`), then `adapters`, `config`, `api`, `fleet`, `pocket`, `armada`, `testkit` are `Dirty ...: info of dependency changed`. In the stored list, `operations/x.toml` and `migrations/x.sql` appear as `/private/tmp/.../A/crates/ipc/operations/x.toml`.
- **Sharing one directory ping-pongs.** B then A rebuilt the same nine again, because the stored fingerprint names whichever tree built last.
- **Mtime.** B's checkout was older than A's outputs, so the four crates above stayed fresh. After `touch` on every B source file, those four rebuilt (`the file ... has changed ... after last build`). A slot created after the base was built has all sources newer than the base, so it rebuilds every workspace crate.
- **Concurrency.** A (one line in `pocket`) and B (one line in `testkit`), each `cargo build -p`, started together on one target: A printed `Blocking waiting for file lock on build directory` and waited for all of B's build (B finished after 2187 s at load 20 to 38; A's own work after that was about 260 s). The lock is the whole invocation, not the crate. Both also rebuilt third-party crates (105 and 76 compiled): `-p` narrows feature unification, so a `-p` build is not warm against a `--workspace` base.

## E2: copy-on-write clone

| | |
|---|---|
| `cp -c -R TA TB` of a 9.9 GB target | 9.3 s, preserves mtimes |
| Disk used, `df` before and after | 0.98 GB (other Jobs wrote at the same time, so an upper bound) |
| B built into TB | 9 crates, 466 CPU s, 1228 s wall at load 19.9 |

The clone behaves as a shared directory in every way E1 measured, minus the lock and the ping-pong: the same nine rebuild, for the same two scripts.

## The pause

The owner ordered every new timed build held at 23:37:04 EST on 27 Nov, because the builds added to the load that kept nine Jobs' Checks queued.
The resume rule was a 15-minute load under 20 and at most one holder of the check-slots directory.
A poll ran for its full two hours and the rule never held.
The lowest 15-minute load sampled was 24.2, at 01:37 on 28 Nov.
The one-minute load swung between 14 and 64, and `lsof` showed 10 to 13 holders.
The pause had not ended when this was written, at 01:47 on 28 Nov.
Every run in this document is from before it, on the loaded Mac described above.

## Not measured

- **E3, `sccache` across paths.** It needs two cold builds in a private cache, then two more with `CARGO_INCREMENTAL=0`, about 800 CPU seconds each. Not run.
- **E4, `--remap-path-prefix`.** Not run. E1 bears on it: workspace members compile with relative source paths and their `-C metadata` holds no worktree path, so a remap is not what stands between two paths and reuse.
- **E5, the saving on a one-crate edit.** Not run: not for `fleet`, not for `config`, not from a warm base, an empty target or a target warmed at `origin/main~30`, and not as a `fleet` test-harness build. The nearest number is E2's: B built into a clone of the base took 466 CPU seconds for nine crates against 824 for a cold build. The nine include `fleet`, so until the two scripts are fixed a `fleet` or `config` edit costs no more at B than no edit at all.
- **E6, the cause removed in a scratch worktree.** Prepared, not run. The patch is `spike-3/E6-scratch-relative-rerun.patch`, applied only in the scratch worktrees `/tmp/armada-red/shared-builds/s3/A` and `B`, never under `slot-6/crates`. It prints `migrations/<name>` and `operations/<name>` in place of absolute paths. The run is: clone the base, build A, clone that, build B, count what rebuilds. Until it runs, "two build scripts stop it" is a reading of the fingerprint log, not a measurement of what removing them saves. `store/build.rs` also hands absolute migration paths to `include_str!`, which the patch leaves.
- **Repetitions.** E0 cold, E1, E2 and the concurrency run are one run each, not three. A cold build took 27 minutes of wall time here.
- **`-Z trim-paths`.** Nightly only; not measured.
- **`bridge-native`.** Left out.
- **`cargo xtask verify-docs --write`.** Not run, because it compiles `xtask`. This file has lines of more than one sentence, which the gate will break, and it is not yet in `docs/INDEX.md`.

## Scratch left in place

- Worktrees `/tmp/armada-red/shared-builds/s3/A` and `/tmp/armada-red/shared-builds/s3/B`, both at `bc6018a03f`, with the E6 patch applied.
- Targets `TA`, `TB` and `TC` beside them, 17 to 20 GB each by `du` though they are mostly clones of one another, and `bin/` with the harness scripts.
