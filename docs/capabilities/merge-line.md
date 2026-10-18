---
capability: merge-line
issue: 1315
milestone: Throughput
---

# Merges take turns, and each one is checked against the main it lands on

> Work goes through pull requests and the `ci` check. `armada land` and
> `scripts/land` are retired, and Fleet no longer reads the files an earlier run
> left under `.git/armada-land/`: the queue and outcome lists below are always
> empty on the wire.

## What replaces each part

GitHub's pull requests and the `checks` workflow land this repository's code
([the decision](../../.claude/decisions/2026-10-06-ci-and-pull-requests-replace-the-merge-line.md)).
**The rest of this page describes the line being retired**, and what stays of
it: main going red, and Bridge's panel. It is kept for the reasoning.

| The line's part | What replaces it |
|---|---|
| Queue and turn | GitHub's merge queue |
| Rerunning what `main` moved under | The `plan` job, through `armada covers`, as in *Choosing what reruns* |
| `verify-foundations` on every turn | A `foundations` job, on the branch `ci-foundations` and not merged. Until it lands CI does not run it |
| Batching | The merge queue's own grouping |
| Bridge's panel | A thin view of every open pull request and how Fleet knows them to connect, by Job links. Not built |

Two branches can each pass every Check, merge a minute apart, and leave `main`
red. Each one's Checks ran against a `main` that had moved by the time it
merged.

**Merges wait in line; work does not.** Branches are worked in parallel, and
a write clash is a warning ([Fleet](../concepts/fleet.md),
*Write clashes*). Only the step onto `main` takes turns.

`armada land` was the local stand-in, for agents working this repository
outside Fleet, and is gone. It was built so each part has a named home in
Fleet, listed under *Where each part goes in Fleet*.

## What holds

| Rule | Held by |
|---|---|
| `verify-foundations` runs on every turn | A throwaway worktree at the commit being merged |
| The Checks the branch hits run even when `main` has not moved | `armada covers`, over the branch's own paths |
| When it has moved, `main` is merged in first | A throwaway detached worktree at the branch head |
| A Check reruns when its `when:` matches either side | `armada covers`, over both sets of paths |
| `verify-foundations` reruns, read against `main` | Only a failing line `main` lacks is red |
| A stale generated file never fails a turn | Its generators run on the candidate first, and their commit lands |
| What lands is exactly what was gated | A `--no-ff` merge commit over the candidate's own tree, made by the runner |
| Nothing lands on a `main` it was not gated against | The runner's own push, never forced; a refusal gates again |
| A gate does not run on once `main` has moved | A look at `origin/main` before the first Check and after each; a move stops the gate there and gates again |
| A failed Check is told while the turn runs | Right after it fails, with a Check still to run, `main` is asked about it; green is `--status` exit 10, naming the Check and its log |
| A branch needs no push and no pull request | The runner reads the branch from this clone |
| An agent previews green work, then lands it without asking the owner | The agent's own brief, `work-issue` step 6; the owner reads what landed afterwards |

## One turn

```
scripts/land preflight    clean tree, commits ahead of main -> stamp HEAD^{tree}, note any open PR
scripts/land              stamp matches -> queue entry -> runner started if none -> returns
                                                  |
runner (holds the turn lock) ----------------------+
  take the first <size> entries, in place order: one batch (size: 1 to ARMADA_LAND_BATCH, 8)
  each member: entry still queued, same nonce?  -- no -> dropped (withdrawn, or resubmitted)
  each member: local branch still at the queued head?  -- no -> outcome stopped
  fetch main; a member's head already in main? -- yes -> outcome landed (a killed runner's push)
  stack = main; for each member, in order:
    worktree at <head>; stack not in it? -- yes -> git merge <stack>
        |            | conflict only in generated files -> regenerate, commit
        |            | other conflict, and with main alone too -> that member: outcome conflict
        |            |                                            (keeps its place); the rest go on
        |            | other conflict, only with a member before it -> split the batch
    stack = commit-tree <candidate>^{tree} -p <stack> -p <candidate>   (Landed-from: <branch>)
  seed: cp -c the build directories into a worktree at the stack
  regenerate: each generator -> anything changed? commit it | one failed -> red
  verify-foundations: new FAIL / missing: lines vs main's own run
        | new lines, every path each names touched by one member -> those members red; regate the rest
        | new lines, otherwise -> red, no Check runs (several members: split)
  covers(each branch's paths, + what landed since it was cut) -> setup -> armada check each --changed
        | main moved (looked at before setup and after each Check) -> stop, skip the rest, gate again
        |   (a Check already running is let finish; the same bounded rounds as a refused push)
        | red, one member -> outcome red, nothing pushed
        | red, several    -> split the batch in half, first half first, and take each
  git push origin <top>:main  -- not a fast-forward -> gate again (bounded rounds)
        | pushed -> each member: outcome landed, naming its own merge
  each member: PR open? wait for GitHub to read it merged, else gh pr close --comment <merge>
  each member: remote branch all landed? -> git push origin --delete <branch>; print cleanup commands
```

## A failed Check is told at once

- **`--status` exits 10 as soon as a failed Check has a green answer from `main`.** The turn is still going, so 3 would say "wait"; the agent can read the log and start the fix. The runner asks `main` about a Check the moment it fails, when a Check still follows it, through `checks_on_the_base`. The outcome carries `own_failures` (the branch's, `main` green) beside `already` (`main`'s). `--status` prints `<Check> failed (log <path>); main is green for it. The turn is still running its other Checks; do not push this branch, a push is dropped as stale.` and exits 10 while the state is `gating`. Any other state keeps its own code.
- **A Check red on `main` too is `main`'s and keeps exit 3**; the turn's end says so as before. Nothing else about a turn changes: no rerun, and the verdicts and what lands are the same. A timeout is not a failure here and is told at the end.
- **A batch cannot name the one at fault.** While the group has more than one member, every member's line says it is a heads-up for the whole batch and that the split names the branch at fault. Each member's verdict follows at the end, as before.
- **The cost is a Check run on `main` while later Checks wait.** The answer is cached per Check and `main` commit, so it is paid once, and the end of the turn asks only about Checks not already answered. A timeout on `main` is not cached but is kept for the turn, so it is not run twice either.

