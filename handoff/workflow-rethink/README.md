# Handoff: rethinking Armada's workflow system

Written 4 Nov 2026 on the first Mac, for picking up on the other one. Nothing below is decided unless it says so.

## Where things stand

- **Workflows are switched off** (`features.workflows` / `workflows.off`). The owner turned them off on 4 Nov because they are "just not usable in their current form".
- **The owner wants to rethink the workflow system, and is not sold on beads.** Beads is one option. He has said he does not want to lose the rail: "I want to see what agents are doing."
- **The rail rebuild is not approved.** Two designs and a spike exist, and none of them is approved or built. They are summarised below, and their files are on pushed branches (see *Where the files are*).

## The problem, with evidence

Fleet's rail (`crates/fleet/src/rail.rs`) guesses which step a Job is on from proxies: the build phase, pushes, test-file paths, and plan task order. When a proxy is wrong, the agent gets a notice telling it to do something it already did, or should not do.

| Evidence | Count |
|---|---|
| Bug reports about the workflow system | 22 of 26 |
| Disputed Fleet notices (`fleet_disputes`) | 225 |
| Of those, about where the rail placed a Job | 116, plus 29 from the walk-ask check, which is not rail |
| Disputes ever reviewed | 4. Dispute reviews are off, by the owner's decision of 21 Oct (`features.disputeReviews`) |

The store is `~/Library/Application Support/Armada/armada.db`. Read it with `sqlite3 -readonly`.

Disputed notices by the rule that raised them:

| Rule | Disputes |
|---|---|
| Plan tasks out of order (`plan_record.rs` `stale_of`) | 35 |
| The reply asks for a walk with no window open (`session_host/reply_claims.rs:131`) | 29 |
| Step entered, instructions sent again | 27 |
| Edited before the plan step | 18 |
| Turn ended on a step, told to push or show | 16 |
| Pushed with plan tasks open | 11 |
| Test written before the walk was approved | 7 |
| Pushed before the plan was approved | 2 |

The other ~80 are slot, pull request, merge and subagent notices.

**Root cause:** the rail decides in Fleet and argues with the agent afterwards. Anthropic's article on folding subagents into the main loop (claude.com/resources/articles/building-agents-when-to-fold-subagents-into-your-main-agent-loop) puts the alternative plainly: procedure and order go in skills, and hard limits go in tools that refuse plainly.

## Options on the table

### A. The rail on beads molecules (designed and spiked, not approved)

Each workflow is cooked into a beads formula and poured as a molecule for each Job.
- **Step beads:** one bead per step. The agent claims and closes it through an Armada step tool.
- **Gate beads:** one bead between steps. Only Fleet closes it, from what it saw: your Approve, checks and judge, the merge.
- **Rail notices:** the proxy-based ones go away. A step held while its gate is shut is shown once, as a mismatch.

**Spike 40** (bd 1.3.2, throwaway database) found:
- **Works:**
  - A step stays blocked until its gate closes, and `bd` refuses to close it early.
  - `bd ready --gated` names the step to resume.
  - A molecule closes itself after its last step.
  - `bd swarm` runs a plan's groups as waves.
- **Does not work, so Fleet must fill it:**
  - Any agent can close a gate, and `bd mol current` even steers an agent towards one.
  - An agent can claim a step whose gate is shut.
  - Only the events journal records who closed something, and it is off by default.
  - A `gh:pr` gate resolves only on merge. It never fires on red `ci`, a conflict or a queue drop, and a closed PR only prints `ESCALATE`.
  - A human gate's timeout is never checked.
  - Each `bd` read takes 2.5 to 5 s, so Fleet must keep its own record and never call `bd` per notice.
  - `bd` reads formulas only from `.beads/formulas/<name>.formula.json`, so Armada's workflow files would have to be cooked into that form.
- **Leases** last 5 minutes and must be heartbeated. They are per machine and do not sync.

**The design's recommendations (not decided):**
1. Armada's step tool refuses to start a step whose gate is shut. Fleet's hook refuses a push before the plan is approved, and nothing else is refused. *Cost:* a one-line fix has to get a plan approved before it can push.
2. A repository whose tracker is not beads gets a private, unsynced beads database kept by Fleet. *Cost:* `bd` must be installed on every Mac. The alternative is keeping two rails.
3. Workflow files stay in Armada's shape, and Fleet cooks them into formulas when it pours. *Cost:* Fleet owns and tests that translation.
4. Pour molecules, which sync, rather than wisps, which are local, and filter step and gate beads out of the Issues list. *Cost:* about 10 beads per Job, roughly 120 a day.

It changes one rule in `docs/concepts/session/the-rail.md`: "the agent's word is never an input". The agent's claim becomes what Bridge shows it doing, but only Fleet ever moves a Job past a gate.

