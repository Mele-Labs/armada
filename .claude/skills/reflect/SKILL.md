---
name: reflect
description: >-
  Use when the user wants to end, wrap up, or close out a session working in this repo —
  "reflect", "wrap up", "wrap this up", "close out this session", "are we good to close
  out?", "anything left to update?", "/reflect". Drives every branch through the gate to
  merged, gives back the worktrees, fixes the prose the work just falsified, and checks
  that follow-up work has a home outside the transcript — so the session can be closed
  knowing everything already landed.
disable-model-invocation: true
---

# Reflecting before a job closes

Seven steps. The first six check that six things are true, and the seventh
writes what the job taught into the skills. A job is safe to close only when
those six are true: **every branch is
through the gate and merged**, **every worktree this job cut is gone**, **no
prose still asserts what the work just made false**, **`docs/OPEN.md` and the
milestone match reality**, **every follow-up has a durable home**, and **the
owner has been told if a running Fleet is now stale**.

**PASS means the owner closes the job and never opens it again.** Not one
action left for them — no restart, no merge, no decision, nothing to remember,
nothing "for context" that they would then have to act on. Anything left makes
the verdict **NOT READY**, however green every row looks. See *Output*.

This skill **drives the first four to completion itself** —
merge and cleanup and doc-fixes, not detection handed back as a to-do list. Then
it reports a verdict.

Announce: *"Using the reflect skill to check whether this job is ready to
close."*

## The one rule under all of it

**Code that moves falsifies prose that did not.** Everything below is a place
that prose lives. Work through them; do not reason about which are likely.

## Every edit this skill makes goes in a worktree

**This skill writes, and what it writes is not exempt.** Steps 3, 4 and 7 fix
prose, and step 7 edits this file. All of it happens on a branch in a worktree —
`git worktree add .claude/worktrees/<name> -b <branch> main` — and lands as a
pull request like any other work. Never on the checkout at `main`.

**A reflect edit is ordinary work and takes the ordinary bar**: the gate, the
diff and the review. **Editing `main` while committing from a worktree strands
the edit.** (4 Sep 2026: a doc comment fixed in the main checkout while the
commit came from a worktree, so the pull request described a change its diff did
not carry, and it cost a second pull request.) **Check `git status --porcelain`
on the main checkout before declaring anything merged**, and take every edit
inside the worktree the commit comes from.

## 1. Through the gate, merged, and `main` synced

For every branch this job committed to. The gate is what `work-issue` step 4
says each branch's change can affect, run once per branch and one heavy run at a
time. `verify-foundations` is **the delta, never the colour**: a `missing:` line
this job added is a regression.

**A warning is a gate line this table used to let through.** (2026-09-09:
`cargo build` printed `method allowance is never used` for a day and a run
reported PASS, because the `build` Check passes with warnings and
`verify-foundations` does not read dead code.) **Count the warnings and compare
the count**: `cargo build --workspace --all-targets 2>&1 | grep -c '^warning'`
against the same on `origin/main`. A warning this job added is a regression
whatever the exit code says.

**Take the baseline from `origin/main` in the same pass**, not from a brief
written hours ago or a local `main` (4 Sep 2026: a local `main` 19 commits behind
read the repository red on two refusals already fixed on the remote). `git fetch`
and compare.

**Take it in a worktree, never by stashing the main checkout**, and give that
worktree its own `CARGO_TARGET_DIR`. (2026-09-07: a `git stash` on `main` beside
another agent's writes measured six failures that were work in flight.
2026-09-13: a baseline pointed at the branch's target directory left the `ipc`
build script with the baseline's path cached, and `verify-foundations` read RED,
77 failing, on files that were all on disk.) A separate target directory costs
one cold build.

**A measurement is a timestamp, not a state.** Re-read before you report, not
just before you act; this repository moved 19 commits in two hours on 4 Sep 2026.

**Open a pull request, never merge by hand**, and land it through `land`, which
owns watching `ci`, the merge queue and requeueing. Never rebase to catch up:
merge `origin/main` in.

**Drive it to landed.** Do not stop and hand the owner a branch to land — that
is the step this skill owns. Never land past a failing gate, a real conflict,
or an unresolved question, and never land a branch this job did not produce.

**A Job's work is a pull request on a branch.** Check the branch out in a
worktree and land it through the line like any other; the push of `main`
closes the pull request as merged.

## 2. Worktrees given back

`agent-worktrees` owns giving them back, at the merge. This step finds the ones
that were missed:

```
git worktree list | grep .claude/worktrees
git branch --list "armada/*"
```

Remove merged-and-clean ones yourself. **Worktrees this job did not create
are out of scope** — other agents' work lives in the same listing, and a dirty
tree is indistinguishable from an idle one from outside. Name those to the owner
rather than removing them.

## 3. Prose the work made untrue

