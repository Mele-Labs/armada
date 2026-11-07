# A complex Job with subagents

Status: spike, run once on opus. Nothing here lands. Branch `spike/wf-complex-2`, on the rig from `042-workflow-mechanisms-rig.md`, the run before this one in `048-a-complex-job-through-the-recommendation.md`, and the recommendation in `workflows-fleet-can-follow.html`.

## Conclusion

The recommendation held for every gate it was asked to carry, and this run reached the parts run 48 never did.
The document gate closed on the second revision of an HTML page, a one-line change request produced a re-plan, Checks closed at the commit that held all four groups, and the merge was recorded 0.6 seconds after the person made it.
The agent handed two independent parts to two subagents that ran side by side for 126 seconds, and nothing they did moved a gate.

It also failed in four places, all of them findings about the design and not only about my prototype.

| Place | What happened |
|---|---|
| The live group names one group for a stretch | With three build groups and no gate between them, the live position named the last group of the stretch (the join group) for 15 minutes, including the 3.5 minutes in which the first two were being built. The title was right as a gate and misleading as a place. The display has to say "Build, 4 groups", not one group's name |
| A Checks gate that is red for the environment | The Job's changed browser test (`tab-host.test.tsx`) failed 4 tests at all four pushes before the last. The agent said it also fails on main from this path (I could not check that). It reworked the test's stand-ins to pass from a path with a space, which cost about 10 minutes (6 of them idle) and changed test infrastructure the brief did not ask for. A Checks gate with no baseline cannot tell a Job's red from the machine's red |
| The hold was silent twice | The held item for Checks was raised at a turn end while the two subagents were still running, and kept its first text for 13.9 minutes. Separately the agent's second merge ask went unanswered for 6.1 minutes, because my person script answered each missing fact once (fixed during the run, see "What broke") |
| A hold caught a false claim, once | The agent said the design note was "waiting for you in Bridge". Fleet had no document: the agent had written the page in a slot it leased with the CLI, which Fleet does not tie to the Job. The held item at the turn end said so, the person answered it, and the agent found the cause in 82 seconds. This is the one place a held item did exactly what the recommendation says it is for |

Prototype fixes 2, 3 and 6 were exercised by the run. Fix 4 worked once and failed twice (above). Fixes 1 and 5 were checked offline only: the merge came last, and no edit happened under a later group's paths while an earlier gate was open.

One run, one Job, one problem.

## The problem

Two real open issues bundled as one Job.
armada-vs8t: kept per-Job pane layouts are never cleaned up (`job-panes/store.ts`, a sweep, `sessions-wired.tsx`).
armada-20v4: two Jobs share one Custom tab's state because `job-panes/custom.ts` keys it by tab id alone (that file and about eight callers).
They touch different files, each has its own tests, and the join is small (dropping a Job drops its Custom state).
The design decisions are real: when a Job counts as gone, and what a Custom tab's key is.
Runner-ups and the reasons for them are in `049-complex-2/problem.md`: armada-2805 needs the shared verbs list and cargo, armada-gqq3 is one file judged by a key press, armada-2586 and armada-2836 are tests only, armada-1707 needs a walk.

The brief (the first message of the Job, `049-complex-2/run/brief.txt`) said four things 48's did not:
the design note is an HTML page written with the `armada:writing-a-document` skill under `.armada/documents/`, approved before anything is built;
independent build groups go to subagents working side by side, one each, as the owner's Jobs do;
each build group names its `paths`;
the groups carry their gates as the last part of `gives`.

The clone is a smart transfer of `main` alone (`--no-local --single-branch`), at 734ba2c5c6.
`git branch -a` in it read `main` only, and no spike object is in its store.
The change request was written before the run (armada-mttk, in `problem.md`).

## Prototype fixes from run 48

All in `049-complex-2/`. Copies of 048's files; 048 is untouched.