## Batching

**A turn takes up to eight waiting branches and gates them once.** One gate per branch made the wait grow with every agent landing. `ARMADA_LAND_BATCH` sets that ceiling; `1` is the line as it was.

- **The size moves with how turns go.** Decided 2 Oct 2026. After a red turn, one where any group went red, alone, by blame or by a split, the next takes half as many, down to one. After a green turn, one where a group landed and none went red, it takes twice as many, up to the ceiling. A conflict with `main` is the branch's own and moves nothing, nor does a clash between members, a stop or a withdrawal. The first turn takes the ceiling. A batch narrows on its union, so a bigger one narrows less; that is accepted.
- **The size is kept in `armada-land/batch.json`**, written whole, so a new runner goes on from it. A ceiling lowered below it holds it down. `--status` says it and why: `taking up to 4 — halved after a red at 14:02 UTC`. **The runner records its ceiling there too**, before each turn, and `--status` reads that one, so a shell with another `ARMADA_LAND_BATCH` reports the line's ceiling, not its own; it falls back to its own only where no runner has recorded one.
- **Each branch still lands as its own merge commit, in place order.** Branch one is merged onto `main`, branch two onto that, and so on; the regeneration commits on top, and the push is of the top. `git log --first-parent main` reads one merge per branch, each with its `Landed-from:` trailer, and each outcome names its own merge.
- **The gate and the Checks run once, over the union.** A Check runs when it covers any member's paths, or what landed on `main` since any member was cut.
- **A new gate line that names a file goes to the member that touched it.** Where every path each new line names was changed by exactly one member, in its own diff from its merge-base, those members go back red with their own lines and the rest are gated again in the same turn, keeping their place. A line naming no path, or a path several members or none touched, splits the batch as a red does. Measured 1 Oct 2026: one `no_file_too_long` line named the only member that touched the file, and the blind split behind it cost two more turns of about eight minutes each.
- **A red on more than one member splits the batch in half**, first half first, down to a single branch, where a red is reported to that branch's own agent exactly as it was before batching. Three greens and one red cost three gates, not four, and not one each.
- **Two members that do not merge with each other split the batch too.** That costs merges, never a gate. Alone, the second meets the first on `main` and goes back with the conflict as any branch would.
- **A member that does not merge with `main` itself goes back with the conflict, and the rest go on without it.** Whether a clash is with `main` or with a member before it is one extra merge of that branch with `main` alone.
- **A Check red on `main` itself stops the whole batch as `main`'s**, without splitting: no half of it would pass.
- **`--status` says who gates together.** Each member's line ends `together with` the others while the batch is in its turn.
- **A half waiting its turn asks the queue again before it is gated.** A member whose entry is gone (`armada land --withdraw`) or carries a new nonce (landed again) is dropped from it. A gate already running still finishes, and can land the member.

## What the first run found

The line's first real turn on this repository merged `main` in, ran the gate and every Check either side hit, and went red on `test` — on two `config` tests that were already failing on `main`, which is what sent this page's *asking `main` too* rule into the design. Nothing landed. A branch merging on its own green reading would have carried that state forward as its own.

## The line and the turn