**Proposed order:**
1. Turn the 225 disputes into an eval set and replay them three times each against today's rail, as a baseline.
2. Cook a workflow into a formula and pour one per Job, behind a setting that is off.
3. Fleet resolves gates and reopens any it did not close.
4. The step tool and the refusals.
5. The step list on the wire and in Bridge, with a walk.
6. Run the evals, switch it on for Armada, then delete the old rail.

### B. The same shape without beads (not designed; worth weighing because the owner is not sold)

Spike 40 shows that Fleet ends up enforcing every gate itself anyway. Beads contributes:
- the dependency graph
- `bd ready --gated`
- swarm waves
- a synced record across machines and in the backlog

Without beads, the same shape would be explicit step state in Armada's own store:
- an agent-facing step tool, as in A
- gates Fleet closes from what it saw, as in A
- the proxy notices deleted

That means no `bd` dependency, no 2.5–5 s reads, and no gate-closing loophole to police. The cost is that steps don't appear in the backlog or on another Mac unless Armada syncs them itself. **The real question is whether a synced record of every Job's steps in beads is worth the dependency.** Everything else in A can be built either way.

### C. Smallest change: delete the proxy notices, keep the rest

Keep workflows as they are, and remove the notices that come from proxies: plan task order, edited/pushed before a step, and turn ended on a step. Keep only refusals that tools can make plainly. That is cheap, and it removes most of the disputes. It leaves Bridge's step display still guessed.

## Background delivery (designed, not approved, separate from the rail)

Today's delivery is slow, and every Job holds its slot until its PR merges.

| Measure (4 Nov) | Value |
|---|---|
| Median time from PR opened to merged | 28 min, up to 145 |
| PR `ci` run | 4–28 min |
| Merge queue run | 3–23 min |
| Merge queue runs failed in a row, 18:50 to 19:38 | 8, mostly `desktop_unit` |
| Named test runs recorded failed | 226 of 383 |
| Recorded failed at 895 s or more (really timeouts) | 20 |

**Cause of slow local checks:** compile contention, not cold folders. Every slot shares one compiler job pool (`crates/checks-runner/src/build_slots.rs`). The 900 s limit covers the build as well as the tests (`crates/checks-runner/src/run.rs:290`). A timeout is known as `Exit::TimedOut` (`run.rs:328`), but that is lost at `crates/armada/src/session_checks.rs:147`.

**Design recommendations:**
- **When it parks:** a Job with Auto land on is parked at its push once its PR is open, its tree is clean and nothing is waiting in Needs you. Its slot and agent are released, and the Job reads "Landing".
- **While it lands:** Fleet's existing 60-second pull reading drives landing. Fleet merges main in itself when the merge is clean (`git merge-tree`, no slot).
- **On red `ci` or a real conflict:** Fleet wakes the same Job, not a fresh ci-fixer. A queue failure on a branch whose PR run was green is requeued once first.
- **Preview:** it also includes the Landing branches.
- **Compile budget:** the compile gets its own budget (a `--no-run` prerequisite), and a Rust type-check runs before the PR opens (`session_host/pre_pr.rs`).
- **Timeouts:** they read as "timed out", not "failed".
- **Beads gates are not used to wake a Job,** since they miss red, conflicts and queue drops. A gate may mirror the wait into the backlog later.
- **Off-Mac options:** cloud sessions, remote subagents, sccache and a Checks-only VM were all weighed, and none is recommended now. A Checks-only VM is worth revisiting if timeouts persist.

## Live Jobs (first Mac, end of 4 Nov)

| Job | State |
|---|---|
| Workflow Rail on Beads (09460b5f, slot 12) | Design pushed. The owner is closing it on the first Mac |
| Background delivery: hand off at push (cf4801f0, slot 11) | Design pushed. The owner is closing it on the first Mac |
| Flaky merge queue and timed-out checks (45d39729, slot 4) | Working. Nothing pushed yet |
| Needs You Missing Items (19341cda, slot 5) | PR #3283 open: a late plan, both pages, only the newest sketch, Kit workflows |
| Workflow Rework (4d4e3d53, slot 6) | The grooming Job that wrote this |

Already merged: **#3276**, which makes Open show unmerged PRs and unclosed tickets on a stepped Job.

## Beads issues (sync to both Macs)

| Issue | What | State |
|---|---|---|
| armada-ddfe | The rail reads a step's own output, test fakes, handed tasks and no-code Jobs as skipped steps | Deferred, moot under A, B or C |
| armada-z2co | Wrong step: skips Walk, returns to it, re-asks approved walks, nudges after merge | Deferred, moot under A or B |
| armada-hq0b | Plan task notices fire on groups side by side | Deferred, moot under A, B or C |
| armada-mj3k | The mod misses a Job's push | Deferred. **Still needed**, because background delivery hands off at the push |
| armada-n2ie | The walk-ask check fires on replies that ask for no walk | Deferred, not rail work |
| armada-6epa | A cancelled GitHub job or a broken Buildkite job reads as failed | Deferred. Background delivery needs it |
| armada-fv69 | finish_up re-leases a slot, handoff slot, settled conflicts, question guard | Deferred |
| armada-io3f | A Job cannot end, withdraw, pause or re-chain the Jobs it started | Open |
| armada-djw3 | A Job reads its children's state and result in one act | Open |
| armada-pp22 | Logbook workarounds become bug fixes, not prompt lines | Open |
| armada-qjz8 | Measure which skills Jobs load, and prune the rest | Open |
| armada-10jz | Fold test-writing into the implementer's loop | Open |