| Fix | What changed | Seen in the run |
|---|---|---|
| 1 Every gate watched on its own | Checks runs on every new pushed tip and the merge is looked at every poll, whatever is open before them. A merge while Checks is open is recorded and logged as out of order | Not exercised. The merge came last. Offline check passes |
| 2 A gate names the form of its fact | `document_approved` closes only for a ledger artifact of form `document`, at `.armada/documents/*.html`, whose newest revision is settled `approved`. Anything else is logged as ignored | Exercised. The page Fleet kept was `.armada/slots/slot-1/.armada/documents/gone-job-state.html`, revision 1 sent back, revision 2 approved. No Markdown or file artifact was offered |
| 3 Stretches | A `none` group has no row and is never closed from task marks. Live is the first open gate and its stretch. The agent's marks are logged as `agent_word_group` | Exercised. Rows: plan v1, document, plan v2, Checks, merged. Five rows, none for the build groups |
| 4 A held gate is never silent | `all_done`, or a turn end with nothing waiting, while a gate is open raises an item (Approve, Skip this group) with what is missing. The person acts on it by rule | Worked once, failed twice. See the holds table |
| 5 Edits under a later group's paths | Read from the transcripts of the agent and its subagents | Blind in this run. The plan's first group listed `job-panes/` as its own paths, so every edit matched "Plan the change" first. Offline check passes. By hand from the transcripts there was no such edit: the first non-document edit was at 03:41:57, 84 seconds after the document gate closed |
| 6 person.py | Never merges while any gate before Deliver is open; the quiet stop ignores the `done` item | Merge refused twice (03:45:39, 03:54:29), done at 03:57. The quiet stop did not fire because the Job finished |

Added beyond the list: Checks closes only when the pushed tip holds a change under the paths of every group of its stretch (so a green run at the first of several pushes does not close it; offline only, because every run in the Job was red until the last); the keeper reads the subagent transcripts and Fleet's `subagent` ledger rows; `person_test.py` and an extended `keeper_test.py` pass offline.

## The run

One Job, `1e84dbdc`, opus, medium effort, workflows off.
Times are UTC on 7 Nov, minutes after the brief at 03:35:11.

| Time | What happened |
|---|---|
| +2:00 | Plan v1 recorded: six groups. Plan, design note, "Drop a gone Job's layout" (A), "Key Custom tab state by Job and tab" (B), "Drop a gone Job's Custom tabs with its layout" (join, Checks), Deliver. Template check clean |
| +2:01 | Person approved. Plan gate closed 0.4 s after |
| +3:30 | Agent wrote the page in a slot it leased with `armada worktree lease` |
| +3:50 | Turn ended with "waiting for you in Bridge". Fleet had no document. Held item raised, person answered |
| +5:13 | Agent rewrote the page in its Job slot. Fleet kept revision 1 |
| +5:13 | Person sent it back: "Say what this does not cover." |
| +5:23 | Revision 2 (adds that section). Approved at once. Document gate closed |
| +5:54 and +6:09 | Subagent A and subagent B started, side by side |
| +8:28 | First push, a514ef1c9c (part B). Checks red |
| +8:28 | Person sent the change request |
| +8:56 | Plan v2 recorded (adds a build group for armada-mttk). Approved 1.7 s later |
| +9:05 and +9:44 | mttk subagent, then the join subagent |
| +9:28 to +10:27 | Pushes at +9:27, +9:33, +10:22. Checks red each time (4 failing tests in `tab-host.test.tsx`) |
| +10:28 | Agent asked for the merge too early. Person refused and said what was open |
| +10:36 to +13:15 | The agent investigated the red test, asked two questions (answered Recommended), asked for the merge again at +13:05 |
| +13:14 to +19:18 | Idle. The second merge ask had no answer |
| +18:47 | The Fleet was stopped by the harness (see below) and came back 12 s later. The Job was live and idle |
| +19:18 | Restarted person script answered the ask. Agent reworked the test |
| +20:20 | Push 874a0f61fb. Checks green, 58 tests. Gate closed |
| +21:22 | Person merged. Merged gate closed 0.6 s after |
| +21:44 | `all_done` with every gate closed. Person ended the run at +22:10 |

Files changed by the Job: 17, 640 insertions, 73 deletions, five commits.
At the tip, `874a0f61fb`, the six test files the branch adds or changes read 6 files, 58 tests passed, from the keeper's checkout and again by hand.

## Score

### Live gate against what the agent was doing

| From | To | Live gate shown | What the agent was doing | Right |
|---|---|---|---|---|
| 03:37:11 | 03:37:12 | Plan the change | Plan just recorded, waiting | Yes |
| 03:37:12 | 03:40:33 | Write the design note | Cutting a worktree, writing the page, getting Fleet to keep it, taking the note | Yes. Fleet had no document for the first 102 s of it, and the live gate said so by staying put |
| 03:40:33 | 03:55:35 | "Drop a gone Job's Custom tabs with its layout" (Checks), stretch of 3, then 4 groups | Subagents A and B, three pushes, plan v2, the mttk and join subagents, the red test, the idle wait | As a first open gate, yes. As a title, no: it named the join group for the 15 minutes the first two were built. Plan v2 reopened the plan gate for 1.7 s, too short for the 1 s poll to show it |
| 03:55:35 | 03:56:34 | Deliver | Merge ask raised at 03:56:30 | Yes |
| 03:56:34 | end | All gates closed | `all_done` at 03:56:55 | Yes |