- **The turn is a `runner.lock` file, created exclusively.** A holder that dies leaves a lock naming a pid the next caller finds dead and reclaims, so there is no timeout and no eviction of one still working.
- **The runner is detached into its own process group.** The Bash tool kills a command at 600 s; the runner outlives that call.
- **A dead runner's entry stays queued.** The next `scripts/land` or `--status` finds the lock free and starts a runner, which retakes the turn.
- **An entry is keyed by a hash of its branch** and carries the name, because branch names hold `/`.
- **Every finished entry leaves the line**, whatever the outcome, as soon as its own outcome is known, not when the rest of its batch finishes.
- **A conflict and a red keep their place.** Resubmitted, the entry reuses the place the outcome recorded: the wait was already served.
- **A Check that runs past its limit is killed and read as red, and the line moves on.** The limit is 15 minutes, the retired runner's `CHECK_LIMIT`, and `ARMADA_LAND_CHECK_LIMIT` overrides it in seconds. It holds on the branch and on `main`'s rerun alike, and the kill takes every process group under the Check, since `armada check` starts the command in a group of its own. Until then a hung Check held the turn until somebody killed the runner, and one turn took 4,364 s. A timeout had been ruled out because evicting a holder that is still working puts two merges in flight. This evicts nothing: the runner keeps the turn, kills its own Check and ends the turn red. What it costs is a slow Check that was not hung, such as a cold build plus the app suite, which now reads as red.
- **A turn's Checks run one at a time, and each asks ahead of every other ask for a Check slot.** Decided 2 Oct 2026, when running them at once was dropped: the machine was already at a load of 20 to 32 on 18 cores, so a parallel turn would raise the peak for little. Asking ahead stops a turn queueing behind agents' own `armada check` runs instead. [Manifest](../concepts/manifest.md), *How many Checks run at once*.
- **State lives under the common git directory**, in `armada-land/`, so every worktree of one clone shares one line.
- **Each gate's logs get a directory of their own**, `armada-land/logs/<entry>/<turn>/`, where `<entry>` is the branch's key (or the batch's) and `<turn>` is when the gate started, in UTC. `main`'s reruns of a Check log into the same directory as the turn that asked. The outcome names the files of its own turn. **A Check's log is written as it runs**: the `$ <command>` line first, each chunk of stdout and stderr as it arrives (in arrival order, so the two interleave), and `[exit N]` last. Until 1 Oct 2026 the directory was the entry's alone and each turn emptied it first, so a rerun that landed erased the red before it: about fifteen `desktop_test` files timed out on `main` that morning and the logs that would have said why were gone.
- **A turn's logs are kept for two weeks**, the retired runner's `KEPT_FOR`, and pruned when the runner takes a turn. Age rather than a count per entry, because each batch is an entry of its own and is seldom gated twice, so a per-entry count bounds nothing. Measured 1 Oct 2026: 79 MB over 247 entries, about 0.3 MB a turn.
- **Two worktrees, under `.armada/land/`, kept and reused.** `candidate/` is where a branch is gated, `base/` where `main`'s own runs happen. Inside the repository, because a checkout outside it is not somewhere this project's tooling runs: Vite refuses to serve a `node_modules` outside its allow list and `tsc` cannot name a type through one, and three Bridge Checks failed there for reasons that had nothing to do with the branch. `land/` is neither `worktrees/` nor `bases/`, so nothing here is taken for another checkout, and `.armada/*` is already ignored.
- **Each turn resets its worktree and cleans it, keeping the build directories.** `git reset --hard`, then `git clean -xdff` with `target/` and `node_modules/` excepted — so nothing of the turn before survives but what makes the next one fast. **The lock is what makes reuse safe**: one turn at a time means there is never a second reader of either worktree.
- **A worktree that is missing, unregistered or no longer a worktree is remade**, and a killed runner's half-merged tree is the same case — `reset --hard` clears the merge with everything else.

## Choosing what reruns

**Which Checks a set of paths hits is one answer, shared with Fleet's gate.** `armada covers` reads paths on stdin and asks each Check's `covers`, which calls `Covers::reach` in `crates/core-model/src/workflow/covers.rs`. `ResolvedCheck::covers`, which the gate's skip decision asks, calls the same function.

**Where the repository has workspaces, the keys are `<dir>:<name>`.** `armada covers` answers with the Checks of each manifest the paths gate, root first, and the line runs `armada check <key>` for each, in that workspace's directory. Setup follows: the root's `setup.requires` once, then each workspace's own for the workspaces that have a Check in the set, and a root Command a workspace names runs in the root and is not repeated. A repository with no workspace `armada.yml` sees none of this. [Manifest](../concepts/manifest.md), *Workspace gating*.

**A Check reruns when it covers what landed on `main`, or what the branch changed, or both.** Either side, not both: the pair most likely to break only in combination is a Rust change landing on the base against a branch's TypeScript, where the generated types meet, and asking for both sides skips exactly that.

**Every Check in this repository declares `when:`, including `build` and `test`.** They name what their commands read rather than what they are about — the workspace, the lockfile, `.cargo/`, `protocol-version.toml`, the shipped workflow definitions, `armada.yml` itself, and for `test` the Bridge tree that `xtask`'s own tests read and the documents code reads: `agent-prompt.md`, `design-system.md` and `docs/spikes/`. Any other change to the documents alone hits no Check at all.

**File overlap alone would miss cross-file breakage.** A type changed in one crate breaks a caller in another file, and both sides still hit `test`.

**The gate runs every turn, and the Checks run on every turn it passes.** On an unmoved turn the Checks are the ones the branch's own paths hit; on a moved one, those and the ones what landed on `main` hits. A new gate line is red whatever the Checks say, so none run behind it.

**The line used to run no Check on an unmoved turn, trusting the agent's own run.** That trust went when `work-issue` step 4 became a quick self-check of build, typecheck and the tests of what changed, so the line is now where a branch's Checks are measured in full. Preflight still stamps the tree, so what the line measures is what the agent pushed.

**A Check that failed is asked of `main` too, before the branch is blamed.** Only the Checks that failed, only on a turn that had one, and the answer is cached per base commit — so a green turn pays nothing for this and a red one pays for what it already knows is broken. A Check red on both sides is reported as `main`'s, naming it and sending the reader at `main`; a Check red only with `main` merged in stays the branch's. A turn holding one of each says both. **A Check that times out on `main` is `main`'s for that turn and is not cached**, so the next branch on the same commit asks again: cached, one slow run told every branch after it that `main` was broken. **It is said as a timeout, with the limit** — `test timed out on main itself, past its limit of 15 minutes` — never as `already fails`, which sent a reader looking for a failing test that was not there.

**How a `verify-foundations` run is read:**

| Read | Why |
|---|---|
| Only `FAIL` and `missing:` lines | A warning does not fail `main` either |
| Line numbers normalised out of the subject | A line inserted above an old failure renumbers it |
| Findings counted, not collected into a set | A second violation of one rule in one file reads like the first |
| A non-zero exit naming no failing rule is red | A branch that breaks `xtask` prints one `error[E0433]` and would be gated on nothing |
| A Check whose command is not installed stops the turn | A missing tool is not the branch breaking `main` |
| A Check red on `main` too stops the turn as `main`'s | "Fix your branch" and "fix `main`" send a reader to different places |
| A Check past its limit is red, on either side, and says so | One hang would otherwise hold every branch behind it |
| A Check past its limit on `main` is not cached, and is said as a timeout | A slow run is not a broken commit, nor a failing test |
| The same on `main`'s own run, which stops the turn | There is nothing to compare against |
| `main`'s run cached per commit, only once read as a report | A killed run cached empty makes every branch after it red |
| A new failing line ends the turn before any Check | It is red whatever they say. A 1 Oct 2026 turn was red at 15:12 and ran Checks until 15:17 |
| A run that names no rule still runs the Checks | Nothing was gated, and they say what they can |
| One report carries the gate's crash and the Checks together | An agent reads everything wrong once, not twice |
| A `main` that cannot run it stops every turn, saying so | The branch behind it is not the one to fix |

## What a narrowed Check runs

**A Check that declares `narrow` with `under` runs over what the turn reaches, not the whole workspace.** In this repository that is `test` and `build`. Measured 2 Oct 2026: `test` was 4 to 5.4 of a turn's 13 to 22 minutes, almost all of it compiling crates the turn never touched.

```
the turn's paths (every member's, + what landed on main)
  -> any Cargo.toml, Cargo.lock, build.rs, rust-toolchain, .cargo/ ?        -- yes -> whole
  -> no Cargo.toml at the root, or `cargo tree` fails ?                     -- yes -> whole
  -> a file under a member that is not .rs ?                                -- yes -> whole
  -> + the directory of every member depending on one touched (cargo tree -i, normal, build and dev edges)
  -> armada check <name> --changed, those paths on stdin
       each path the Check's `when` covers must derive a value under `under`,
         or match `outside`                                                -- neither -> whole
       values in `except` dropped; none left and no `outside` path         -- nothing to run, passes
       otherwise                                                           -- narrow.run + each value
```

- **`checks-runner` holds the one Cargo fact, and the Manifest the rest.** `crates/checks-runner/src/reach.rs` asks `cargo tree` what depends on what; `armada check --changed` spells the result through the Check's own `narrow`, read by `checks_runner::narrowed_over`.
- **The gate's reading is strict.** One path it cannot name runs the Check whole, and a verbatim `narrow` never narrows at all, since a file list leaves out what the command reads beside it, such as a config file.
- **A file that is not Rust source runs it whole**, because the dependency graph says nothing about who reads it: `ipc`'s tests read `testkit`'s fixtures without depending on `testkit`.
- **`xtask` is in every narrowed `test`**, written into the Manifest's `narrow.run`: its tests read the whole tree, so no change under `crates/` is outside their reach.
- **`apps/` and `packages/` run `test` as xtask alone.** The Manifest declares them `outside`: paths `narrow.run` already reads, since xtask's tests are the only ones reading either tree. A change touching only them runs `narrow.run` with nothing appended; beside a crate, they add nothing to its `-p` values. Any other covered path `under` cannot name still runs it whole. Decided by the owner 4 Oct 2026, after a Bridge-only change ran every Rust test.
- **An agent's `armada check --changed` is not this reading.** At the default priority on a shared machine it runs the test modules the changed files are or hold, in their own members and no dependents (`docs/concepts/manifest.md`, *How much of the tree a Check reads*). The line sets `ARMADA_CHECK_PRIORITY=normal` and CI `ARMADA_SOLE_TENANT=1`, and either keeps everything described here; `checks_runner::reached` and `narrowed_over` are unchanged.
- **It is said.** The status line and the outcome carry `test narrowed to -p …`, and the turn's `reach.log` holds the paths or the reason it ran whole.
- **`main`'s rerun of a red Check is whole and cached by commit, as before.** It answers whether `main` itself is red, and a whole red is the stronger answer.

## What a turn prepares, and in which order

**The build directories are cloned into both worktrees, and only the generators run between that and `verify-foundations`.** `setup.requires` — `pnpm install` and the browser download — runs only where a Check is about to, after the comparison is read.

```
merge main in -> seed (cp -c) -> regenerate -> verify-foundations -> setup, if it passed and a Check reruns -> the Checks
```

**A stale generated file is regenerated, not refused.** `ARMADA_LAND_REGENERATE` holds the generators, `;`-separated: by default `cargo xtask verify-docs --write` and `cargo xtask verify-tokens --write`, which write what `every_open_question_is_collected` and `the_tokens_generate_what_is_checked_in` read. They run on the candidate only, after the seed so `xtask` builds warm. What they change is committed onto the candidate and lands in the merge with it, and a generator that fails is a red turn naming it. Every other rule reads the tree as the branch left it. `main`'s own run is not regenerated, so a stale `main` stays `main`'s.

**The two sides of the comparison are then identical by construction**, rather than by two code paths being kept in step. What makes it safe to read `verify-foundations` in a tree with no `node_modules` and no built bundle:

- A missing bundle is a warning rather than a failure, and warnings are outside the comparison. `xtask/src/rules_bundled.rs` says so about itself.
- The one rule that shells out to `node` runs the codegen script directly, with no installed dependencies. `xtask/src/rules_vocabulary.rs`.
- Measured: a worktree with neither produced no new failing line against the main checkout's own run.

**What it gives up:** a rule that did read build output would be blind on both sides. The bundle rule already declares a warning for that reason, and a rule that wanted more would have to say so.

**A cold turn and a warm one, measured on this repository** — a worktree reset, the seed, `setup.requires`, then `typecheck` and `build`:

| | Cold, worktree just created | Warm, reused |
|---|---|---|
| Reset and clean | 0.6s | 0.1s |
| Seed, `cp -c target` | 13.2s | none needed |
| `setup.requires` | 2.4s | 0.7s |
| `typecheck` | 12.4s | 12.6s |
| `build` | 20.3s | 0.1s |
| **Total** | **48.9s** | **13.4s** |

**What reuse buys is the build, not the checkout.** `build` is the whole difference: cargo finds its own output where it left it, and `tsc` caches nothing either way, so a turn whose Checks are all TypeScript saves little. The saving grows with what the Checks compile.

**The seed stays on both sides, and a clone that fails stops the turn.** It is an APFS clone and it is what keeps `xtask` from cold-building — but a clone that worked in one tree and not the other would prepare the two sides differently, silently, which is the failure this order exists to remove.

**`main`'s own run is cached by its commit, and a commit does not carry the machine.** A cached result was taken whenever it was taken, with whatever was installed then, so a machine that changed underneath is compared against a reading from before it did. Deleting `armada-land/foundations/` is how that is thrown away.

## The merge

**The runner merges and pushes `main` itself; GitHub is not in the path.** Until 1 Oct 2026 a turn pushed the merged-in candidate onto the branch, waited up to 120 s for GitHub to show it as the pull request's head, merged with `gh pr merge --merge --match-head-commit` and then read the merge commit's first parent to say whether an ungated combination had landed. Every step of that was a round trip to the forge, and the race it reported existed only because the forge made the merge. With one engineer and no CI on GitHub, the forge added wait and nothing that guarded `main`.

- **A `--no-ff` merge commit, never a rebase or a squash.** It is made with `git commit-tree` over the candidate's own tree, with the gated base and the candidate as its parents, which is the commit `git merge --no-ff` would make there because the candidate already holds the base. So the tree that lands is the tree the Checks ran on, by construction.
- **Its message names the branch in a `Landed-from:` trailer**, and the pull request in its subject where one is open.
- **The push of `main` is never forced.** A `main` that moved since the gate refuses it as not a fast-forward, and the turn gates again against the new `main`, the same bounded rounds as before. Nothing is pushed that was not gated against the `main` it lands on.
- **A gate stops at the next Check boundary once `main` has moved.** Only this line pushes `main` from this machine, one turn at a time, so a `main` that moves mid-gate was pushed from somewhere else, and every Check still to run would measure a base nothing can land on. The runner asks the remote for `main`'s head before `setup` and after each Check, one `ls-remote` that fetches nothing; a head other than the gated one ends the gate there, says so on the status line (`main moved to <sha> while <Check> ran; skipped <Checks>, so gating again against it`) and in `moved.log` beside the turn's other logs, and starts the next gate, which merges the new `main` in. A Check already running is let finish. **This spends a round**, from the same `ROUNDS` a refused push spends, so a remote that keeps moving ends the turn the same way: stopped, `main moved during each of <ROUNDS> gates`. A look that fails or takes longer than three seconds reads as "has not moved", and the push's refusal stays the guard.
- **Only `origin/main` moves.** Local `main` is checked out in the owner's checkout, and moving it under that would show as a change nobody made.
- **A runner killed after its push** leaves the entry queued. The next turn finds the queued head already in `main` and reports it landed, naming the merge the killed turn recorded, rather than merging it twice.
- **The gate runs in a throwaway worktree**, never the agent's own tree. A clean tree is required at preflight and at land, and the stamp is the tree id.
- **The runner removes only its own gate worktrees**, under `armada-land/gates/`. An agent's worktree is never removed; the outcome prints the `agent-worktrees` cleanup commands.

## The pull request, and the remote branch

**Both are optional.** The runner reads the branch from this clone, so a branch never pushed lands the same way.

**A pull request that is open is closed as merged by the push.** GitHub marks a pull request merged once its head commit is in the base branch, and the head is an ancestor of the merge. The runner waits up to `ARMADA_LAND_PR_WAIT` seconds (30) for `gh pr view` to say so, then closes it with `gh pr close` and a comment naming the merge. It waits before deleting the remote branch, because deleting the head branch of an open pull request closes it unmerged.

**The remote branch is deleted only where everything on it landed**, with `git push --delete`. One holding a commit that did not land is kept, and so is its pull request, and the outcome says so.

## The guard

**`.claude/hooks/guard_merge.py` refuses `gh pr merge`, any `git push` whose destination is `main`**, including `--delete main`, **and any `git merge` run in the checkout that has `main` checked out**, and splits a compound command so `cd x && git push origin main` is caught too. The merge rule reads `.git/HEAD` of the checkout the command runs in, so a branch catching up with `origin/main` in its own worktree is left alone, and `--ff-only`, `--abort`, `--continue` and `--quit` are too: none adds a commit nobody gated. Added 5 Oct 2026, after a plain `git merge --no-ff` in the checkout at `main` went around the line. Its refusal names `scripts/land preflight`, `scripts/land`, `scripts/land --status` and this page. `.claude/settings.json` registers it as a `PreToolUse` matcher on Bash.

**The hook is gated by the Manifest.** `hooks_test` runs the hook's suite when anything under `.claude/hooks/` changes. The script's own suite, `scripts/test_land.py`, stopped being a Check when CI and pull requests began to replace the line: it was the slowest Check in every turn.

**It cannot see the line's own push, and so needs no way to let it through.** That runs in the detached runner, outside the Bash tool. An allowance keyed on something a command can carry, such as an environment variable, would be one any typed command could claim, so the hook refuses `ARMADA_LAND_RUNNER=1 git push origin main` like any other push to `main`. An agent who goes around it lands a combination nothing checked, and nothing says so afterwards.

## In Bridge

**This panel is becoming the thin pull request view**, every open pull request with how Fleet knows them to connect, and Armada prompts when `main` goes red (`../../.claude/decisions/2026-10-06-ci-and-pull-requests-replace-the-merge-line.md`). Main's state and the open pull requests are built and served (*The hub*, below). A Job taking the red is built too (*When main goes red*, below), and the Recently landed list reads the forge.

**The Merge line page draws each line as a panel, reached by name from the palette, and the Cockpit's merge queue draws the same lines as chips**, `apps/desktop/src/renderer/src/merge-line.tsx` over `packages/components/src/compositions/MergeLine/`. Both read the same views (`viewsOf`). A panel shows place, a state mark, the branch, its pull request and what the runner is doing, with a turn's batch drawn as one bracketed group rather than `together with` on every member. **A turn in its Checks draws them as the plan's boundary strip**, one segment a Check as it stands, rather than the runner's `running <name> (...)`; the runner's words are drawn only for what is not a Check, reading `verify-foundations` or merging main in. Every cell of a row sits on its first line, so a detail that wraps leaves the mark beside the branch. Under it are two lists, each headed and each drawn only with something in it: **Recently landed**, with its merge commit, and **Sent back**, with its Checks strip or conflicted files. The marks are `land_state` in `crates/core-model/domain/enum-verbs.toml`, keyed by `OutcomeState::word`. Bridge's labels are its own and the words on disk and in `--status` do not change: `gating` reads *Preparing to land* (`git-merge`) until the turn's first Check starts and *Running Checks before landing* from then, which Bridge tells apart by whether Fleet serves any `checks` (`packages/screens/src/merge-line.ts`); `merging` reads *Pushing onto main* and `red` *Checks failed*. A conflict's mark is `unplug` and the rail row's is `merge`. The strip carries no `?`: `GroupBoundary`'s guide is the caller's, and no caller passes one (guide 5 was a plan group's and is retired). The mock's lines are `?walk=theMergeLine`.

