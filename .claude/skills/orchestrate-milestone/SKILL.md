---
name: orchestrate-milestone
description: Running a whole milestone by dispatching agents at its issues — sequencing, briefing, verifying what comes back, merging, and what to decide versus what to bring the owner. Load before starting a milestone, not before starting an issue.
---

# Orchestrating a milestone

**One agent per issue, and you are not one of them.** Your work is sequencing,
briefing, checking what comes back, and merging. `work-issue` is what each agent
loads; this is what you do.

Focus was run this way on 30–31 Aug 2026 — eight issues, merged in a night, and
the milestone's own acceptance test passing at the end. Everything here is what
that cost to learn.

## Before dispatching anything

**A milestone whose issues are one-liners is not ready and no amount of briefing
fixes it.** Focus ran clean because every issue named the file, the failure each
decision existed for, and the wrong fix to avoid. Throughput's five said
*"Migrated from the project's design notes"* and nothing else; they had to be
rewritten before anyone could start.

**Settle the open questions first, with the owner.** Not during. An agent that
hits an undecided question stops, and a stopped queue at 3am is a wasted night.
Read every issue and its comments, list what is genuinely undecided, and put them
to the owner in one pass.

**Sequence by dependency and say what blocks what.** Focus was: the claim and the
acceptance test alone, then the schema and the artifact in parallel, then the
brief, then the lifecycle switch, then the model dial. Each one made the next
buildable. Getting this wrong wastes a whole agent.

## The brief

Every brief carries these, and each is here because leaving it out cost something:

- **The skill to load.** `work-issue` first, always.
- **The slot it works in.** The path `agent-worktrees` says to lease, in the
  brief; release it after the merge.
- **The current verification numbers**, exactly: tests with `--exclude
  acceptance`, acceptance separately, and what `verify-foundations` reads on
  `main`. Ask for the delta, not the absolute — an agent reporting "86 failing"
  has told you nothing without the baseline.
- **What has changed since the issue was written.** Issues written a week ago
  describe a codebase that has moved, and an agent trusting a stale `## In`
  builds the wrong thing. Name what merged and tell it to verify rather than
  trust your summary.
- **Scope discipline**, including who else is working where
  (`work-issue`, *Dispatching several agents at once*).
- **The `**QUESTION:**` rule**, and explicitly: *if something needs deciding that
  nobody has decided, put it in your question line rather than deciding it.*
- **The Bridge half**, if the change crosses the wire. A feature means both
  halves unless the owner says otherwise.
- **Commit and push after each piece that passes.** A restart or crash loses
  what is uncommitted (5 Oct 2026: a wire agent resumed twice with 36
  uncommitted files; another died with a deliberate break still applied, found
  only because the parent read `git diff`). An agent that breaks a behaviour on
  purpose reverts it and checks `git status` is clean.
- **Wait for its own runs in the foreground.** An agent is woken only by a
  message, never by a background build finishing (14 Sep 2026: an agent ended
  its turn three times with `cargo build` still running). Say *"run heavy
  commands in the foreground and wait; never end your turn while a run is in
  the background"*.
- **A brief points.** Each file as `path:line`, at most two skills (the one the
  work needs, plus `commit-message`), and *"read by range — grep first, never
  read a file over 300 lines whole; tail test output"*. An agent starts cold,
  and reads were 55% of what a job took in (2 Oct 2026: four agents with five
  skills each used 61k–551k tokens apiece).