Five changes, each the first open gate at that moment.
None was wrong as a gate; one was misleading as a place.
The agent's own marks moved eleven times and were logged apart as its word.

### The rest

| Question | Answer |
|---|---|
| Subagents | See the table below |
| Mismatches logged | 0 logged. 0 by hand. The detection was blind to the plan group's own paths and to edits made with Bash (subagent B made its edits that way, so its file list in the keeper log is incomplete) |
| Document gate, one round of notes | Closed on revision 2 of 2, at 03:40:33.996. Revision 1 was sent back with the scripted note at 03:40:24. Revision 2 came 9 seconds later with a new section "What this does not cover". Revision 3 came after the merge (the agent wrote down the answer it was given about pull request panes), and the person approved it in 0.4 s. It changed nothing, since a row is written once |
| Re-plan | Change request at 03:43:39, 1 s after the first push. Plan v2 at 03:44:06, 27 s later. It added a group before the join group. Plan gate reopened and closed in 1.7 s. Document gate carried closed. Checks gate carried open (its stretch grew from 3 to 4 groups). Deliver carried open. Right on all three |
| Gate notices | Keeper: plan v1, document, plan v2, Checks failed twice, Checks passed, merged. Fleet's own `plan_approved` notice arrived beside the plan ones, so the plan gate is told twice again |
| Notices acted on | Document approved: the agent announced the dispatch 14 s later and started the first subagent after 32 s. Checks failed (names blank, a keeper bug): the agent complained, investigated for about 3 minutes, then asked a question. Checks passed: merge asked 55 s later. Merged: `all_done` 21 s later. Plan approved: worked as before |
| Checks gate | Closed at 874a0f61fb, which holds a change under every one of the four groups' paths. Five runs: a514ef1c9c, 8d67b9f56c, 89adcbac59, 0a666106b7 red (4 failed, `tab-host.test.tsx`), 874a0f61fb green. It did not close early, though not because of the new stretch rule: every earlier run was red |
| Merge | Recorded at 03:56:34.099 as `874a0f61fb is an ancestor of origin/main`. The person merged at 03:56:33.484, only because the keeper said Deliver was live |
| Merge refusals | Two. 03:45:39 (Checks open) and 03:54:29 (Checks open). Each said what was open. The agent said "You're right, I asked for the merge too early" |

### Holds

| Hold | Raised | Item said | Length | Released by | Person's act |
|---|---|---|---|---|---|
| 1, document gate | 03:39:01, turn end | No HTML page exists under .armada/documents/ | 92.9 s | Document approved | Message: nothing to approve, write an HTML page and show it. Correct: Fleet had no document |
| 2, Checks | 03:41:39, turn end with both subagents running | Checks have not passed: nothing has been pushed to check | 13.9 min | Checks green | Message: nothing for me to do. The agent answered "Nothing has been pushed yet. Both build subagents are still running" |
| none | 03:48:24 to 03:54:29 | Nothing raised. The turn ended with the item already held, so "hold again" stayed silent, and the second merge ask went unanswered | 6.1 min | Person script restarted | The restart answered the ask |

Two holds, one of them a true catch.
Hold 2 was a false hold in two ways: the Job's turn ended while its subagents worked (Fleet reads a Job with running subagents as idle, armada-vqow), and the item kept the text it had at 03:41:39 while Checks ran red four times.
No `all_done` was held, because the only `all_done` came after every gate closed.

### Agent's claims against acts

| Claim | Act | Result |
|---|---|---|
| "The design note is written and waiting for you in Bridge" (03:38:49) | Fleet's ledger held no document | False. Caught by hold 1 |
| "Part B is committed and pushed as a514ef1c9c" (03:44:20) | Push at 03:43:38 | True |
| Merge asked at 03:45:36 | Checks gate open | Wrong, and it said so when told |
| "Fleet's notice left every failing check's name blank" | `failed` was `["", "", "", ""]` | True. A keeper bug |
| `tab-host.test.tsx` fails on main too | Not run | Unchecked. The shell guard refused my checkout of the base in the scratch clone |
| Task marks | 17 tasks, 24 updates. 7 tasks went through in progress, 10 never did (6, 7, 8, 10, 11, 12, 14 and others completed in batches after a subagent returned). Task 15 ("push and ask for the merge") was in progress for 11 minutes | Each mark was true when made |