| What Fleet serves for the pick | What draws |
|---|---|
| No line and no hub, or Fleet has not answered | No panel, no palette entry |
| A line with something in line, landed or sent back, or a hub with open pull requests | The panel, with whichever of the lists have rows |
| A line with nothing in any of the three | The panel, with a picture under its heading and no words: the owner's one exception to the empty-state rule, 2 Oct 2026 |
| All, with lines in more than one repository | One panel per repository, its label beside *Merge line* |

**Fleet served the line from protocol 22.1**, `landed`, `sent_back` and `checks` from 23.2, by reading the runner's files. The reader is removed with the command, so those lists are empty and the hub is the whole answer. What it did, for the record:

```
armada land (retired) --wrote--> <common git dir>/armada-land/{queue,outcomes}/   (no longer read)

Fleet, every 2 s, per served repository --folds the hub
   |  moved?
   +--> merge_lines.changed (MergeLines, whole) --> Bridge replaces state.mergeLines
GET /merge_lines -------------------------------------> Bridge reads it once per connection
                                                        mergeLineViews(mergeLines, pick, repositories) -> one panel each
```

| On disk | On the wire | In the panel |
|---|---|---|
| Queue order | `place`, 1-based | Place |
| Outcome `state`, `waiting` with none | `state` | The mark |
| `detail`, live states only, before ` — together with `, and not while a Check runs | `doing` | The runner's words |
| The names after ` — together with ` | `batch`, the member first in place order | One bracketed group |
| `pr` and `origin` on the forge | `pull_request` `{number, url}` | `#1770`, opening the address |
| `pr_settled`, `landed` only: `merged` when the forge read the push as the merge, `closed_unmerged` when the runner closed it or found it closed | `pull_request.settled`, the pull request's own `Settled` | The pull request badge beside the number, *Merged* or *Closed without merging*; nothing where it is not known |
| `merge_commit`, `landed` only | `merge_commit`, whole | Its first ten characters |
| `failed`, `red` and `stopped` only | `failed` | The failed Checks |
| `conflicts`, `conflict` only | `conflicts` | The files |
| `checks`, each Check the turn runs as `waiting`, `running`, `passed`, `failed` or `timed_out`; `gating`, `red` and `stopped` only | `checks` `{name, state}` | The plan's boundary strip, `GroupBoundary`: one segment a Check, its list open where one failed |

