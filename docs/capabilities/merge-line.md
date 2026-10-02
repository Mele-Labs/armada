---
capability: merge-line
issue: 1315
milestone: Throughput
---

# Merges take turns, and each one is checked against the main it lands on

Two branches can each pass every Check, merge a minute apart, and leave `main`
red. Each one's Checks ran against a `main` that had moved by the time it
merged.

**Merges wait in line; work does not.** Branches are worked in parallel, and
write-scope overlap stays a warning ([Fleet](../concepts/fleet.md),
*Write-scope overlap*). Only the step onto `main` takes turns.

`armada land` (`scripts/land` is a thin shim over it) is the local stand-in,
for agents working this repository outside Fleet. It is built so each part
has a named home in Fleet, listed under *Where each part goes in Fleet*. How
to run it is `docs/practices/running-locally.md`, *Landing a branch*.

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
| A branch needs no push and no pull request | The runner reads the branch from this clone |
| An agent lands green work without asking the owner | The agent's own brief; the owner reads what landed afterwards |

## One turn

```
scripts/land preflight    clean tree, commits ahead of main -> stamp HEAD^{tree}, note any open PR
scripts/land              stamp matches -> queue entry -> runner started if none -> returns
                                                  |
runner (holds the turn lock) ----------------------+
  take the first ARMADA_LAND_BATCH entries (4), in place order: one batch
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
  covers(each branch's paths, + what landed since it was cut) -> setup -> armada check each
        | red, one member -> outcome red, nothing pushed
        | red, several    -> split the batch in half, first half first, and take each
  git push origin <top>:main  -- not a fast-forward -> gate again (bounded rounds)
        | pushed -> each member: outcome landed, naming its own merge
  each member: PR open? wait for GitHub to read it merged, else gh pr close --comment <merge>
  each member: remote branch all landed? -> git push origin --delete <branch>; print cleanup commands
```

## Batching

