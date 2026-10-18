---
name: work-issue
description: How to work one GitHub issue — worktree, plan, implement, self-check, commit, land. Load before starting any issue, and before dispatching agents at several.
---

# Working one issue

**This is the `bug` workflow, run by hand.** A Job works in its own leased
slot, runs `armada check` when it needs to, and goes up as a pull request. Here
green work is previewed, adopted and goes up as a pull request without waiting
(step 6).

`milestone-step` owns how to read an issue, what to check it against, and how to
close it. **This skill owns where the work happens and how it lands** — the two
things that skill does not say, and the two that went wrong.

## Why this exists

On 2026-08-28 ten issues were worked in one working tree on `main`. At the end
none of it could be committed per issue: `gate.rs` carried two, `work_product.rs`
carried two, `routes.rs` carried three, and splitting by hunk would have produced
commits that did not compile. A proposer change that had been sitting
uncommitted since before the job was swept into somebody else's commit by a
single `git add crates/fleet`.

**Nothing was lost and the tests were green.** The cost was entirely in the
history — which is to say, entirely in what the next person can reconstruct.

A Job in a leased slot never has this problem, because Fleet decides where it
works and it works on one thing. The rule is not "be careful"; it is that the
isolation is not the agent's to arrange.

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
touched by hand.

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

**A migration takes no number: it has a name.** Add one file,
`crates/store/migrations/<UTC yyyymmddThhmmZ>-<module.slug>.sql`, and nothing
collides: no two branches touch the same file, and order between them does not
matter. It is additive unless a `-- breaking` comment line says otherwise, and
a test refuses an additive one that drops, renames or rewrites. A breaking one is a decision: ask first. A branch that previews
with a breaking migration `main` lacks is refused by `scripts/restart --from`.
`docs/practices/store-migrations.md`.

**A wire change carries no number.** The protocol ID is a hash of the wire
files, so changing `crates/ipc`, `crates/ipc/operations/`, the route table or
`packages/protocol` moves it and nothing is bumped, declared or renumbered.
`docs/practices/protocol.md` says what it covers.

### 4. Test

`milestone-step` step 4, and it is not optional because the change looks small.

**A quick self-check, once: build, typecheck, and the tests of what you
changed.** The full run is `ci` on the pull request: it runs the Checks the
merged tree hits, and a red there comes back to you (step 6).
`docs/practices/ci.md`.

**Never every Check the branch touches, and never twice.** The load rules are
`session-habits`, *Load*: no load to reproduce a flake, a brief names the tests
the change touches, whole suites are CI's. Confirmed 13 Sep 2026: every row run
on every branch and again after every rebase made the machine unusable.

| You changed | Run |
|---|---|
| A crate under `crates/` | `git diff --name-only main \| armada check test --changed` runs the test modules you touched, in their crates only; `armada check test a b c` runs only the crates declaring those tests, in one build (`session-habits`, *Load*). The whole `armada check test` is CI's |
| Any Rust | `cargo build --workspace --all-targets 2>&1 \| grep -c '^warning'` once — **the same count as `main`**, whatever the exit code |
| What a milestone's claim reads | `armada check acceptance` |
| `apps/` or `packages/` | `printf '<paths>\n' \| armada covers` names the Checks your change reaches, as keys, and `armada check <key>` runs one: `packages/screens:typecheck`, `packages/components:components_test`. **A story is in `packages/components:components_test`**, a screen's test through `App` is run by hand (`src/renderer/src/mock/*.test.tsx`, `vitest run --project 'smoke*'`; no Check), and `apps/desktop/unit:desktop_test` is main and the renderer's own modules, and `packages/screens:screens_test` has only `packages/screens`' own `.test.ts` and `.test.tsx`. `brand`, `protocol`, `shell`, `tokens` and `icons` have no manifest: `armada check typecheck` is theirs |
| `docs/`, or `crates/ipc/operations/` | `cargo xtask verify-docs` |
| Anything | `cargo xtask verify-foundations` once, before landing — **no worse than the baseline you took off `main`.** Read what each line names; never chase a colour |

**Through `armada check`, never `vitest` or `nextest` bare.** A Check waits
for one of the machine's Check slots and hands its runner `${width}`; a bare run
takes neither. Confirmed 30 Sep 2026: jobs running suites bare beside a
Fleet took the load to 19 on 18 cores, and a different test timed out each run.
`docs/concepts/manifest.md`, *How many Checks run at once*.

**A heavy command the table runs bare, prefix with `taskpolicy -c utility`**,
for example `taskpolicy -c utility cargo build --workspace --all-targets`.
`armada check` already lowers its Checks beneath the merge line and Bridge, and
`nice -n 10` was measured to change nothing on this machine.
`docs/concepts/manifest.md`, *At what priority a Check runs*.