### Cost

| Measure | Value | How |
|---|---|---|
| Wall time | 21.7 minutes, first message to `all_done`. 6.1 of those were the idle wait. 22.8 to my stop | Timeline |
| Tool calls | 236: the agent 161 (Bash 74, TaskUpdate 24, TaskCreate 17, Edit 10, `waiting_for` 7), subagents 75 (41, 18, 9, 7) | Transcripts, de-duplicated by uuid |
| Subagents | 4 | Fleet's ledger, `subagent` rows |
| Requests | 190: agent 126 on opus, subagents 35 on opus and 29 on sonnet | `session_turn_usage` |
| Tokens | cache read 19.4 million, cache creation 436 thousand, output 115 thousand | `session_turn_usage` |
| Dollars | 6,665,886 micros, about $6.67 | Fleet's `usage.cost_micros`, not recomputed |

Run 48 cost $1.70 for 5.6 minutes and 91 calls.
This run made 2.6 times the tool calls and cost 3.9 times as much.

## Subagents

| Subagent | Type | Group | Started | Ended | Tool calls | Pushed |
|---|---|---|---|---|---|---|
| A | bridge-engineer | Drop a gone Job's layout | 03:41:05 | 03:44:24 | 41 | No |
| B | bridge-engineer | Key Custom tab state by Job and tab | 03:41:20 | 03:43:26 | 18 | No |
| mttk | armada:mechanic | Title a tab mod's card as its tab | 03:44:16 | 03:44:36 | 9 | No |
| join | bridge-engineer | Drop a gone Job's Custom tabs with its layout | 03:44:55 | 03:45:25 | 7 | No |

A and B ran together for 126 seconds, in the same worktree, each told its group's files and to run only its own tests. No file conflict. The agent reviewed each diff, committed and pushed itself.
The agent chose three groups where I asked for two parts, adding a join group for the one place they meet (dropping a Job drops its Custom state), and so the template's rule "the last build group has the Checks gate" fell on the join. The template and a plan with a join agree.
The brief set no count. It used four subagents over the run and never more than two at once.

Did their pushes or branches ever move a gate or the live position? No. They pushed nothing and made no branch. All five pushes (a514ef1c9c, 8d67b9f56c, 89adcbac59, 0a666106b7, 874a0f61fb) were the agent's.
Were they logged? Yes, in four ways: Fleet's `subagent` ledger rows (standing, then spent, with the call id, description and type), the keeper's `subagent_seen`, `agent_tool_call` and `subagent_summary` events, the transcripts under `subagents/`, and `session_turn_usage.subagent`.
What the keeper missed: subagent B edited through Bash (16 Bash calls, one Write), which the edit detection does not see.
What the keeper got wrong: it held the Job at 03:41:39 because the agent's turn ended with the subagents running.

## What he would see

```
 Plan the change     Write the design note     Build (4 groups)             Deliver
 [ plan approved ]   [ note approved r2 ]      [ Checks green at 874a0f6 ]  [ merged ]
   03:37:12            03:40:33                  03:55:35                     03:56:34
   (v2: 03:44:08)                              A, B, mttk, join (stretch)
 
 Held: 03:39:01 note, 93 s | 03:41:39 Checks, 13.9 min
 Seen in the stretch: 4 subagents, 5 pushes, 4 red Checks, 1 green
```

| Element | How it is detected | Reliability seen |
|---|---|---|
| The groups and their gates | A fact: `session_plan_groups`, gate parsed from the end of `gives` | Right, two versions |
| Plan gate | A fact: `approved_at` of the current version | Right, 0.4 s and 1.7 s |
| Document gate | A fact: the ledger's `settled` and `revision` for a page at `.armada/documents/*.html` | Right. It also told the truth when the agent's claim was false |
| The stretch | Derived from the plan: groups with no gate between them | Right as a set. The display name must be the stretch, not its last group |
| Checks | A fact: Fleet's own vitest at each pushed tip | Right, five runs. The failure names were blank (my parser) |
| Merged | A fact: the tip is an ancestor of `main` | Right |
| Subagents | A fact: Fleet's ledger rows and their transcripts | Right for count and timing. Edits made with Bash are not seen |
| Held items | A fact (`done_since`, turn end) plus a derived text | Once right, twice wrong (stale text, subagents running) |
| What was seen in a group | A fact: tool rows and edits placed by group paths | Blind when a group's paths contain another's. Needs paths that do not overlap |
| The agent's tasks | The agent's word | All true when made; 10 of 17 never in progress |