- **The facts are taken only for the state that owns them.** An outcome keeps fields from earlier turns, so a branch that landed and then went red still holds the old merge commit on disk.
- **Recently landed is the three newest `landed` outcomes** of branches no longer queued, by the file's own write. Outcomes are never pruned; this clone held 310 on 2 Oct 2026.
- **Sent back is every `red`, `conflict` or `stopped` outcome of a branch no longer queued, written in the last three days** (`SENT_BACK_FOR` in `adapters::land_state::line`). By age rather than by count, so three newer reds never hide a fourth that is still owed. A branch that has since landed is not there without a rule saying so: a branch has one outcome file, and the landing overwrote the red.
- `off` is still served, the three newest of either, for a Bridge before 23.2. This one does not read it.
- A `gating` outcome with no queue entry is a turn a killed runner left, and is not drawn.
- **A picked repository draws its own line. All draws every repository Fleet serves a line for**, each named by its repository once there is more than one.
- A repository is in the answer only for its hub, with an empty line.
- **A Check in the strip opens its log** in the log panel (owner, 2 Oct 2026), live while the runner writes it and whole once it has ended, over `observe_land_check` since protocol 23.7. The request is the root, the branch and the Check; Fleet finds the file from the branch's outcome and opens nothing else. A Check still `waiting` has no log and is no button.

