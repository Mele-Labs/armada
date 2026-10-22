# Spike: What does a fresh test map cost to seed, and how far does it narrow a change?

**Answer: a seed is a two-hour job on the forge's runner, and it narrows a typical commit to about a
tenth of the suite. A change to a root file of `core-model` or `config` still reaches about a
third.** Only the runner was timed. The map is measured on a Mac nowhere in this record.

Measured on 22 Oct 2026, from the first `test_map` job on `main`: run 39728088971 of
`.github/workflows/checks.yml`, commit `8465a31c389ff6aae7fcc50506ed5dba42dfe192`, on GitHub's
ubuntu runner. The job is described in `docs/practices/ci.md`, *The test map's laps*; what the laps
feed is `docs/concepts/test-map.md`.

## The seed

`armada test-map record --check test` ran from 05:28:59 to 07:22:57 UTC.

| | |
|---|---|
| Wall time | 1 h 54 min |
| Includes | The instrumented build in `target/test-map`. The log does not split build from run |
| Job `timeout-minutes` | 120, so the run finished 6 minutes inside its ceiling |
| Laps | 4261 from the nextest coverage collector, one per test, each naming the files it reached |
| Artifact `laps.json` | 6.7 MB |

## What a test reaches

| | Files per test |
|---|---|
| Median | 8 |
| p90 | 145 |
| Max | 203 |
| Tests reaching no file | 0 |

1340 files were reached in all. Of the 963 non-test `.rs` files under `crates/` at that commit, 774
are reached and 189 are not.

| Crate | Files no test reached |
|---|---|
| ipc | 56 |
| fleet | 51 |
| api | 30 |
| adapter-traits | 11 |
| store | 10 |
| config | 8 |
| core-model | 6 |
| adapters | 5 |
| testkit | 5 |
| armada | 4 |
| verification | 2 |
| checks-runner | 1 |

These are likely type-only or wire-only modules with no executable line, and code no test runs.
**Not checked file by file**, so the split between the two is unknown.

## What a change reaches

The changes are those in the last 300 non-merge commits touching `crates/*.rs`, read against the
laps of the one commit above, so a file's edge is as it was at that commit.

| | Sample | Median | p75 | p90 |
|---|---|---|---|---|
| Tests reaching one changed file | 1988 changes to source files that still exist | 17 | 165 | 722 |
| Tests reaching one commit's changed files, as a union | 272 commits | 448 | 999 | 1192 |

Of the 1988 changes, 518 are to a file no test reaches. The union is of 4261 tests.

The widest files:

| File | Tests reaching it |
|---|---|
| `crates/core-model/src/envelope.rs` | 1774 |
| `crates/core-model/src/workflow/ids.rs` | 1659 |
| `crates/config/src/yaml.rs` | 1590 |
| `crates/config/src/manifest.rs` | 1458 |
| The files of config's `manifest` module | About 1455 each |

## What it means

- A median commit runs about a tenth of the suite (448 of 4261), and a commit at p90 runs 28%.
- A change to a root file of `core-model` or `config` still reaches 1458 to 1774 tests, about a
  third to two fifths. The map narrows most for leaf modules.
- A fresh seed is a two-hour job. That is why it runs only on a push to `main` and is fetched, and is
  never recorded on a shared machine (`docs/concepts/test-map.md`, *How it stays current*).

## What was not measured

| Not measured | So |
|---|---|
| The seed on this Mac at low priority | Only the runner was timed. Whether a sleeping bay finishes a seed in a night is unknown |
| How many tests the map would cut from a real agent run, end to end | The figures above are edits from history, not the edits an agent makes |
| How well the reading and static sources agree with measured edges | No edge here came from either |
| One run | The wall time and the laps are of a single job |
