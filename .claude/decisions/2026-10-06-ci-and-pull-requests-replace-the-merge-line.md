# Does this repository keep its own merge line?

**Decided 2026-10-06.** The owner moved the repository's landing onto GitHub that day, and retired the local line (`armada land`, `scripts/land`) at a cutover that is not finished. What is built, what is measured and what is owed are separate below.

**Chosen: GitHub CI and pull requests land this repository's code, and the merge line is retired.** The repository is now `github.com/Mele-Labs/armada`, in a free organization, since it was `NickMele/armada` and a personal account cannot use GitHub's merge queue.

**Measured the same day, on the owner's Mac.** Eight Claude sessions were running and memory was nearly full. The macOS security scanners were saturated, and a new binary took about 800 ms to launch until his terminal was given a Developer Tools exemption. Checks failed under that load, and a turn took fifteen to twenty minutes serially. GitHub runs each Check as its own job in parallel, a critical path of about five minutes.

- **The `checks` workflow** (`.github/workflows/checks.yml`, `docs/practices/ci.md`) runs on pull requests, merge groups and pushes to `main`.
- **A `plan` job asks `armada covers` which Checks a change reaches**, from either side, and runs none when the branch's own change hits no Check.
- **`ci` is the one gate.** `desktop_test` runs sharded four ways on macOS and is not required yet. `scripts_test` is gone with the line it tested.
- **A ruleset on `main` requires `ci`.** The repository allows merge commits only, and auto-merge is on. The owner, as admin, bypasses the ruleset so the draining line still works.
- **The line takes nothing new and drains what is queued.** It goes at the cutover.

**Measured on a scratch repository: GitHub ignores `merge=union`.** Append-only list files conflict on a pull request where the line merged them. The cutover converts them to one file per entry plus a generated index.

**Where Bridge goes.** Bridge's merge line becomes thin: every open pull request in the repository, and Fleet's knowledge of how they connect, which is its needs and its Job links.

**When `main` goes red,** he wants Armada to prompt him to dispatch a new Job, or to send the work back to a previous Job. He also wants Jobs to watch their own work as it lands, and to pick it up themselves when what they landed turned `main` red.

**Cost he took:** `desktop_test` stays outside `ci` until it stops flaking on the macOS runner, so a red shard does not block a change.

**Not built:**

| Owed | Note |
|---|---|
| Converting the append-only list files | `.gitattributes`, `docs/practices/list-files.md` |
| `needs` as a required check | Fleet will publish it |
| The thin pull request view in Bridge | Not designed yet |
| Prompting him when `main` is red, and Jobs picking up their own red landings | Not designed yet |
| Retiring `armada land` | `crates/armada/src/land/` |

**Where it landed:** the workflow in #1805, `scripts_test` dropped in #1806, the plan rule in #1810, and the agents' landing instructions in #1808, which is still open. The ruleset and repository settings are GitHub's, not files. This record and the docs that cite it are the written half.