### The hub

**The panel's head carries main's state, and under the line it lists every open pull request.** Green is a mark with its tooltip. Red is a frame naming the failing CI job, the failing test and the merge that turned it red; a press on the job opens its log. Each open pull request shows how its `ci` stands, and a `ci` red only because main is reads as waiting on the fix. Where the pull request came from a Job, Fleet hands the red to that Job and the head links to it, with no question asked (*When a Job's pull request turns main red*, below). The "Broke in" row names the Job that owns the pull request, by its icon. `?walk=mainRedSession` plays one. A Job that took it is drawn in the band's Fixing row by its glyph and title, which open it (`?walk=main-red-session-fixing`). `?walk=mainGoesRed` plays it on the mock.

**A red is held while newer checks run on main.** When a newer commit's CI is still going the band is caution, not failure: one line, "New checks are running on main", names the running pull request, the red's labelled rows stay under it, and the band's acts are gone, because that run may already have fixed it. Main's state is the newest commit whose run has finished, so a green run on an intermediate commit clears the red even while later ones run: the mark beside the heading says checks are running and no band is drawn. A held run ending red on the same job brings the red band and its buttons back. **Recently landed shows each merge's own run on main**, apart from the checks the pull request passed before it merged: a running one pulses, and a failed one opens that run's failing job's log. `?walk=mainChecksRunning` plays it on the mock.

**Fleet serves the hub since protocol 23.41**, as `hub` on each repository's line, over the same `get_merge_lines` and `merge_lines.changed`. The line's two-second loop folds it on before it compares, so main going red or green and a pull request opening publish through the one loop. The same loop enters each difference on the Fleet's timeline (`practices/protocol.md`, *The Fleet's timeline*): a pull request opening, its checks, its queue place, its merge, and main going red or green. Fleet reads the forge for one repository a sweep interval, rotating, and each visit is three cheap asks: main's head (`concepts/fleet.md`, *What Fleet knows about main's CI*), one listing of the open pull requests, whatever their number, the newest 100, and one of the newest five merged into the base.