**A turn takes up to four waiting branches and gates them once.** One gate per branch made the wait grow with every agent landing. `ARMADA_LAND_BATCH` sets the size; `1` is the line as it was.

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
- **A Check that runs past its limit is killed and read as red, and the line moves on.** The limit is 15 minutes, `CHECK_LIMIT` in `crates/armada/src/land/env.rs`, and `ARMADA_LAND_CHECK_LIMIT` overrides it in seconds. It holds on the branch and on `main`'s rerun alike, and the kill takes every process group under the Check, since `armada check` starts the command in a group of its own. Until then a hung Check held the turn until somebody killed the runner, and one turn took 4,364 s. A timeout had been ruled out because evicting a holder that is still working puts two merges in flight. This evicts nothing: the runner keeps the turn, kills its own Check and ends the turn red. What it costs is a slow Check that was not hung, such as a cold build plus the app suite, which now reads as red.
- **State lives under the common git directory**, in `armada-land/`, so every worktree of one clone shares one line.
- **Each gate's logs get a directory of their own**, `armada-land/logs/<entry>/<turn>/`, where `<entry>` is the branch's key (or the batch's) and `<turn>` is when the gate started, in UTC. `main`'s reruns of a Check log into the same directory as the turn that asked. The outcome names the files of its own turn. Until 1 Oct 2026 the directory was the entry's alone and each turn emptied it first, so a rerun that landed erased the red before it: about fifteen `desktop_test` files timed out on `main` that morning and the logs that would have said why were gone.
- **A turn's logs are kept for two weeks**, `KEPT_FOR` in `crates/armada/src/land/logs.rs`, and pruned when the runner takes a turn. Age rather than a count per entry, because each batch is an entry of its own and is seldom gated twice, so a per-entry count bounds nothing. Measured 1 Oct 2026: 79 MB over 247 entries, about 0.3 MB a turn.
- **Two worktrees, under `.armada/land/`, kept and reused.** `candidate/` is where a branch is gated, `base/` where `main`'s own runs happen. Inside the repository, because a checkout outside it is not somewhere this project's tooling runs: Vite refuses to serve a `node_modules` outside its allow list and `tsc` cannot name a type through one, and three Bridge Checks failed there for reasons that had nothing to do with the branch. `land/` is neither `worktrees/` nor `bases/`, so nothing here is taken for a Job's checkout, and `.armada/*` is already ignored.
- **Each turn resets its worktree and cleans it, keeping the build directories.** `git reset --hard`, then `git clean -xdff` with `target/` and `node_modules/` excepted — so nothing of the turn before survives but what makes the next one fast. **The lock is what makes reuse safe**: one turn at a time means there is never a second reader of either worktree.
- **A worktree that is missing, unregistered or no longer a worktree is remade**, and a killed runner's half-merged tree is the same case — `reset --hard` clears the merge with everything else.

## Choosing what reruns

**Which Checks a set of paths hits is one answer, shared with Fleet's gate.** `armada covers` reads paths on stdin and asks each Check's `covers`, which calls `Covers::reach` in `crates/core-model/src/job/covers.rs`. `ResolvedCheck::covers`, which the gate's skip decision asks, calls the same function.

**A Check reruns when it covers what landed on `main`, or what the branch changed, or both.** Either side, not both: the pair most likely to break only in combination is a Rust change landing on the base against a branch's TypeScript, where the generated types meet, and asking for both sides skips exactly that.

**Every Check in this repository declares `when:`, including `build`, `test` and `format`.** They name what their commands read rather than what they are about — the workspace, the lockfile, `.cargo/`, `protocol-version.toml`, the shipped workflow definitions, `armada.yml` itself, and for `test` the Bridge tree that `xtask`'s own tests read. A change to the documents alone now hits no Check at all.

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
- **Only `origin/main` moves.** Local `main` is checked out in the owner's checkout, and moving it under that would show as a change nobody made.
- **A runner killed after its push** leaves the entry queued. The next turn finds the queued head already in `main` and reports it landed, naming the merge the killed turn recorded, rather than merging it twice.
- **The gate runs in a throwaway worktree**, never the agent's own tree. A clean tree is required at preflight and at land, and the stamp is the tree id.
- **The runner removes only its own gate worktrees**, under `armada-land/gates/`. An agent's worktree is never removed; the outcome prints the `agent-worktrees` cleanup commands.

## The pull request, and the remote branch

**Both are optional.** The runner reads the branch from this clone, so a branch never pushed lands the same way.

**A pull request that is open is closed as merged by the push.** GitHub marks a pull request merged once its head commit is in the base branch, and the head is an ancestor of the merge. The runner waits up to `ARMADA_LAND_PR_WAIT` seconds (30) for `gh pr view` to say so, then closes it with `gh pr close` and a comment naming the merge. It waits before deleting the remote branch, because deleting the head branch of an open pull request closes it unmerged.

**The remote branch is deleted only where everything on it landed**, with `git push --delete`. One holding a commit that did not land is kept, and so is its pull request, and the outcome says so.

## Where each part goes in Fleet

| `scripts/land` | Fleet | Notes |
|---|---|---|
| The line and the turn | The sweep, and the places line | 1 |
| `armada covers` | `ResolvedCheck::covers`, already shared | 2 |
| Merging `main` in | `crates/adapters/src/merging_in.rs` | 3 |
| The two worktrees under `.armada/land/` | `.armada/worktrees/<handle>` and `.armada/bases/<sha>` | 8 |
| Asking whether `main` fails it too | *A test broken on main* | 9 |
| Rerunning the Checks | A gate run over the merged worktree | 4 |
| The merge | `crates/adapters/src/onto_base.rs`, shared | 5 |
| Outcome file and `--status` | The Job record, served on detail | 7 |

1. `docs/concepts/fleet.md`, *Checks share one limit*. A press to merge joins a line the sweep turns, one landing per repository at a time.
2. Fleet needs no port for this part.
3. It already merges and never rebases, and leaves conflict markers for a Drone to clear.
4. The reruns take places like any other Check run.
5. A Manifest chooses, with `merge_by` ([Manifest](../concepts/manifest.md), *How work lands*). `forge`, the default, asks the forge to merge the pull request, as Fleet always has. `push` makes the merge commit and pushes the base through `adapters::onto_base`, the same code this line lands with, so the two cannot come to land work two ways. A base that moved past the branch is merged into it in the Job's own worktree and the Job's Checks run again, up to the same `ROUNDS` as this line, before the push is asked again ([Manifest](../concepts/manifest.md), *How work lands*). The push also names the tree the Job's Checks last passed on and refuses any other, so a head Fleet's own sweep merged the base into is gated where it stands first. This line has no such sweep: its candidate is always the tree it just gated.
6. *Proving what merged*, in `docs/concepts/manifest.md`, holds for both: the forge names the base after a forge merge and the push names it after a push, and the main checkout is brought up to what the remote holds before the run. A push needs no first-parent reading to say whether an ungated combination landed: it is refused unless the base is still the one the branch holds.
7. An outcome becomes a Job event and a log line. Until then Fleet reads this line's own files and serves them as `get_merge_lines` (*In Bridge*); the queue and outcome types already live in `adapters::land_state`, where Fleet can reach them.
8. Same directory, same reason: a Job's Checks run inside the repository because that is where this project's tooling works. **Fleet's own lifecycle already answers the reuse half** — a base checkout belongs to a commit and every Job on that commit shares it, and `setup.seed` warms it when the base moves. What it does not do is drop the one it has superseded: two were found holding 16 GB after two merges, and `armada clean` is the only thing that takes them back.
9. `docs/concepts/fleet.md`. Fleet runs one named test against a checkout of `main` on a Drone's word; the line asks the same question of a whole Check, without being asked.

## What does not port

| Piece | Why it stays here |
|---|---|
| The cross-process turn lock | Fleet is one process; its turn is in-process state |
| The detached runner | Fleet already outlives the caller |
| `ARMADA_LAND_SETUP` and `ARMADA_LAND_SEED` | A second copy of `setup:`, which Fleet reads from the Manifest |
| The `GENERATED by` header as a regeneration recipe, and `ARMADA_LAND_REGENERATE` | Fleet leaves conflicts and stale outputs to a Drone |
| Printing cleanup commands | `armada clean` gives a Job's worktrees back itself |
| The installed `armada` binary gating | Fleet gates with its own build |

**Gates run the `armada` on `PATH`, not one built from the gated tree.** `armada check` only resolves a name in `armada.yml` and spawns its command, so the binary needs only to read the file. A change to how `armada.yml` is read is the one case it gets wrong.

## The guard

**`.claude/hooks/guard_merge.py` refuses `gh pr merge` and any `git push` whose destination is `main`**, including `--delete main`, and splits a compound command so `cd x && git push origin main` is caught too. Its refusal names `scripts/land preflight`, `scripts/land`, `scripts/land --status` and this page. `.claude/settings.json` registers it as a `PreToolUse` matcher on Bash.

**Both halves are gated by the Manifest.** `scripts_test` runs the script's suite when anything under `scripts/` or `armada.yml` changes, and `hooks_test` runs the hook's when anything under `.claude/hooks/` does — two Checks rather than one, because a `run` gets no shell to chain them with and because the two are read by different changes.

**It cannot see the line's own push, and so needs no way to let it through.** That runs in the detached runner, outside the Bash tool. An allowance keyed on something a command can carry, such as an environment variable, would be one any typed command could claim, so the hook refuses `ARMADA_LAND_RUNNER=1 git push origin main` like any other push to `main`. An agent who goes around it lands a combination nothing checked, and nothing says so afterwards.

## In Bridge

**Overview draws each line as a panel below its lists, and the rail's Merge line row draws the same panels on their own**, `apps/desktop/src/renderer/src/merge-line.tsx` over `packages/components/src/compositions/MergeLine/`. The two share one fold. A panel shows place, a state mark, the branch, its pull request and what the runner is doing, with a turn's batch drawn as one bracketed group rather than `together with` on every member. **A turn in its Checks draws them as the plan's boundary strip**, one segment a Check as it stands, rather than the runner's `running <name> (...)`; the runner's words are drawn only for what is not a Check, reading `verify-foundations` or merging main in. Every cell of a row sits on its first line, so a detail that wraps leaves the mark beside the branch. Under it are two lists, each headed and each drawn only with something in it: **Recently landed**, with its merge commit, and **Sent back**, with its Checks strip or conflicted files. The marks are `land_state` in `crates/core-model/domain/enum-verbs.toml`, keyed by `OutcomeState::word`; `gating` reads *Running Checks before landing* in Bridge, and stays `gating` on disk and in `--status`. A conflict's mark is `unplug` and the rail row's is `merge`. The mock's lines are `?walk=theMergeLine`.

| What Fleet serves for the pick | What draws |
|---|---|
| No line, or Fleet has not answered | No panel, no rail row, no palette entry |
| A line with something in line, landed or sent back | The panel, with whichever of the three lists have rows |
| A line with nothing in any of the three | The panel, with a picture under its heading and no words: the owner's one exception to the empty-state rule, 2 Oct 2026 |
| All, with lines in more than one repository | One panel per repository, its label beside *Merge line* |

**Fleet serves the line since protocol 22.1**, `landed`, `sent_back` and `checks` since 23.1, and reads it rather than runs it:

```
armada land (another process) --writes--> <common git dir>/armada-land/{queue,outcomes}/
                                                   |
Fleet, every 2 s, per served repository --reads----+   adapters::land_state::line, no runner, no mkdir
   |  moved?                                            git asked once per repository, not per read
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
| `merge_commit`, `landed` only | `merge_commit`, whole | Its first ten characters |
| `failed`, `red` and `stopped` only | `failed` | The failed Checks |
| `conflicts`, `conflict` only | `conflicts` | The files |
| `checks`, each Check the turn runs as `waiting`, `running`, `passed`, `failed` or `timed_out`; `gating`, `red` and `stopped` only | `checks` `{name, state}` | The plan's boundary strip, `GroupBoundary`: one segment a Check, its list open where one failed |

- **The facts are taken only for the state that owns them.** An outcome keeps fields from earlier turns, so a branch that landed and then went red still holds the old merge commit on disk.
- **Recently landed is the three newest `landed` outcomes** of branches no longer queued, by the file's own write. Outcomes are never pruned; this clone held 310 on 2 Oct 2026.
- **Sent back is every `red`, `conflict` or `stopped` outcome of a branch no longer queued, written in the last three days** (`SENT_BACK_FOR` in `adapters::land_state::line`). By age rather than by count, so three newer reds never hide a fourth that is still owed. A branch that has since landed is not there without a rule saying so: a branch has one outcome file, and the landing overwrote the red.
- `off` is still served, the three newest of either, for a Bridge before 23.1. This one does not read it.
- A `gating` outcome with no queue entry is a turn a killed runner left, and is not drawn.
- **A picked repository draws its own line. All draws every repository Fleet serves a line for**, each named by its repository once there is more than one.
- A repository nobody has run `armada land` in is not in the answer, and gains no `armada-land/` from being read.

## What it depends on

- `concepts/fleet.md` — *Write-scope overlap*, *Catching a branch up*, and what Fleet knows after a merge.
- `concepts/manifest.md` — *Which paths a Check covers* and *Proving what merged*.
- `practices/running-locally.md` — *Landing a branch*, how to run it and read its outcomes.