**One heavy run at a time, across every job on the machine**, your own
background runs included.

**A wait loop that greps for the run matches itself.** Confirmed 17 Sep 2026: a
job queued its gate behind `while pgrep -f 'vitest|cargo xtask' ...; do
sleep 20; done`, and `pgrep -f` matched the shell running that very line. It
waited on itself twice and cost an hour, with the owner asking what it was
waiting for. Excluding the loop's own pid is not enough — the parent shell
carries the same command line. Wait on the thing itself (`wait`, a pid file, or
the tool's own background handle), or put the pattern where no shell repeats
it, and read a wait that has lasted longer than the run would have as a bug in
the wait.

**Warnings were missing from this table, and a merge paid for it.**
Confirmed 12 Sep 2026: #843 ran every row above, merged green, and left `main`
with an unused import.

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

### 6. Preview it, adopt it, then open the pull request

**The owner's order is: propose, implement, walk if it is visual, preview and
adopt, land.** He uses Armada as its end user, so he runs the change before it
lands. Once the work is committed and step 4's self-check passes, do these in
order and ask nothing between them:

1. `scripts/preview` merges the branch into the preview with every other branch
   in flight. Load `preview-app` first.
2. If the change reaches Fleet or Bridge, `scripts/preview --restart --adopt`
   moves his Fleet and Bridge onto it. Say in one line what it will do, since his
   permission prompt is the confirmation. A change that is only docs or scripts
   needs no restart. A restart already in progress refuses a second: say so and
   go on.
3. Push the branch and open the pull request, straight away. **Do not wait for
   him to try the preview, and never ask "land it?"** He files what he finds as
   separate work or comes back to you, and `ci` is the guard.

**A visual change is the exception: it walks before it previews, and the pull
request waits for his OK on the walk.** `annotations`, step 4, has the rule, and
`docs/practices/running-locally.md` *Walks* has the walk.

```
git push -u origin <branch>
gh pr create --base main
```

The description follows `commit-message`: say what the diff cannot, and end with
"Merge with Create a merge commit". `land` owns watching `ci`, merging and
requeueing, and the worktree going back; `docs/practices/ci.md` owns the gate.
You watch `ci` so he does not have to: do not report the work done while it is
running, and a red one comes back to you.

**Where a file sits near a threshold, leave headroom.** A branch and `main` can
each sit under a limit that the two together cross, and `ci` measures the merged
tree. Confirmed 2026-09-12: #730 passed `verify-foundations` at 898 lines in
one screen file; `main` grew the same file by seven while the branch was open,
and the merge landed it at 905, over the 900-line rule.

**Bring a moved `main` in by merging it, never by rebasing.** One pass meets
every conflict at once, the commits already reviewed keep their ids, and a
plain commit carries the result, which is the same reason Fleet stopped
rebasing in #1131.

Say in the commit message and the pull request description what you would want
looked at closely. Then `milestone-step` steps 5, 6 and 7: close the issue with
what contradicted the plan, give every open item an owner, report. **The report
names the pull request and says whether `ci` is green.**

## Dispatching several agents at once

Briefing is `orchestrate-milestone`, *The brief*, and `session-habits`,
*Briefing agents*. What follows is only what dispatching adds.

**Write scope is surfaced, never serialised, and nothing holds an agent.**
Fleet tells both agents when their changes would conflict (`docs/concepts/fleet.md`,
*Write clashes*), and there is no lease (#47, closed). So disjoint scopes are
the dispatcher's to arrange. The split that worked was by crate boundary and by
side of the seam:

| agent | scope |
|---|---|
| one | `crates/config`, `crates/adapters` |
| two | `crates/ipc`, `crates/api`, `crates/fleet` |
| three | `apps/desktop`, `packages` |

Then say so in the prompt — *"another agent is working in X in parallel; do not
touch it; if your change needs one, stop and report it rather than making it."*
Every agent given that sentence obeyed it.

**An interim "has not reported yet" notice is not a report.** On 4 Oct 2026 a
finished agent's report sat in its transcript for 12 hours while the owner was
told it was still running. When the notice repeats, read the agent's branch,
`git status` and the last lines of its transcript before waiting again.

**Two issues that land in one file do not run in parallel.** They run in
sequence, and the one that decides the arrangement runs first.

**An agent that stalls takes its uncommitted work with it** (17 Sep 2026: two
`bridge-engineer` agents ended on "no progress for 600s" with nothing committed
and no worktree left). After a second stall on the same brief, do the work
inline rather than dispatching a third.

## Give the worktree back

Whoever merges does it, at the merge: `agent-worktrees`, *Release at the merge*
and *Give it back when the branch merges*.