| On the forge | On the wire | In the panel |
|---|---|---|
| Main's newest commit and its CI jobs | `hub.main.state`: `green`, `red`, `running` or `nothing_ran` | A mark for green, the band for red, nothing for the other two |
| Each failed job, the Check it maps to, its tests | `hub.main.failed` `{name, check?, log_url?, tests?}` | *Check* where one maps, *CI job* where none does, and *Test* only where the log named one |
| The pull request that merged the red commit, and the Job that held it | `hub.main.merge` `{number, url?, branch?, session?}` | *Broke in*; absent for a direct push |
| An open pull request and the check its Manifest names as `ci_gate` (`ci` where it names none), or every check where none carries that name | `hub.pull_requests` `{number, title, branch, url, author?, ci?, queue?, opened_at?, updated_at?, draft?, mergeable?, review?, additions?, deletions?, mine?}` | A row: the ci mark, the branch, the number |
| A pull request whose failing checks all fail on main too | `ci: waiting_on_main` | The mark and tooltip for a red the fix is to clear |
| A pull request merged into the base, newest five | `hub.merged` `{number, title, branch, url, author?, merged_at, commit?}`, since 23.42 | **Recently landed**: the number opens the pull request, the short merge commit beside it |
| A Job that took the red | `hub.fixing_session` `{id, title?}`, since 23.80 | *Fixing*: the Job's glyph and title, which opens it |

- **An open pull request carries what the forge says of it beyond its checks, read on the same listing** and not one ask a pull request: when it was opened and last moved (`opened_at`, `updated_at`, RFC 3339), `draft` (absent is ready), `mergeable` (`clean`, `conflicting` or `unknown`, the forge still computing after a push, which is not a conflict), `review` (`approved`, `changes_requested`, `review_required`, or `none` where no review rule applies) and the lines `additions` and `deletions`. Each is absent where the forge named none, and from a Fleet before it. `mine` is Fleet's answer to whether the person opened it, absent meaning yes; Bridge draws a pull request that is not mine only while it is in the queue, quietly, and raises no call for it (`concepts/fleet.md`, *Whose pull requests Fleet reads*). Bridge's hub fold carries them, and `author`, onto each pull request it draws. A pull request that has settled loses them with the listing.
- **A red stays red while a fix runs**, so `running` is only a commit nothing has failed on yet. The head draws neither `running` nor `nothing_ran`.
- **A pull request waits on main only when every check that failed on it also fails on main.** A failure on any other check is its own, and a pull request with no failed check is never waiting.
- **Main's log opens through `observe_land_check` under the branch `main`**, with the job's Check or name as the check. Fleet asks the forge for that job's log when it is pressed, bounded to its last 256 KiB, and serves it as a finished Check's. Bridge never calls the forge.
- **Recently landed reads the forge when Fleet serves `hub.merged`**, newest first, and the queue's own outcome files only from a Fleet before 23.42 or where the forge said none. The retired queue's `landed` list is otherwise not drawn.
- **How old the hub is rides on it as `forge_read_at`, and a forge that has stopped answering as `quiet_since`** (both RFC 3339, `docs/concepts/fleet.md`, *What Fleet knows about main's CI*). Bridge puts the read time in the tooltip of main's chip and, while `quiet_since` is set, marks the chip with a glyph naming the time and draws its dot dimmed, so a green nobody has confirmed does not read as a live one. Neither is a phrase on the row.

### When main goes red

**A red is taken by a Job, and a take is one record.** The record is a row for each Job per red (`main_ci_fixes`), keyed by when main first read red, and it is what `hub.fixing_session` is read from. **While a Job holds the red, `take_main_red` and the automatic pickup are refused**, and the band draws the holder in its Fixing row. There are two ways in: Fleet hands the red to the Job that merged it (below), and a Job takes it with `take_main_red` (*Fixing a red across machines*).

- **The brief is the facts Fleet read**, one line each (the Check or CI job, each test, the pull request and its branch), and the failing job's log address and last 60 lines follow it.
- **A take ends when its Job ends, or when main goes green.**
- **A person's pull request** (#1839 was one) has no Job, so nothing is sent. A Job's is handed to the Job, below.
- **The open pull requests are held in memory.** A Fleet restarted shows main at once and the list from its next visit to that repository.
- **The draining queue draws where it did**, beneath the hub, so nothing in flight disappears.

### When a Job's pull request turns main red

**Fleet hands the red to the Job that merged it, and takes the claim itself.** The owner, 15 Oct 2026: both reds on main that day came from Job branches, and nothing was dispatched. A Job is not trusted to call `take_main_red`; Fleet detects the red and claims it.

| Who merged it | What Fleet does, on the turn or reading that finds the red |
|---|---|
| A Job still there | Claims the red for it, tells it as Fleet tells it anything, which wakes it, and records its take |
| A Job that ended, cannot be told, or whose wake is refused | Forks it (the machinery of `fork_this_session`: an ended one at its end, a live one at its latest message of its own), moves the claim to the fork, tells the fork, and records the fork's take, naming the Job it was forked from |

- **The message is the facts Fleet read.** The brief, its log address and last 60 lines, under *Main went red at <commit> after your pull request #N merged. Fleet has claimed the red for you. Fix it.* A fork's reads *after pull request #N, which job <id> merged. Fleet has claimed the red for this fork.*
- **Once per red per Job**, by its take among the red's `main_ci_fixes` rows, which is kept where no store applies (`how` is `took` for a Job woken, `dispatched` for a fork, and the fork's row names its parent in `forked_from`, so the parent is not handed the red again). The hub's Fixing row shows the take (`hub.fixing_session`), it ends when the Job ends or main goes green, and it refuses a second fixer. A red held by a newer run, held elsewhere or with the store silent, or already taken by a Job here, is left alone.
- **The claim comes before the message and before a fork is made**, so a red held elsewhere makes nothing. A fork that cannot be made gives the claim back.
- `crates/fleet/src/session_red.rs`; the proof is `tests::session_red`.