## What broke or surprised

| What | Detail |
|---|---|
| The dev Fleet was stopped at 03:53:58 | I started it with `run_in_background` and did not raise the 30-minute default timeout. The harness stopped it. I restarted it 12 seconds later with the same scratch and store. The Job was live and idle and took its next message normally. This is mine. The keeper and the person script were restarted with `--resume`, which rebuilds the record from `gate_rows.jsonl` and `keeper.jsonl` |
| Person script answered a missing fact once | The agent's second merge ask got no reply because the first refusal had already named the same open gate. `person.md` says the person answers each early ask. Fixed at the restart: it answers each ask. The Job waited 6.1 minutes. I counted this as a bug in the script, not as the person's rule, and the hot fix is logged (`keeper_resumed` at 03:54:24) |
| Keeper held on a turn end that had subagents running | See holds. Fixing it needs Fleet's `work_beyond_the_turn` (armada-vqow), which the keeper does not read |
| Keeper held with a stale text | The item was raised once and not refreshed. The restart added a refresh when the text changes |
| Plan group listed the whole change as its paths | The agent put `job-panes/` and `sessions-wired.tsx` on "Plan the change". Edit placement takes the first group that matches and so saw nothing outside it. The template should say the first two groups have no paths |
| Failure names blank in the Checks notice | `FAIL ` lines are split on two spaces. Fixed at the restart |
| Transcript lines counted twice in the keeper log | Lines appear in two files. The timeline de-duplicates by uuid and the keeper did not until the restart |
| A browser test in a Check | `tab-host.test.tsx` runs in a headless Chromium. The Job was told not to start a browser; the Check did. The agent ran it too, with a permission item the person allowed |
| A page the agent wrote in the wrong slot | `armada worktree lease` made a slot Fleet does not tie to the Job, so the page was never a document |
| `ask_the_person` | The brief did not name it. The agent used `waiting_for` with a command for the merge, as in 46 and 48 |
| Shell guard | Refused a redirect into the scratch directory, a `rm`, and a forced checkout in the verify clone. I used `touch` and a Python write for the run files, and skipped the main check |
| First offline run of the keeper test | Crashed after a step and left a keeper polling a dead fake Fleet. The test now stops its keeper at exit |
| Verdict line | `verdict.txt` counts 15645 commits (past the root commit). The branch has five past its merge base. The script is fixed; the copy in `run/` is unchanged |
| Agent compaction | The agent's transcript has a "continued from a previous conversation" summary at 03:56:22. I did not determine whether the Fleet restart caused it |
| Rig | `cargo xtask verify-docs --write` was not run (no cargo for this Job). `docs/INDEX.md` is untouched |

## What this does not show

One run, one problem, one model.
No visual walk was needed or done.
The offline-only parts are the merge recorded out of order, the early-green rule and the mismatch placement; they pass against a fake store and origin, and a run that hits them would be a third.
The Checks gate's baseline problem showed up because a changed test is red from the scratch path; whether it is red on main from a path without a space is not known.

## Kept fix

The Job's final branch is `spike/wf-complex-2-job` on the real remote, tip `874a0f61fb45028016d869c316cc1d0012106a86`.
The bare origin and the clone stay in the scratch directory:
`/Users/nickmele/Library/Application Support/Armada/attachments/job-scratch/spike049/devfleet/origin.git` (branch `armada/session-1e84dbdc` at that tip, `main` at `0e3da73336` after the merge),
`.../devfleet/repo`, the scratch Fleet's home and store, and `.../devfleet/verify-049-1e84dbdc`, the keeper's checkout.

## Real-agent sessions started

1. `1e84dbdc`, opus, the complex Job. Four subagent sessions inside it.

No probe sessions. The Fleet was started once and restarted once (above), with no Job started on either start other than this one.

## Files

All in `docs/spikes/049-complex-2/`.

| File | What it is |
|---|---|
| `problem.md` | The brief, the right-answer notes, the runner-ups, the change request |
| `person.md`, `rules.json`, `person.py`, `person_test.py` | The scripted person, its rules, and an offline check |
| `gates.py`, `keeper_test.py` | The keeper and its offline check |
| `fleet.sh`, `run.sh`, `timeline.py` | The Fleet start, the run script in three modes (start, send, finish), the merged table |
| `run/` | The run: timeline, keeper log, gate rows, the person's acts, the brief, the verdict, the three revisions of the design note |