Grep for the claim, not for the file. A sentence asserting Armada *cannot* do a
thing is the one that rots, and it rots silently because nothing tests prose.

- **Bridge copy** — `NOT_SERVED`, absent-state sentences, handover notes.
- **Enum verbs and registry notes** — `enum-verbs.toml` carries a note per row
  saying why the verb is safe. The note outlives the reason.
- **Manifest and config comments** — `armada.yml`, `settings.toml`.
- **Doc comments naming a gap** — closing the gap is what makes them false.
- **`CLAUDE.md`** — it routes; it does not explain.
- **`docs/INDEX.md`** — the gate refuses a document that is not in it, including
  a new skill.
- **Issue bodies.** They are prose, they are public, and nothing gates them.
  Confirmed 2026-09-07: seven issues cited `docs/journeys/<n>-<name>.md` after
  the journeys were renamed without the number prefix, and #188 told whoever
  picked it up to expect three open questions on `docs/concepts/doctor.md` that
  the page had already answered and closed. Every path was still a live link in
  the issue and a dead one in the tree. **Extract the paths an issue names and
  test each for existence** — `verify-docs` will not, because an issue is not a
  document it reads.

## 4. `docs/OPEN.md`, issues, milestones

- **An open question the work answered is a stale entry**, and `verify-docs`
  fails on one. Ask it of every area you touched.
- **Close what merged**, with what was decided and why — not "done". That comment
  is what the next reader gets instead of the diff.
- **Confirm the milestone is empty** before saying it is:
  `gh issue list --milestone <name> --state open`.

## 5. Follow-up work

Scan the job for anything flagged and not finished — a deferred fix, a known
gap, an aside that deserved an issue, a `TODO` left in code.

**Filing is the owner's call and is never done on your own initiative** —
`armada-bug`'s standing rule. For each item, use `AskUserQuestion`: file it now,
file it and fix it, fix it without an issue, or drop it. A follow-up that exists
only as a sentence in the transcript counts as **not documented**.

**A fix chosen here is new work, and its agent's leftovers are not this step's
to offer.** Filing is the default option. When the owner picks a fix, land it,
and file whatever its agent reports as "found, not touched" rather than offering
it as the next round of fixes. Confirmed 2 Oct 2026: a job about rendering
markdown chained six PRs out of this step, deleting about 18,000 lines, adding a
task-sheet feature and removing keys. Each agent's leftovers became the next
question. The owner: "this is way past markdown parsing".

**The history file is not a home, and writing one does not discharge this
step.** It is a record of a job, read by nobody looking for open work. A
deferred item needs a durable home in the repository — an **open question** in
the document that blocks on it (`armada-open-questions`), or a **GitHub
issue** — and the history file then points at that.

**"Talk it through later" is not an outcome; it is where the item goes while it
waits.** A decision the owner defers is exactly what an open question is for:
one lives in the document that blocks on it and carries a slug code can cite, so
answering it surfaces everything that was waiting. Offer that as an option
alongside filing an issue, and say which fits — a call to make is a question, a
thing to do is an issue.

Confirmed 2026-08-31: two design conversations were deferred, recorded in the
history file, and reported as "recorded as decisions, not loose ends." The owner
read that and said they would get lost. They were right — `docs/OPEN.md` is what
the repository reads, and neither was in it.

## 6. The running Fleet

**A running Fleet is a stale binary the moment you merge.** If
the protocol ID moved (the wire files changed) or a store migration landed, the
job is not done until Fleet and Bridge are running the merged code — Bridge
refuses to connect to a Fleet whose protocol ID differs.

**It is a blocker, not a note.** The restart is `preview-app`'s and is not an agent's to
run on its own, so ask with `AskUserQuestion` whether
to run it now. On a yes, run it and confirm Bridge connects; then the row is ✅.
On a no, the verdict is **NOT READY** and the restart is its first sentence.

## 7. Skills, and this file

If something went wrong that a skill would have prevented, the skill did not
exist or did not say it. Write it where the next agent will hit it.

**A skill earns its lines by naming what the mistake cost.** A rule with no
incident behind it is advice, and advice is skipped.

**Waste is a mistake too, and nothing else will catch it.** A job can land
every branch green and still have spent half its context on work that bought
nothing. Read back over the job's own tool calls for it:

- a file read whole when a line range or a grep would have answered;
- the same file or command output read twice, or re-read after an edit;
- a broad search repeated with small variations instead of delegated once;
- a subagent dispatched for a lookup one command answers, or several where one
  would have done;
- a cold build forced by a shared or deleted target directory;
- a whole suite rerun after the agent already ran it and before the merge line
  runs it again (`annotations`, *Verify what comes back*);
- a brief that named files without lines, or five skills where two would do
  (`work-issue`, *A brief points*);
- a fact in a skill or doc that the job rediscovered from code because
  nothing routed it there.

