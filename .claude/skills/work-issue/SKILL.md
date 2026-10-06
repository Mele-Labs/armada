---
name: work-issue
description: How to work one GitHub issue the way a Job works a workflow — worktree, plan, implement, self-check, commit, land. Load before starting any issue, and before dispatching agents at several.
---

# Working one issue

**This is the `bug` workflow, run by hand.** Armada dispatches a Drone into its
own worktree, gates each step, and holds the work at `awaiting_review` before it
lands. When Fleet is not the one dispatching, that shape still applies up to the
hold, and this skill is it. Here green work lands without waiting (step 6).

`milestone-step` owns how to read an issue, what to check it against, and how to
close it. **This skill owns where the work happens and how it lands** — the two
things that skill does not say, and the two that went wrong.

## Why this exists

On 2026-08-28 ten issues were worked in one working tree on `main`. At the end
none of it could be committed per issue: `gate.rs` carried two, `work_product.rs`
carried two, `routes.rs` carried three, and splitting by hunk would have produced
commits that did not compile. A Job-proposer change that had been sitting
uncommitted since before the session was swept into somebody else's commit by a
single `git add crates/fleet`.

**Nothing was lost and the tests were green.** The cost was entirely in the
history — which is to say, entirely in what the next person can reconstruct.

A Drone never has this problem, because Fleet decides where it works and it
works on one thing. The rule is not "be careful"; it is that the isolation is
not the agent's to arrange.

## The loop

### 1. Worktree, before reading the issue

**Branch and worktree first, every time, including for a change you are sure is
one line.** The judgement about how big a change is comes after you have read the
code, and by then the tree is already dirty.

**Lease a slot: `armada worktree lease <branch>`**, and work at the path it
prints. A slot keeps the last lease's `target/`, so the first build is
incremental, and the pool is the machine's cap — a lease waits while every slot
is held. It fetches and cuts the branch from the base itself.
`agent-worktrees` has the rest. `.armada/worktrees/` is Fleet's and is never
touched by hand — a Drone is working in there.

**Only where `armada` does not know the verb** — the installed binary predates
it, and `scripts/restart` is the fix — fall back to `EnterWorktree`, under
`.claude/worktrees/`, named for the issue. **Then fetch, and fast-forward onto
what `main` is now.** `EnterWorktree` cuts from the local `origin/main` ref,
which is only as fresh as the last fetch.
Confirmed 14 Sep 2026: the #1001 worktree came up at a commit from before #999
merged, missing the code the step was built on. `git fetch origin main && git
merge --ff-only origin/main` before reading anything.

**Never work on `main`.** Not for a doc fix, not for a comment.

**One issue per worktree.** Two issues in one tree is the defect above, arriving
early.

### 2. Plan

`milestone-step` steps 1, 2 and 2.5 are the plan: read the issue in full, read
what it disagrees with, read the registry before minting anything. Do not repeat
them here; load that skill.

One addition, from tonight. **Check the source the issue points you at before
trusting it.** Two Jobs failed against #118 because it said to port from
`crates/core-model/domain/workflow-samples/`, and those samples disagree with
the parser in three ways — `config/src/judge.rs` had said so in its own module
docs and nothing had acted on it. An issue is a claim like any other.

### 3. Implement

`milestone-step` step 3. One issue. Finish it, and stop.