### Fixing a red across machines

**A take is claimed in the central store, so a Fleet on another machine knows the red is held and refuses a second fix.** `main_ci_fixes` is one machine's record; two Fleets serving one repository never read each other's. #2248.

**Nothing is written to the repository.** No ref, no issue, no label and no comment, because a work repository must carry no trace of Armada. The claim is a row in the same store that holds a [pull request's registered owner](../concepts/session/what-is-kept.md#which-machine-owned-a-pull-request), opened by [signing in](../concepts/account.md) and behind the same *Pull request owners* switch in the Manifest. The store sits behind a `Store` port beside `Delivery`, since `Delivery` is the forge's and this is the account's, so Fleet asks the port to claim and meets no store of its own.

| Piece | What it is |
|---|---|
| The key | Two opaque hashes: the repository, and the commit main went red at (`MainCi::red_commit()`). `red_at` is each Fleet's own clock and is never shared. |
| The lock | One call, **claim if nobody holds this red**, answered atomically by the store. The first take wins and the other is told who holds it. |
| The holder | The same row: `session`, its short id, the machine, and when. No title, no Check or test name, no pull request and no repository name. |
| The machine | The host's name, or `machine.name` in settings where two hosts share one. |

**A row never holds work.** Both machines read the same red from the same forge, so the Check, the test and the culprit are each drawn from that machine's own reading. A holder's title is drawn only by the machine that holds it, from its own ledger.

**A claim is made on any take, whoever turned main red.** Blame decides only whether the automatic handoff fires; it is often wrong, and a red nobody local caused is still taken by a Job, and claimed then.

- **The claim comes before the Job is told.** Each way in reads the row first, so a held red makes nothing, then claims before the message or the fork, and a red already held refuses the take as `MainNotFixable::TakenElsewhere` (409 `fleet.main_taken_elsewhere`), naming the machine, holder and age. The automatic pickup skips silently, as it skips a red already held here.
- **A store that cannot be reached refuses the take** (the owner, 13 Oct 2026). A fix nobody else can see is the double fix this exists to stop.
- **A repository with the switch off claims nothing, and shows nothing of another machine's claim.** Its takes proceed on this machine alone, as they did before the store. A repository bound to no profile, or to one the machine has signed out of, is the same.
- **A Job takes a red with `take_main_red`**, which `agent_access` opens to agents as `claim_pull_request` is opened. It writes the claim with the Job's id, and the local take in `main_ci_fixes`; where no store applies it answers `claimed: false` and writes no claim, and the take stays local. A Job is told who holds the red when it is refused, and not before. A Job asking again for the red it already holds is not refused.
- **A take over is asked for, never assumed.** `take_over: true` on `take_main_red` replaces the other machine's row with this one's. A claim shows its age so a stale one reads as stale.
- **Reading the other machine's claim** rides the reading that finds the red: one read of the row for that repository and commit. `hub.fixing_elsewhere` `{machine, holder, since}` carries it beside `hub.fixing`. A remote claim is seen as late as the rotation reaches its repository; a take is never late, because it meets the store itself.
- **Bridge draws another machine's claim in the band's Fixing row**: the holder's chip as a pull request's owner is drawn (the Job glyph and short id, then `monitor` with the machine's name as its tooltip), and the claim's age, `12m`. The band's other acts are not offered; **Take over** is, as a quiet act, and it asks once with the claim's age in the ask before it sends `take_over: true`. A claim an hour old or more wears a `clock` beside its age, and no sentence. Once taken, the band is a local take's Fixing as before. (The owner, 14 Oct 2026; `?walk=red-held-elsewhere`.)
- **A refusal Fleet makes of a press is drawn in the band, not a toast**, for two codes. `fleet.main_claim_not_made` reads *Not made, store unreachable* and `fleet.main_taken_elsewhere` (a take pressed before the hub read the remote claim) reads *Held on another machine*, each a row with **Try again**, which sends the same press. The refusal's text names the holder only as a sentence, with no field, so Bridge does not parse it; the next reading brings `hub.fixing_elsewhere` and the band draws the claim as above. Every other refusal still toasts.
- **A take that ends gives its claim back**: its Fleet deletes the row, so the other machine may take the red. It does so on the reading that next finds the red, and only a row of its own machine's whose Job ended; a take of its own that finds such a row replaces it. A Job asking again for the red it already holds is the same take.
- **Green clears every claim.** Any Fleet that reads green deletes every row under that repository's hash, its own or not, so a claim left by a machine that is asleep still ends.

**The proof is two Fleets on one scripted store** (`crates/fleet`'s `tests::red_claim`, since `crates/acceptance` builds no Fleet): A takes the red and B's hub names A; B's `take_main_red` is refused naming A; a store that refuses every call refuses the take; a repository with the switch off claims nothing; green clears both.

## What it depends on

- `concepts/fleet.md` — *Write clashes* and *What Fleet knows about main's CI*.
- `concepts/manifest.md` — *Which paths a Check covers*.
- `practices/running-locally.md` — *Landing a branch*, how a branch lands now.