For each, propose the edit that would have avoided it: a line in the skill that
was loaded, a routing row in `CLAUDE.md`, a pointer in a doc. Name what it cost
in calls or in output read, the same bar as any other rule here. Land the edits
on this skill's worktree. A habit no file could have prevented goes in the
history file's one line, not in a skill.

## Output

| Check | Status | Detail |
|---|---|---|
| Gate passed, merged, `main` synced | ✅ / ❌ | branches; merged or blocked and why; test and foundations delta |
| Worktrees given back | ✅ / ❌ | removed; which remain and whose they are |
| Prose reconciled | ✅ / N/A | what was corrected; N/A only if no code changed |
| `OPEN.md`, issues, milestones | ✅ / N/A | entries removed, issues closed, milestone confirmed empty |
| Follow-up documented | ✅ / ❌ | where each item now lives, or what is missing |
| Fleet running the merged code | ✅ / ❌ / N/A | protocol ID, migrations; restarted, or why not |

**PASS only when the owner has nothing to do.** Every row ✅ or N/A, and no row,
sentence or footnote anywhere in the message asks anything of them. **A row with
an action for the owner is ❌**, whatever surface would remind them of it — a
restart they must run is not a ✅ with "Owner: you" beside it.

**Before the verdict, clear what is left.** For each thing that would need the
owner: do it yourself if it is this skill's to do; if it needs their say-so, ask
with `AskUserQuestion` and do it on a yes. Only what they decline, or what
genuinely cannot be done from here, is left — and then the verdict is NOT READY.

**The shape of the message follows the verdict:**

- **PASS** — the first sentence is "Ready to close." Then the table, and nothing
  after it: no "one more thing", no "you'll need to", no closing note.
- **NOT READY** — the first sentence is "Not ready to close," and the next lines
  are the owner's actions as a numbered list, each one a thing to type or press.
  Then the table.

**Context that asks nothing may sit in a row's Detail** — another agent's
worktree left in place, a count — but never as a sentence of its own after the
table, where it reads as the thing still open.

(2026-09-08 and 2026-09-12: runs reported PASS and then asked the owner to
restart Fleet and Bridge, with the restart row marked ✅ and "Owner: you". His
words: green checks followed by one more thing means it was never ready to
close.) **An item naming its surface is still an item.** The surface decides
where a follow-up lives, not whether the job is done.

## Gotchas

- **A pipe masks a failed merge.** (2026-08-31: `git merge --ff-only X 2>&1 | tail
  -2` exits with `tail`'s status, so an `&&` chain deleted the branch after a
  failed merge.) **Never pipe a command whose exit code an `&&` depends on.**
- **`set -e` does not stop on a failed `test … && echo`.** (2026-09-14: a merge guard written that
  way failed silently and `gh pr merge` ran on the next line.) A guard a merge
  depends on is `test … || { echo why; exit 1; }`, or an `if`.
- **Parallel agents collide in vocabulary, not only in files.** Confirmed
  2026-08-31: two agents independently added a type named `Keeping` on one
  evening — different subjects, one name, caught only at the merge build. Tell
  each agent the nouns the others are introducing.
- **`verify-foundations` reads one higher inside a worktree.**
  `docs/v1-decommission.md` names `.claude/worktrees/`, which does not exist
  inside one. That line is an artifact, not a regression — compare `FAIL` lines,
  not the count.
- **The report is not the work.** An agent's summary is written by the thing with
  an interest in it being right. Read the diff. A gaming flag was once relayed to
  the owner as a Drone cheating; the diff showed both flags were false positives.
- **Abandoned work is not unlanded work.** Confirmed 2026-08-31: a `stash@{0}`
  dated three days earlier was described to the owner as a whole `forget_job`
  operation existing nowhere else, and dropping it as irreversible — on the
  strength of its date and its contents. `forget_job` was already on `main`,
  landed after the stash was taken. **Grep the tree for what the artifact
  contains before saying what losing it would cost**; a stash, an old branch or
  a detached worktree is a claim about the past, and the present is what decides
  whether it matters. The wrong framing had already reached a decision — the
  owner chose "leave it" against a loss that could not happen.
- **A branch left unmerged reads exactly like a merged one** from the job
  transcript. This skill found one that had been reported as "verified and
  waiting" and then never merged. Check `git branch --list`, not your memory.

## Verification

Before ending: state **PASS** or **FAIL** plainly — PASS only if what this skill
exists to produce actually landed **and the owner has nothing left to do**, not
"I ran the steps." A history file whose Follow-ups table has an owner row is a
FAIL.

Then write `.claude/history/<UTC-timestamp>-reflect.md`:

```
---
skill: reflect
verdict: PASS | FAIL
date: <UTC ISO-8601>
milestone: <name>    # if applicable, omit otherwise
---

<one line on what happened>

## Follow-ups
| Action | Owner | Detail |
|---|---|---|
```

Omit the table entirely if nothing is outstanding. Commit it with the job's
own changes, not as a separate step.