**Declare a need before choosing a migration number or a protocol minor, and use
the number the answer gives.** Never take "the next one" from `main`: another
branch takes the same one, and whichever lands second renumbers, rewrites every
mention and runs the Checks again (#1059).

```sh
armada need crates/store/src/migrations.rs "a new migration"   # says who is ahead, and what each took
armada need --took crates/store/src/migrations.rs "V96"        # once chosen, after theirs
armada need protocol-version.toml "a minor"
```

A need is a path and what is needed there, in your words, and the first to
declare goes first. If something is ahead of you, pick the value after what it
took. `armada land` then holds your branch until every need ahead of yours has
landed, so you land in order and nothing is renumbered. Declaring is a no-op the
second time. **If you had already written a number when you declared**, it says
so: search comments and docs for the old number and change every mention.
`armada need --release <path>` gives one back; a branch deleted locally gives its
needs back by itself, and a need that stalls is given back by a person, since
nothing expires. `armada need --status` lists every need by path.

### 4. Test

`milestone-step` step 4, and it is not optional because the change looks small.

**A quick self-check, once: build, typecheck, and the tests of what you
changed.** The merge line is the full run: `scripts/land` runs the Checks the
branch hits, and those `main` brought in when it moved, against the real
`main`, and a red there comes back to you (step 6).
`docs/practices/running-locally.md`, *Landing a branch*.

**Never every Check the branch touches, and never twice.** Confirmed 13 Sep
2026: a session ran every row of the old table on every branch and again after
every rebase, beside another session's test run, and the owner's machine was
unusable.

**Never make load to reproduce a flake.** No CPU or disk burners, no stress
loop beside a full suite, nothing left running in the background. One
`--stress-count` run of the one test, and if it will not fail, fix what the code
shows. Confirmed 2 Oct 2026: a brief asked for load 50+, the agent started
endless 2 GB `dd` loops and a 30-worker burner, and the owner's machine froze
until he killed them by hand. There is no other machine.

| You changed | Run |
|---|---|
| A crate under `crates/` | `armada check test`, and `armada check test <test>` for one test while you work |
| Any Rust | `cargo fmt --all --check`, and `cargo build --workspace --all-targets 2>&1 \| grep -c '^warning'` once — **the same count as `main`**, whatever the exit code |
| What a milestone's claim reads | `armada check acceptance` |
| `apps/` or `packages/` | `armada check typecheck`, and `armada check <name>` for the package you changed. **A story is in `components_test`**, a screen's test through `App` in `desktop_test` (`src/renderer/src/mock/*.test.tsx`), and `screens_test` has only `packages/screens`' own `.test.ts` and `.test.tsx` |
| `docs/`, or `crates/ipc/operations.toml` | `cargo xtask verify-docs` |
| Anything | `cargo xtask verify-foundations` once, before landing — **no worse than the baseline you took off `main`.** Read what each line names; never chase a colour |

**Through `armada check`, never `vitest` or `nextest` bare.** A Check waits
for one of the machine's Check slots and hands its runner `${width}`; a bare run
takes neither. Confirmed 30 Sep 2026: sessions running suites bare beside a
Fleet took the load to 19 on 18 cores, and a different test timed out each run.
`docs/concepts/manifest.md`, *How many Checks run at once*.

**A heavy command the table runs bare, prefix with `taskpolicy -c utility`**,
for example `taskpolicy -c utility cargo build --workspace --all-targets`.
`armada check` already lowers its Checks beneath the merge line and Bridge, and
`nice -n 10` was measured to change nothing on this machine.
`docs/concepts/manifest.md`, *At what priority a Check runs*.

**One heavy run at a time, across every session on the machine.** Never start a
build or a test suite while another is running, your own background runs
included.

**A wait loop that greps for the run matches itself.** Confirmed 17 Sep 2026: a
session queued its gate behind `while pgrep -f 'vitest|cargo xtask' ...; do
sleep 20; done`, and `pgrep -f` matched the shell running that very line. It
waited on itself twice and cost an hour, with the owner asking what it was
waiting for. Excluding the loop's own pid is not enough — the parent shell
carries the same command line. Wait on the thing itself (`wait`, a pid file, or
the tool's own background handle), or put the pattern where no shell repeats
it, and read a wait that has lasted longer than the run would have as a bug in
the wait.

**Warnings and Format were missing from this table, and a merge paid for it.**
Confirmed 12 Sep 2026: #843 ran every row above, merged green, and left `main`
with ten unformatted hunks and an unused import. `format` is a Check in
`armada.yml`, so the next Job cut from `main` would have failed on work that
was not its own.

**Verify it yourself rather than on a report.** An agent's claim of green has
been wrong here.

**When a `desktop_test` failure may not be yours, compare alternately.** Run
the branch and `main` in alternating runs, several of each, never a block of runs
on one tree: machine load drifts across a block and makes one tree look clean and
the other broken. Confirmed 17 Sep 2026: runs alternated between `main` and a branch
failed 3 of 10 on each, where blocks had blamed the branch.

**A filtered Check that prints nothing did not pass.** `armada check` reads
`armada.yml` from the working directory. Confirmed 14 Sep 2026 on #1117: a
typecheck run from `packages/components` could not read the file, and the
`grep` for `passed|failed` around it printed nothing at all. Run Checks from
the repository root, and read a silence as nothing having run.

**That is a silence after it exits. Mid-run, `armada check` writes nothing
until the suite ends**, so an empty log is a suite still running, not one
blocked; a slot wait says so on stderr. Confirmed 1 Oct 2026: six runs of
`armada check test` beside desktop_test logged 0 bytes, and they were read
as blocked. That cost two probes, a question to the owner and a read of the
slot code before the test binaries' build times showed they had been running
all along.

### 5. Commit

Read `.claude/skills/commit-message/SKILL.md`. Say what the diff cannot.

**Commit at each step, not at the end.** A worktree that has been running for an
hour with nothing committed is the tree this skill exists to prevent, one scope
smaller.

**`git add <path>` takes what is under that path, including files you did not
write.** Stage by name, or read `git status` first and know every entry. That is
how someone else's uncommitted work ended up inside a commit about something
else.

### 6. Land it, and let it be read afterwards

**Green work lands without asking.** Once the work is committed and step 4's
self-check passes, run `scripts/land` straight away. Pushing the branch and
opening a PR are optional: open one when there is something you want the owner
to read before it lands, and the line closes it as merged when it lands. The
merge line is the guard, and it pushes `main` itself. The owner reviews
afterwards by reading what landed, for example
`git log --merges --first-parent main --since=yesterday`; each merge carries a
`Landed-from:` trailer naming its branch.

**A visual change is the exception: it waits for the owner's look.** It ships
with a walk, he opens its link on a mock served from your worktree, and it lands
only on his OK. `annotations`, step 4, has the rule, and
`docs/practices/running-locally.md` *Walks* has the walk.

**`scripts/land` is how it merges**: never `gh pr merge`, never a push to
`main`, and a hook refuses both.

```
scripts/land preflight   # once step 4's self-check passes, on a clean tree
scripts/land             # joins the line and returns at once
scripts/land --status    # poll in short foreground calls
```

**Exit 3 is still going; exit 10 is still going and already failing.** A Check
of your branch failed that `main` is green for, and the turn is running its
other Checks. The status names the Check and its log. Read the log and start the
fix now; do not push the branch, since a push is dropped as stale. The turn
still ends red, and that is the verdict.

**A red turn comes back to you.** Read the logs it names, fix on the branch,
then preflight and land again.

**Read the gate's verdict before you join the line, and never chain `land`
after it with `;`.** Confirmed 2 Oct 2026: `verify-foundations …; git push &&
… scripts/land` queued a branch the gate had just called RED on a vendor
literal. That took two re-preflights to replace it. Make the gate a guard:
`if`, or `|| exit`.

**Watch a branch with `armada land --status <branch>` and its `<branch>:`
line.** The bare `scripts/land --status`, run outside that branch's worktree,
lists the line as numbered entries. A watcher grepping `^<branch>` there finds
nothing. Confirmed 2 Oct 2026: one reported three queued branches as out of the
line, and two more expired silently while all three were still gating.

**The line exists because a branch's Checks measure a `main` that moves.** The
gate reads the merged tree, and a branch and `main` can each sit under a limit
that the two together cross. Confirmed 2026-09-12: #730 passed
`verify-foundations` at 898 lines in `packages/screens/src/JobDetail.tsx`;
`main` grew the same file by seven while the branch was open, and the rebase
landed it at 905 — over the 900-line rule. It merged on its own green
measurement and left `main` red, with nothing to raise it until somebody ran
the gate by hand. **`land` is what reruns that**: it merges `main` in, runs the
Checks the combination hits and merges only on what it measured. Where a file
is near a threshold, leave headroom rather than sitting on it.

**Bring a moved `main` in by merging it, never by rebasing.** One pass meets
every conflict at once, the commits already reviewed keep their ids, and a
plain commit carries the result — which is the same reason Fleet stopped rebasing
in #1131. `docs/capabilities/merge-line.md` is the design, and
`docs/practices/running-locally.md` has what each exit code means.

Say in the commit message what you would want looked at closely, because with
no PR that is where the owner reads it. Then `milestone-step` steps 5, 6 and 7:
close the issue with what contradicted the plan, give every open item an owner,
report. **The report names the merge commit the branch landed as.**

## Dispatching several agents at once

**Write scope is reserved by hand, because #47 is not built.** Nothing stops two
agents editing the same file, and the second one wins silently.

Before launching, write down each agent's scope and check the sets are disjoint.
The split that worked was by crate boundary and by side of the seam:

| agent | scope |
|---|---|
| one | `crates/config`, `crates/adapters` |
| two | `crates/ipc`, `crates/api`, `crates/fleet` |
| three | `apps/desktop`, `packages` |

Then say so in the prompt — *"another agent is working in X in parallel; do not
touch it; if your change needs one, stop and report it rather than making it."*
Every agent given that sentence obeyed it.

**Two issues that both land in `JobDetail.tsx` do not run in parallel.** They run
in sequence, and the one that decides the arrangement runs first.

**Pass the owner's rules down.** Report bottom line first, be brief, no
unnecessary caveats, tables over paragraphs for anything comparative, label every
finding and table row with who acts on it, and surface any question as a single
`**QUESTION:**` line at the end rather than burying it in prose. **A decision
the owner made is not the agent's to drop or reverse**, even with evidence
against it: it stops, and that is its `**QUESTION:**`. See `asking-a-person`.

**Tell every agent to wait for its own runs in the foreground.** An agent is
woken only by a message, never by its background build finishing. Confirmed
14 Sep 2026: the Fleet agent for #1105 ended its turn three times with nextest or
`cargo build` still running, and sat idle with nothing committed until it was
watched and woken by hand each time. Put *"run heavy commands in the foreground
and wait; never end your turn while a run is in the background"* in the brief.

**A brief points, so the agent reads less.** Give each file as `path:line`, name
at most two skills (the one the work needs, plus `commit-message`), and put
*"read by range — grep first, never read a file over 300 lines whole; tail test
output"* in the brief. An agent starts cold, and what it reads to find its place
is most of what it costs. Confirmed 2 Oct 2026: four Studio agents briefed with
five skills each and file names without lines used 61k–551k tokens apiece, and
the session's own `/context` put reads at 55% of everything it took in.

**Put the numbers and the Checks in the brief, because an agent never loads this
skill.** Step 3's `armada need` reaches nobody who is dispatched: the brief must
say *"before choosing a protocol minor or a store migration number, run `armada
need`"*, or each agent takes the next one from `main`. Confirmed 4 to 5 Oct
2026: five agents in one session took 23.23, 23.24, 23.33 and store V104 and
V109 that another branch had taken, and the merge line sent each back to be
renumbered by hand, five full requeues. The brief must also name what the line
will run on the files the agent touched: *"run every test module that references
what you changed, then `armada check typecheck`, `cargo fmt --all --check` and
`cargo xtask verify-foundations`; a new operation needs `tests::served`."* The
same session lost four more turns to a route-table test, a fixture that no
longer typechecked, a rustfmt line and a story-title rule, each red on the line
and green in the agent's report.

**Pick the model by the work.** Mechanical work goes to `model: "sonnet"`: a
merge or a conflict, a doc or registry edit, a fix whose cause the brief already
names. Design, visual work and anything that crosses the seam stays on the
default model. The owner asked for this on 2 Oct 2026, for the same reason as
the rule above.

**An agent that stalls takes its uncommitted work with it.** Confirmed 17 Sep
2026: two `bridge-engineer` agents in a row, each briefed to move the Board's
Storybook tests onto App, ended on *"no progress for 600s"*. Neither had
committed, and the worktree was gone afterwards, so nothing survived. The same
work then landed in one pass in the main session. Put *"commit and push after
each piece that passes"* in the brief. After a second stall on the same brief, do
the work inline rather than dispatching a third.

## Give the worktree back

**The merge is the moment.** A branch that is in `main` has a worktree holding
nothing `main` does not, and nothing reclaims it on its own — `armada clean`
gives back a *Job's* worktrees, not an *agent's*.

A leased slot is released, never removed — it refuses while anything in it is
uncommitted or unlanded, so it checks for you:

```
armada worktree release <path>
git branch -D <branch>
```

A tree cut by `EnterWorktree` or `isolation: "worktree"` is removed instead:

```
git worktree remove --force <path>
git branch -D <branch>
```

**Whoever merges does this**, in the same breath as the merge. Left undone it is
invisible until a disk fills: seventy-four worktrees and 220 GB, three agents
killed mid-run at zero bytes free, each with uncommitted work.

**Never remove one without checking it is clean.** An agent that has written
files and not committed sits on a branch with no commits ahead of `main` — so it
reads as merged, and removing it destroys the only copy. `git status --porcelain`
is the check that catches it.

`agent-worktrees` has the rest: what to keep, how to sweep safely when it has
already got away, and why sharing one build directory between worktrees was tried
and is not the answer.

## What this skill does not do

**It does not replace dispatching a Job.** When Fleet can run the work, run the
work — hand-landing what a Job should have done hides every gap in the fleet, and
that is how a milestone gets marked complete while nothing can reach it. Use this
when Fleet cannot: for changes to Fleet itself, when a Drone has failed at
something twice, or when the owner says to.
