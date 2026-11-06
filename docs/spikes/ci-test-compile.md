# Spike: Where does a CI test shard's compile go, and would sccache or a nextest archive shorten it?

**Answer: neither pays on landing time, so neither was taken.**
The compile of the whole `test` Check is 278 seconds of wall time, and 212 of them are one unit: `fleet`'s own test harness, which starts at 66 seconds and ends the build.
sccache caches only rlib and staticlib compiles, so at best it takes off the libraries before that unit; a nextest archive saves runner time but puts a second job, and a second wait for a runner, on the queue's path.
What it did find: a pull request's `test_compile` never restored `main`'s cache, and compiled every third-party crate on 5 of 6 pull requests; restoring it saved crates compiled, not time.
What did pay, on 6 Nov: type-checking Armada's own crates on four threads with no debug info takes 31% off `fleet`'s test harness (*Four threads and no debug info*).
What did not, on 6 Nov: splitting `fleet`, measured as a prototype that took 13 s off a shard's 265 s compile (*Splitting fleet*).

Measured on 5 Nov 2026 on a 4-core, 16 GB `ubuntu-latest` runner, run 42111453908, from a branch that was never merged.
The job was named `test` so rust-cache restored `main`'s own entry, `ci1-test-test-Linux-x64-0fb4cc59-4b1d9d8e`, as a queue shard does.
`ci-test-compile-units.csv` beside this holds every unit cargo's `--timings` report named; `ci-test-compile-links.log` every link.

## The compile

`cargo test --workspace --exclude acceptance --no-run --locked --timings`: the test binaries of the whole `test` Check.

| | Seconds |
|---|---|
| Wall time | 278 |
| Units compiled | 38, every one a workspace crate. No third-party crate compiled: rust-cache holds all of them |
| Test harnesses (`lib (test)`, `bin (test)`, `test`) | 576 of compile time across units |
| Libraries and the `armada` binary | 300 |
| Build scripts | 2 |
| Linking, 22 executables | 14.6 in all; the longest, `armada`'s test binary, 3.3 |

| Unit | Starts | Takes |
|---|---|---|
| `fleet` lib (test) | 66 | 212, and ends the build at 278 |
| `armada` lib (test) | 131 | 111 |
| `armada` lib | 111 | 110 |
| `api` lib (test) | 84 | 78 |
| `fleet` lib | 58 | 73 |
| `ipc` lib (test) | 7 | 64 |

From 131 seconds on, `fleet`'s test harness is nearly the only thing compiling on four cores.

## sccache

sccache refuses any Rust compile that is not an rlib or a staticlib (`src/compiler/rust.rs`, `crate-type`), so a test harness is never a hit.
Warm, it could take the libraries before `fleet`'s test harness, at most the first 66 seconds.
Its entries also have to be written where a merge group can read them, which is `main`'s scope only, and after #3308 a push to `main` the queue already passed compiles nothing.
Not taken.

## A nextest archive

`cargo nextest archive` of the same set wrote 371 MB, 19 binaries, and uploaded in 5 seconds.
A queue shard on 5 Nov 2026 spent its 427-second Check about 162 seconds compiling and 265 running 941 tests.

| Per queue run | Today | One compile job, four shards from the archive |
|---|---|---|
| Path to the last test | one shard, about 7.6 min | compile job about 3.4 min, a wait for a runner, then a shard about 4.8 min |
| Runner time on `test` | about 30 min | about 23 min |

It saves runner time and lengthens the path a landing waits on, by about 0.6 minutes and a runner wait, which was a median 5.3 minutes before #3308.
Not taken: the brief was a faster landing.
Worth measuring again if runner waits fall after #3308 lands.

## What was found instead

`test_compile`, a pull request's job, passed `key: test` to rust-cache, which adds the job's id to the key.
It looked for `ci1-test-test_compile-…`, which no run on `main` writes, and printed `No cache found` on 5 of the 6 pull requests before 5 Nov 2026; its Check took 281 to 352 seconds.
It now restores `ci1-test-test-…` (`docs/practices/ci.md`, *What runs when*), and compiles 61 crates where it compiled 318, but no faster (*After*).

## After

Read on 5 Nov 2026 from the real runs either side of #3308 and #3310, every one green.

| | Before | After |
|---|---|---|
| A push to `main`: jobs that ran | 23, on each of 6 pushes | 6, on each of 2 pushes |
| A push to `main`: `ci` green after it started | 820 to 941 s | 181 and 196 s |
| A queue run's `plan` | 121 to 194 s, 18 runs | 28 to 31 s, 3 runs; the two started before `main`'s binary was saved built it, 140 and 147 s |
| A pull request's `plan` | 125 to 182 s | 25 s, #3314 |
| A queue run's slowest `test` shard | 116 to 704 s, 14 runs | 607 and 698 s, 2 runs. Nothing here changed the shards |
| `test_compile`, whole workspace and `acceptance` | 281 s cold, 318 crates, run 42111811461 | 366 s warm, 61 crates, run 42113789320 |