## Where the files are (pushed, no PR)

| Branch | Files |
|---|---|
| `armada/session-09460b5f` | `docs/spikes/040-can-a-workflow-run-as-a-beads-molecule.md`, `040-transcript.txt`, `040-feature.formula.json`, `040-beads-rail-design.html` (design A, in full), plus an INDEX line |
| `armada/session-cf4801f0` | `docs/spikes/background-delivery-design.html` (the background delivery design, in full). It is not in `docs/INDEX.md`, so the docs gate refuses it in a PR until it is indexed or moved under `docs/design/` |
| `armada/session-4d4e3d53` | This file, at `handoff/workflow-rethink/README.md` |

```bash
git fetch origin armada/session-09460b5f armada/session-cf4801f0 armada/session-4d4e3d53
```

## What the two design Jobs learnt that is not in their pages

**Beads rail (09460b5f):**
- **Its four decisions were never answered.** They timed out in AskUserQuestion and died with the Job, so ask them again.
- **`bd mol current` with no id finds nothing between steps,** so Fleet must pass the agent its molecule id.
- **A claim's lease is 5 minutes,** and only the holder's actor can heartbeat it. The step tool must heartbeat as the agent.
- **From a slot, `bd` reaches the owner's real database** through git's common directory. Run beads experiments only in a fresh `/tmp` git repo with `bd init`.
- **Red `ci` against a `gh:pr` gate was never tested live,** because no open PR was red. The spike shows an open PR stays pending.
- **Only `feature` was cooked.** `bug`, with walks on a `follows_plan` step, is harder and untried.
- **Do differently:** build the dispute-replay eval first, so the design can quote a measured before and after.
- **A small live bug if the old rail stays:** notice text has a doubled period ("...works from.. Nothing is blocked").
- **Disputes are 225 rows, not 207.** Of those, 116 are rail-placement disputes, since the 29 walk-ask disputes are not rail.

**Background delivery (cf4801f0):**
- **The main trap for the build:** an unpaused Job that gets a slot again gets a fresh `session-<id8>` branch, not its pushed branch (`gate.rs` ~405–415). Only a paused Job is re-leased on its own branch, so the wake must go through the paused route or extend `lease_taken`.
- **The quiet timeout already ends the agent process after 10 minutes,** so after a push a Job mostly holds a slot, not a live agent. The win is smaller than it first looked, and slots held by open-PR Jobs were not measured.
- **The `pre_pr` gate has its own 4-minute budget,** used today only by TypeScript `format_check`. The Rust typecheck joins it with a one-line manifest change.
- **`took_ms` and `started_at` include the wait for a Check slot** (`crates/armada/src/main.rs` ~215), and the store has no wait time. Rows still `running` are probably abandoned.
- **A notice to a paused or ended Job is dropped and retried** at the next reading (`serving/sending.rs:44`), so a parked Job is silent, not broken.
- **`scripts/preview` reads slot state,** so Landing branches need a new source (a Fleet route or a store read). Fleet never restarts the preview, by the owner's rule (`building.rs:15`).
- **Cloud sessions and `isolation: remote` are blocked at the root.** Every mod hook posts to 127.0.0.1 and reads `fleet.json`, so remote Jobs first need Fleet reachable over the network.
- **Read the store with `sqlite3 -readonly <path>`.** `file:...?mode=ro` fails with code 14 against the live WAL database.
- **Do differently:** query `session_check_runs.target` first. One query overturned the "skip the suite" idea, after three exploration agents had already run.
- **Its plan (6 groups, version 1) and page are unapproved.**

## Decisions waiting on the owner

1. **Which rail:** A (beads), B (the same shape in Armada's own store), or C (delete the proxy notices only).
2. **If A or B, which refusals,** if any: refuse to start a gated step, and refuse a push before the plan is approved.
3. **If A, how non-beads repositories are handled,** and whether to pour or wisp.
4. **Background delivery:** approve, change or drop. It is independent of the rail choice.
5. **The flaky-tests Job (45d39729) and PR #3283** are still live on the first Mac. They are independent of the rail choice.

Lessons from the way 4 Nov went:
- **Too many child Jobs.** Eleven were started, and stopping them took eight manual steps (armada-io3f).
- **Hidden state.** A parent could not see its children's state without reading their transcripts (armada-djw3).
- **A broken question guard.** It refused AskUserQuestion twice when a full answer had already been written (armada-fv69).
- **Keep the next round small:** one decision, then one Job.
