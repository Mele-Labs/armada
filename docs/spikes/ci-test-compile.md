# Spike: Where does a CI test shard's compile go, and would sccache or a nextest archive shorten it?

**Answer: neither pays on landing time, so neither was taken.**
The compile of the whole `test` Check is 278 seconds of wall time, and 212 of them are one unit: `fleet`'s own test harness, which starts at 66 seconds and ends the build.
sccache caches only rlib and staticlib compiles, so at best it takes off the libraries before that unit; a nextest archive saves runner time but puts a second job, and a second wait for a runner, on the queue's path.
What it did find: a pull request's `test_compile` never restored `main`'s cache, and compiled every third-party crate on 5 of 6 pull requests.

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
It now restores `ci1-test-test-…` (`docs/practices/ci.md`, *What runs when*).

## What was not measured

| Not measured | Why |
|---|---|
| A shard's run from the archive | The trial's shard job wrote its start time to `GITHUB_ENV` without an `=` and failed before running a test |
| Why `cargo nextest archive` recompiled for 196 seconds after `cargo test --no-run` had built the same set | It relinked `fleet`'s and `armada`'s test binaries under the same hashes, so something in the fingerprint differed between the two |
| sccache on a runner | Ruled out from its source and the unit split, not run |