The test compile inside those two `test_compile` runs took 229 s cold and 284 s warm, and the timing trial's 278 s warm: the same work varies by a minute from runner to runner, and the third-party crates compile beside `fleet`'s harness rather than before it.
A narrowed `test_compile` with the cache has not run yet.

## Four threads and no debug info

Measured on 6 Nov 2026 on the same runner, runs 42136994178 and 42138933640, from a branch never merged.
Each variant touched `crates/fleet/src/lib.rs` and ran `cargo rustc -p fleet --lib --profile test` with every dependency warm, three times each, interleaved.

| `fleet`'s test harness alone | Mean s | Runs s |
|---|---|---|
| As `main` builds it | 134.6 | 135.3, 133.3, 135.2 |
| `-Cdebuginfo=0` | 119.8 | 127.2, 114.3, 117.8 |
| `-Zthreads=4` | 105.0 | 115.6, 97.5, 102.0 |
| Both | 92.9 | 89.9, 90.7, 98.2 |
| `-Ccodegen-units=16`, one run | 111.1 against 104.0 | Slower; not taken |

`-Ztime-passes` on one run of 112 s: type checking 28, borrow checking 18, coherence 15, the monomorphization walk 25, codegen 56 of which LLVM 28, linking 2.
The front end runs on one thread unless told otherwise, which is why four threads pay on a four-core runner.

So `test` and `test_compile` set `CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_PROFILE_TEST_DEBUG=0`, and `RUSTC_WORKSPACE_WRAPPER` to `.github/ci/rustc-threads.sh`, which adds `-Zthreads=4` and `RUSTC_BOOTSTRAP=1` for Armada's own crates only.
A third-party build script never sees `RUSTC_BOOTSTRAP`, so none switches to nightly-only code.
A failing test's panic still names its file and line; a backtrace in CI loses its line numbers.
A person's machine and the shipped build are unchanged.

Two other levers were measured and not taken.
`TMPDIR` on `/dev/shm`: the store and `fleet` suites went from 255 to 212 s, and 8 tests failed; the runner's disk is ext4 with `nobarrier` and `data=writeback`, so a sync is already cheap.
The slowest test, `a_numbered_file_converts_at_every_count_the_old_list_had` at 55 to 101 s, was cut instead by #3323.

## Splitting fleet

Measured on 6 Nov 2026 on the same runner with the shards' settings, runs 42161928164, 42164177926 and 42168375559, from branches never merged.

Splitting `fleet`'s code into crates was ruled out from its shape.
144 of its 220 modules form one cycle through `daemon`, and 139 modules, 84% of its code, add methods to `Fleet`, which cannot leave `Fleet`'s crate.

`fleet`'s harness compiled in 106 s, and in 14 s with `src/tests/` taken out, so its 88,800 lines of tests cost about 92 s.
`cargo llvm-lines` found why the rest of `fleet` costs so little there: `Fleet<H, V, W>` is generic over its three adapters, so its methods are generated only where the types are named.
`armada` generated 3.5 M lines of `Fleet` for the real adapters, the harness 3.3 M for the fakes, and `fleet`'s library none.

So the prototype, `armada/proto-dyn-fleet`, held the adapters as trait objects (`3f8737581e`) and then moved `src/tests/` to `crates/fleet/tests/suite` (`cbe9730709`).

| Seconds, two runs each | `main` | Trait objects | Trait objects, suite out |
|---|---|---|---|
| A shard's whole compile | 260, 271 | 258, 250 | 252, 253 |
| Every `fleet` test target | 122, 121 | 122, 123 | 113, 115 |
| `fleet`'s harness, or the suite once moved | 107, 109 | 107, 109 | 103, 105 |
| `armada` | 73, 83 | 70, 77 | 80, 78 |
| What `armada check test` rebuilds after a `daemon.rs` edit, incremental | 35.1 | 33.7 | 37.5 |

Trait objects moved 1.6 M generated lines from `armada` into `fleet`'s library, and neither compile moved beyond the 10 to 18 s two runs of one build differ by.
Moving the suite out took 3.7 M generated lines and 4 s off it.
The time is in the test code itself, wherever it compiles, and what in it costs most was not counted.
Not taken, so `docs/practices/rust.md`'s rule that unit tests live beside their code stands.

## What was not measured

| Not measured | Why |
|---|---|
| A shard's run from the archive | The trial's shard job wrote its start time to `GITHUB_ENV` without an `=` and failed before running a test |
| Why `cargo nextest archive` recompiled for 196 seconds after `cargo test --no-run` had built the same set | It relinked `fleet`'s and `armada`'s test binaries under the same hashes, so something in the fingerprint differed between the two |
| sccache on a runner | Ruled out from its source and the unit split, not run |