- **The Checks the line will run**, because an agent never loads this skill:
  *"run every test module that references what you changed, then `armada check
  typecheck` and `cargo xtask verify-foundations`; a new operation needs
  `tests::served`."* Each of a route-table test, a stale fixture and a
  story-title rule went red on the line and green in an agent's report (4 to 5
  Oct 2026).
- **The model.** Mechanical work (a merge or conflict, a doc or registry edit,
  a fix whose cause the brief names) goes to `model: "sonnet"`; design, visual
  work and anything crossing the seam keeps the default; a lookup or a run that
  only reports what failed goes to `"haiku"`. `docs/practices/subagents.md`
  has which type takes what.
- **The owner's rules.** Bottom line first, brief, tables for comparisons, each
  finding labelled with who acts, any question as one `**QUESTION:**` line at
  the end. **A decision the owner made is not the agent's to drop or reverse**
  (`asking-a-person`).

**Brief them to push back.** Say what you believe and where you are unsure. Agents
caught real errors in briefs on every one of these: a stale claim about which
documents needed correcting, a wrong baseline count, and an entire premise —
that a step's deliverable should reach the Judge through git — which the owner
then rejected outright.

**Check each claim at the moment you write it into a brief, not from what you
read earlier in the job.** A fact you looked up an hour ago is a memory by
the time it reaches a brief, and it is indistinguishable from one you verified.
`grep` again. This cost twice in one run on 3 Sep 2026, and both times a child
caught what the parent had already had on screen:

- A brief said *"add its registry row, so the transition registry and the edge
  table still name the same edges"* of `STEP_EDGES`, which has no registry. The
  file's own header said so, and **that line had been in the parent's own grep
  output an hour earlier**.
- A brief said a question was *"not settled anywhere"*. A **different** child
  found it settled with three reasons in a journey document.

**A negative needs the search, not the memory.** *"Nothing decides this"* is the
claim most likely to be wrong and least likely to be checked, because there is no
file to open that disagrees with it. A registry row carrying an open question
says **that row** does not settle it — which is a different sentence. Before
writing one, grep the concept across `docs/` rather than the file you happened to
be in: a decision lives where it was argued, which is usually a journey or a
concept page and not the registry that consumes it.

## Reading what comes back

**The report is not the work.** Read the diff. An agent's summary of its own
change is written by the thing with an interest in it being right, and a summary
that says "handled" is not a change you have seen.

This is not hypothetical: a gaming flag was relayed to the owner as a Drone
cheating, and reading the diff showed both flags were false positives and the
Drone had done nothing wrong.

**Verify a claim before repeating it.** Particularly a claim about what exists.
Check the file. `grep` is cheap and a wrong claim in a merge message or an issue
survives.

**Take the corrections.** When an agent's finding contradicts your brief, it is
usually right — it has just read the code and you were working from memory.

## The merge bar

| | |
|---|---|
| **The quick self-check** | what `work-issue` step 4 names, once per branch and one heavy run at a time |
| The acceptance tests pass | where the change reaches what they read, by test name. **No failing test is merged**, a milestone's own included |
| `verify-foundations` is no worse | against a baseline off `main`, not against zero — a `missing:` the branch added blocks |
| `verify-docs` is green | where `docs/` or `operations/` changed |
| You have read the diff | not the report |

**After bringing `main` in, rerun nothing before landing.** The land turn
reruns the gate and every Check the combination hits, against the real `main`
(12 and 13 Sep 2026: a whole bar after every rebase kept two branches from
landing and made the machine unusable).

**A stacked branch rebases with `--onto` once its base has merged**, since its
history still carries the base's pre-rebase commits (14 Sep 2026: #1071 on
#1070 conflicted until `git rebase --onto origin/main <the base's old head>`).

**A wire change and a store migration take no number** (`work-issue`, step 3),
so two children conflict on nothing. Where two change one file, Fleet tells both
(`docs/concepts/fleet.md`, *Write clashes*), and nobody is held.

**What broke a merge run, each one a rerun (13–14 Sep 2026):**

- **`armada check` refusing `armada.yml`** because another job added a key the CLI did not know yet. Build a current `armada` once in a warm worktree and rerun with it; running the Check's command by hand is no substitute (a nested `pnpm -C` picked a pnpm that refused the `packageManager` pin).
- **The heavy-run hook reads command text anywhere in a Bash call**, fixture strings included, and `verify-foundations` reads `#` plus hex digits in a story title as a colour. Keep both out of fixtures.
- **A screen change merged on green checks still looked wrong** three times (panels jammed to the edge, rows that did not align, a wrapped handle). It lands only after the owner has walked it, at his widths (about 2000px, and 1284×930 with the dock open). `annotations`, step 4.

**Then give the worktree back**, at the merge (`agent-worktrees`).

**Restart Fleet when the wire moves**, and after a store migration. A running
Fleet is a stale binary the moment you merge, and a Bridge built from the merge
refuses to connect to it, because their protocol IDs differ.

**Before landing a change that moves Checks between manifests or renames a Check key, ask what each gate reads.** `armada covers` on the *installed* binary, and `.github/ci/plan.py`, must both name the new keys, or the change gates nothing and says nothing. (6 Oct 2026: the Bridge Checks moved into workspace manifests while the installed `armada` read only the root file, and CI then skipped every Bridge Check on three pull requests.) Run `printf 'apps/desktop/src/renderer/src/App.tsx\n' | armada covers` and read the job list of one PR before the next move.

**A test that pins the list of every Check across all manifests breaks on each new manifest.** Derive the expectation from the loaded manifests. (6 Oct 2026: `each_named_check_resolved_to_the_command_the_manifest_holds` went red three times; it now asserts the root's Checks literally and derives the rest.)

## What you decide, and what you bring the owner

**Decide** anything the code or an existing decision settles. Read the record
first — a decision already made and forgotten is the most common thing to get
wrong.

**Stop and ask** when two readings are both defensible, when a document's promise
would change, or when there is a cost trade that is the owner's. Unsupervised,
the safe failure is a stalled queue, not a wrong merge.

Use `AskUserQuestion` with concrete options and a recommendation first. A question
buried in prose is a question that gets missed.

**Do not file an issue for a problem the owner has not seen.** Bring it, work it
out, then file what was decided. See `armada-bug`.

## If you say you are watching something, arm something

A sentence about intending to check back is not a mechanism. When a Job or a
build is running and you have told the owner you will report on it, use `Monitor`
with a poll that emits on **every** terminal state — not just the good ones.

Silence looks identical to still-running. A filter that matches only success stays
quiet through a crash, and the owner is left believing something is being watched
when nothing is.

## What it looks like when it works

The Focus run: eight issues, five agents in flight at once at the peak, every
merge green, the milestone's acceptance test written first and passing last. Then
a real Job dispatched through the result — three Drones, one per step, seven and
a half minutes, no intervention.

The thing that made it work was not speed. It was that every question was settled
before an agent met it.
