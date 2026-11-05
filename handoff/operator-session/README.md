# Handoff: a spitballing session, carried into Armada

Written 5 Nov 2026 in a cloud session, for picking up in a Job on the owner's Mac. Nothing below is decided unless it says **Decided**. Nothing was built, and no Job was started by this session. Every Job named as running was started by the owner.

## How to read this

The owner said this session is "exactly what I am looking for out of an operator": spitball, organise, dispatch a Job where one is needed, file an issue where it is not. Treat this file as the record of that session and as a model of the behaviour wanted.

| Owner preference seen in the session | Practice |
|---|---|
| ADHD, many things feel critical at once | **One decision, then one Job.** Say which one thing to focus on, and park the rest in a list. |
| Wants the answer first | Lead with the direct answer, then the evidence. |
| Prefers tables and diagrams | Comparisons, sequences and data go in tables. |
| Wants claims checked | Say what was read and what was not. Correct an earlier claim plainly when the data contradicts it. |
| Does not want work started when he is only thinking | "Just need a place to organise my thoughts, don't do any work" was the opening instruction. Brief and wait. |

## What the owner is choosing between, in one line each

| # | Job | State |
|---|---|---|
| 1 | **Operator**, read-only first | **Running** (started by the owner) |
| 2 | **Check and test load** | Measurement done (below). Brief needs a rewrite to the new order. |
| 3 | **Rail option C**, delete proxy notices | Brief written. One edit owed: remove the push-before-approval rule. |
| 4 | **App performance** (sluggish UI, data flood) | No brief. Measure first. |
| 5 | **Font and typography pass** | No brief. Small, independent. |
| 6 | **Unified search** | No brief. Keystone for fragmentation, finding and filtering. |
| 7 | **Filters and saved views** | No brief. Needs 6. |
| 8 | Re-enable workflows, re-measure disputes | Needs 3. |
| 9 | Decide rail A or B from the new dispute count | Needs 8. |
| 10 | Handoff design between machines | Needs 1. |
| 11 | Background delivery, reframed around fast iteration | Direction approved. |
| 12 | Needs You panel and slot narration in the thread | Needs 11. |
| 13 | Widen the operator beyond read-only | Needs 1. |
| 14 | **Project and workspace model** (tasks that know their repos and areas) | Design only. A read-only look at existing docs comes first. |

**Check and test load was ranked second because it slows every Job after it.** That ranking is my recommendation and the owner has not confirmed it.

## Decided

| Decision | Detail |
|---|---|
| First Job is the operator | Chosen by the owner. |
| The operator starts read-only | Chosen by the owner. It widens to supervised, then full, later. |
| **No nudging** | The owner is hesitant about any notice that nudges, "because it never seems correct". So the "pushed before the plan was approved" rule (2 disputes) goes with the rest. The Rail C brief must drop it and its question. |
| Background delivery: yes, in direction | The goal is a faster way to iterate, not mainly slot release. See below. |
| Focus rule | One decision, then one Job. |

## Open decisions

| Decision | Blocks |
|---|---|
| Rail: stay on C, or go on to A (beads molecules) or B (same shape in Armada's own store) | 9, after 3 and 8 |
| Handoff between machines: reach the other Fleet, or move the Job? Seed a new Job from a note, or transplant state? Auto-push a WIP branch, or block until clean? | 10 |
| When to widen the operator | 13 |
| Is a project's area scope a hard fence or a default focus? | 14 |
| Where do tasks live: beads, a tracker, or Armada's own store? | 14 |
| Can one project span several repos? | 14 |
| What does the owner run by hand when the machine takes off? | The order of steps in 2 |

## Job 1: the operator

Part built on another machine. Brief handed to the owner's Job, in summary:

- Read-only first, widening to supervised then full.
- The operator is a client of the same Job API the UI uses, with no private back door.
- A Job it starts can never outrank the operator's tier.
- A message to a busy Job queues as its next turn. Interrupting is a separate explicit action.
- "Waiting on a human" is its own signal and matches what Needs You shows.
- Operator access is revocable from one place.
- In scope: `armada-io3f` (a Job can end, withdraw, pause or re-chain the Jobs it started) and `armada-djw3` (a Job reads its children's state and result in one act).
- Evidence: on 4 Nov a parent started eleven child Jobs and stopping them took eight manual steps.
- Acceptance test first, per `docs/practices/acceptance-tests.md`.

The design rules above came from reading how a comparable coding-agent app exposes thread control to outside agents (a small tool surface, permission tiers with a ceiling, a host CLI over the same tools, exit code 2 for "needs a human"). That app is not named here on purpose; see *Not named here*.

The owner's wish for the operator, in his words: spitballing, organising, dispatching Jobs where necessary, and filing issues otherwise.

## Job 2: check and test load

### Measured (read-only Job, 5070 runs, 9 Oct to 5 Nov, `session_check_runs`)

| Finding | Number |
|---|---|
| Runs of 5 min or more | 257 of 5070 (5%); 56 took 10 min or more |
| Where the hours are | Named-filter test runs hold 74.7 of 87.7 run-hours (3968 runs); whole test runs 10.0 h (689 runs) |
| Average load | About 0.14 Checks running at once; bursts, not constant. Peak overlap 7 (29 Oct) against a default cap of 8 |
| Tail grows with overlap | Whole test runs: p50 1.1 s and p90 63 s alone; p50 4.3 s and p90 334 s with 4 to 7 others |
| Test failure rate | 45%. 580 of 2097 test failures took under 1 s; at least 273 are a filter that matched no test, and 27 were a wrong check name |
| Timeouts | 15 runs at 900 s or more; 14 failed. Rare |
| `--changed` use | 22 runs (0.4%). Agents use named filters (82%) or whole runs (18%) |
| Concentration | One Job ran 650 Checks; another accounts for 8.0 h |
| Stuck `running` rows | 0 |

Three caveats from the handoff doc, checked against the code by that Job: `took_ms` includes the wait for a Check slot (true); a timeout is stored as `failed` (true); `running` rows are abandoned (false in this data, Fleet fails them when a Job ends or pauses).

### What the table cannot show

- It holds agents' `armada check` runs only. Fleet's own gate runs, the merge line and anything run by hand share the same Check places and are not in it.
- It does not say which machine it came from. The owner said the other machine, with several slots running, was much slower.
- Browser checks (`app_smoke`, `components_test`, `desktop_test`) are about 140 short runs. The Chromium pile-up the owner sees in Activity Monitor (10 or more processes) is not explained by this table.

### Hypotheses for the Chromium load, both unproven

| Hypothesis | Evidence in the repo |
|---|---|
| Gate runs of the browser suites | `packages/components/armada.yml` sets `runs_at: gate` for `components_test` |
| Agents running vitest or Playwright by hand | `.claude/hooks/guard_test.py` refuses bare `cargo test` and `cargo nextest` only. Nothing guards `vitest`, `playwright` or `pnpm exec` |

`packages/components/armada.yml` (about lines 50 to 73) records an earlier incident: "A dozen Chromium processes against a shipped limit of 5 timed out unrelated browser tests sharing the machine on 14 Sep. #1102." `components_test` now takes `places: 3` and `--maxWorkers=${width}`, which only applies when it runs through Armada.

A second read-only prompt was written to search the store and the transcripts for by-hand browser runs. **It has not been run as far as this session knows.**

### Revised order for the Job

| Step | What |
|---|---|
| 0 | Record what is missing: a `timed_out` state (`core-model/src/workflow/check.rs:31` already has one), a queue-wait column, a compile versus test split, and gate and merge-line runs in the same view |
| 1 | Fix the noise: a filter that matches nothing is its own "no tests matched" result, not a failure; an unknown check name is refused up front with the valid names listed |
| 2 | Find the heavy runs: why 689 whole test runs happened, and why one Job ran 650 Checks |
| 3 | Measure the owner's Mac during a gate run, Rust and Chrome alone and together |
| 4 | Admission control across slots only if step 3 shows gate runs overlap badly. The data does not support it yet |

`armada check test --changed` already exists (`crates/CLAUDE.md`). The gap is not that it is missing but that it is barely used.

### Related, from the repo

- Store: `session_check_runs` has `session_id, manifest_id, slot, name, state ('running'|'passed'|'failed'), started_at, ended_at, took_ms, log, target`. Read it with `sqlite3 -readonly`.
- The Usage page's Time view already shows medians by hour including "a Check run's time, queued or running" (`crates/store/src/time_summary.rs`, `get_usage_time`). There are no per-test timings, failure-rate or flake views, and no CPU or memory figures.
- `armada-6epa` (a cancelled GitHub job or a broken Buildkite job reads as failed) overlaps. Check whether it is subsumed before filing anything new.
- A code-quality tool (Qlty) was discussed and **not recommended now**: it lints, formats and reports smells, and does not address heavy tests or compile contention. Revisit only if measurement shows lint and format are a real cost, or for a separate code-health Job (duplication, complexity) that could trim redundant tests.

## Job 3: Rail option C

The 4 Nov handoff (`handoff/workflow-rethink/README.md` on branch `armada/session-4d4e3d53`) described the problem: Fleet's rail guesses a Job's step from proxies and argues afterwards. 225 notices were disputed; 116 were about rail placement; only 4 disputes were ever reviewed. Workflows are switched off (`features.workflows`).

| Rule | Disputes | Under C |
|---|---|---|
| Plan tasks out of order | 35 | Remove (named) |
| Edited before the plan step | 18 | Remove (named) |
| Turn ended on a step, told to push or show | 16 | Remove (named) |
| Pushed with plan tasks open | 11 | Probably remove |
| Pushed before the plan was approved | 2 | **Decided: remove** (no nudging) |
| Step entered, instructions sent again | 27 | Settle first |
| Test written before the walk was approved | 7 | Settle first |
| Walk asked for with no window open | 29 | Stays; not rail work (`armada-n2ie`) |
| Slot, pull request, merge, subagent notices | about 80 | Stay |

**A correction made in the session:** C does not remove "most of the 225". Only 116 are rail placement, about half. Certain: 69; likely: 82; at most 116.

The full C brief was written in the session. Its one owed edit is to delete the push-before-approval rule and its question. `armada-ddfe`, `armada-z2co` and `armada-hq0b` become moot under C, but the brief should confirm that first, because `armada-z2co` was described as moot under A or B.

### Rail A, B and C in short

| | A: beads molecules | B: same shape, own store | C: delete proxy notices |
|---|---|---|---|
| Cost | Highest | Medium | Lowest |
| Real step display in Bridge | Yes | Yes | No, still guessed |
| Steps synced across Macs and in the backlog | Yes | Only if built | No |
| Needs `bd` on every Mac | Yes | No | No |

### What the beads documentation says (read this session)

| Fact | Source |
|---|---|
| A molecule is an epic with one child bead per step. Pouring makes persistent beads that sync like any bead. | `beads.gascity.com/workflows` and `/workflows/molecules` |
| A gate is "a bead like any other". A step that declares a gate gets a companion bead `<parent>.gate-<step>`. | `/workflows/gates` |
| So a Job's beads are about 1 + steps + gated steps. The handoff doc's "about 10 per Job, roughly 120 a day" is consistent with that, but the real step and gate count was not seen. | derived |
| Wisps are ephemeral, excluded from federation push by default and deleted by `bd purge`. `bd mol squash` promotes one to persistent. That is a possible middle path the 4 Nov design did not consider. | `/workflows/wisps` |
| TOML is the preferred formula format, with JSON accepted, from `.beads/formulas/`. The 4 Nov spike said `bd` 1.3.2 read only `.formula.json`. Possibly a version difference; check. | `/workflows/formulas` |

Access note: `beads.gascity.com` was added to the owner's Default cloud environment's allowed hosts in this session. It applies to his future cloud sessions.

## Job 11: background delivery, reframed

The 4 Nov design parks a Job at push as "Landing" and releases its slot. The owner's actual goal is a faster loop. Three needs:

| Need | Direction | Exists? |
|---|---|---|
| Use a feature before the full checks finish | Preview of the branch right away | `scripts/preview` merges in-flight branches into a preview. Why it is not enough was not checked. |
| Do not wait on repo-wide checks to start using a feature | Changed-only local checks, with the full suite in CI | Part of Job 2 |
| Survive a broken build | A last-known-good build and a one-step switch back | Not checked |

Rewrite the Job around these three, and keep Landing as one part. Prerequisites from the 4 Nov doc: `armada-mj3k` and `armada-6epa`.

## UI pains (the owner's words, grouped)

| Pain | Direction suggested | Size |
|---|---|---|
| **Sluggish app.** "Everything tries to flood the app at the same time." The owner guesses animations. | Measure first: data flood on start and on opening a Job, long lists rendered in full, re-render storms, animations, per-row slow calls. Fix the top two. | Medium |
| **Fragmentation** of projects/tasks, issues, bug reports and the logbook | One shared item view with a type, a status and a link to its Job. Do not merge the stores yet. | Medium |
| **Cannot quickly find anything** | One search across Jobs, issues, bug reports and logbook entries, grouped by type | Medium |
| **Volume** of issues and bug reports, no good filter | Saved filters on status, type, age and owner; a default view that hides closed and moot items | Small-medium after search |
| **Thread font** feels not quite right | Typography pass: family, size, line height, paragraph spacing, code font. The owner's reference is the thread view in Claude Code desktop. | Small |

Closing the three moot beads issues under C and fewer disputes both reduce volume without UI work.

## Idea: one project that knows its repos and areas

The owner works in a monolith at work but, for a given project, only in certain areas of it. The idea: log tasks into one cohesive project that already knows which repo or repos it spans and which areas of each it should deal with.

- Armada already has a **workspace** inside a repo: a directory with its own `armada.yml`, "even inside a large monolith", with an optional root `armada.yml` and per-workspace Check coverage by path (`docs/concepts/manifest.md`, found by search; only partly read).
- The new layer is a **project** above that: repos, plus areas within each, plus tasks, with Jobs, checks, search and settings inheriting the scope.
- What a scope could drive: which checks run (so one area does not run the whole monolith's tests), default search and list filters, per-project settings, and what a Judge counts as out of scope.
- Not read: `docs/concepts/project.md`, `tracker.md`. A read-only look at what exists, and what is missing, comes first.

## Handoff between machines

The 4 Nov handoff itself shows the problem: state sits on one Mac as pushed branches and live Jobs. The research on how a comparable app does it:

| Idea | Why it matters here |
|---|---|
| Execution stays on the machine that owns the workspace; the client moves | Is "handoff" really "reach the Fleet that has the Job", or "move the Job"? |
| Cross-machine work is a new thread plus a generated handoff note, not a state transplant | A new Job seeded from a summary is cheaper than moving state |
| Code travels by git, and uncommitted work stays behind | Handoff needs a "push first" step |
| A stable environment identity that survives address changes, and an ordered list of routes | Fleet needs an identity before any link exists |
| An access ceiling and an approval card | Required if one machine's agent can start work on another |

Not read: the primary source for the fork-and-handoff-note behaviour. Those PRs were known only from search summaries.

## Other lessons carried over from the comparison

| Lesson | Applies to |
|---|---|
| Anything a user can trigger is a service method that agents reach through the same tools, not UI handler code | The operator |
| An acknowledgement means the intent was committed, not that the work finished; side effects run afterwards and report back as events | Slot creation narrated in the Job thread; optimistic loading |
| The server owns "is this thread done" with no client connected | The operator owning settlement |
| Run only changed checks locally; cap parallelism by cores and memory; report a timeout as its own result; CI owns the full suite | Job 2 |
| A separate preview channel for any branch that no release user is offered; stable is a verified nightly that is promoted | Preview without holding slots |
| Subscriptions send only what the viewed thread needs; audit payload size and list rendering | App performance |
| Client preferences stay on the client; environment defaults and project overrides stay on the owning server | Workspaces and per-workspace settings |
| "Hit every surface" checklist before calling a feature done (entry points, clients, providers, agent route) | A habit worth copying into Armada's practice docs |

## Parked, no order

Snooze. Workspaces and per-workspace settings (now folded into the project idea). Move System into the top-right settings menu as a 90% centred overlay. Default tab layout when walks open. Better document generation. Smoother Job start. Agent reasoning not showing in threads. Multi-harness and provider. Mods and plugins. Nightly and preview builds. The Fleet notices decision (mostly answered by Job 3). Better cache and slot management. Refine Needs You and every state it can be in; slot creation and renewal "sucks", and creating a slot should walk through in the Job thread what is happening.

On beads and the work model: the owner asked how workflows, steps, plans and tasks fit together and where beads fits. The 4 Nov handoff and the beads documentation are the current state of that question. The owner is not sold on beads, and does not want to lose the rail view: "I want to see what agents are doing." No subagent direction: let harnesses and agents drive what they need from the workflow and plans.

## Issues a Job could file now

None were filed. Candidates, for the operator session to confirm and de-duplicate against `bd list`:

| Candidate | Source |
|---|---|
| A filter that matches no test reads as "no tests matched", not as a failure | Check data |
| An unknown check name is refused up front with the valid names | Check data |
| Add a `timed_out` state to Check runs (the type already exists in core-model) | Check data; also the 4 Nov doc |
| Record queue wait separately from run time | Check data |
| Record compile versus test time | Check data |
| Record gate and merge-line runs alongside agent Check runs, or show them together | Check data |
| Guard bare `vitest` and `playwright` runs the way `cargo test` is guarded, if the transcript search shows agents run them by hand | Hypothesis, unproven |
| Rail C: remove the proxy notices (Job, not an issue) | Handoff doc |

## Things said in the session that were later corrected

| Earlier claim | Corrected to |
|---|---|
| C "removes most of the 225 disputes" | At most 116 of 225; certain 69 |
| One running Job rules out cross-slot contention | The data shows modest contention, but the table omits gate runs and manual runs, so it cannot rule it in or out |
| Admission control across slots comes second | Demoted to last; the evidence does not support it yet |
| "Armada lacks a changed-only mode" (implied by a comparison) | `armada check test --changed` exists; it is barely used |

## Not verified

- Nothing in this file was checked against a running Armada. The repo was read, not run.
- The Rail C brief and the check-and-test brief were written from the 4 Nov doc and a repo search. The file paths and line numbers in them come from that doc and from grep, and each Job should verify them.
- Direct messaging from a Job on the owner's Mac to the cloud session was set up as a test. It was never confirmed. Fleet's own MCP server did not connect in the cloud session (`armada` was not on its PATH). A cloud session cannot reach the owner's machine unless Remote Control shares a folder with it.

## Not named here

The comparison app was deliberately left unnamed, and its file paths, internals and quotes are left out. `CLAUDE.md` says to name no other repository, or its internals, in an issue, pull request, commit or code unless a human says to. The owner can say to, and the session that read it can supply the names and sources.

## How the operator session should work from here

1. Read this file, then ask the owner **one** thing: which of the numbered Jobs to start next. Recommend one.
2. Brief a Job with a self-contained prompt (goal, evidence, files to verify, "do not", done-when). Read-only measurement before any fix.
3. File an issue instead of a Job when the work is a single well-bounded change.
4. Keep the parked list parked. Do not pull an item forward without the owner saying so.
5. Never drop or reverse a decision above without asking him first.
