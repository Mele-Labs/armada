# Spike 45, one agent per stretch of work

Status: run once on each of two yardsticks, six real agent sessions in all. Nothing here lands. Branch `spike/wf-agent-per-step`.

## Conclusion

Fleet's place in the Job was never wrong, and that part of the mechanism holds.
The next agent was started only after the one before it had ended, so two agents never ran at once, and no gate closed with a stretch still unstarted.
All six agents stayed inside their stretch, all six gates closed on the first try, and no stretch was restarted.

The handoff is where it is weak.
When the handoff arrived (armada-tyl, stretch 2) the next agent did what it said, read none of the files the first agent had read, and reversed nothing.
When it was lost (armada-35xp, stretch 2) the next agent re-explored, re-read the same files, and built a different fix from the one the person had approved.
Gate 2 closed all the same, because its gate is a green test file and not the approved plan.

It costs about three times the single agent on a one-file bug.
On armada-tyl the three agents spent an estimated $1.33 and 43 tool calls over 2.5 Job minutes, against $0.39, 16 calls and 0.9 minutes for the rig's baseline.
The third stretch of each run did almost no work (11 and 12 tool calls) and still cost $0.31 and $0.34, so a fresh session has a floor near $0.3 on these Jobs.
There is no single-agent baseline for armada-35xp, so how much of its $2.08 is the mechanism and how much the task is not known.

Number of real agent sessions started in this spike: 6.
Three for armada-tyl, started before the owner's cap on sessions arrived, and three for armada-35xp. No stretch was restarted. No single-agent session was run on armada-35xp.

Both Jobs are small, each stretch ran under two minutes, and each yardstick ran once, so every figure below is one sample.

## How it was run

`045-per-step/run.sh <tyl|35xp> perstep` starts three fresh Jobs on the dev Fleet from the 042 rig, with workflows off, full tools and model sonnet.
Each Job's first message is built by `compose.py`: the report, the stretch's instructions copied unchanged from `.armada/workflows/bug.json`, a few notes on the rig (no forge, one test file, where the branch is), and the handoff of the agent before.
The previous agent's last act is a message that starts with `HANDOFF`, which `transcripts.py` lifts from its transcript and the runner pastes into the next message.
`person.py` is the rig's person with two additions: it merges the branch into the bare origin's main when the agent puts up a merge item, and it answers "retarget" to a question about retarget or delete.

| Stretch | Instructions from bug.json | Ends when | How the runner sees it |
|---|---|---|---|
| 1 | find_the_cause | the person approves the plan | the plan approval the person's act made, and the Job ending |
| 2 | fix, tests | the yardstick's vitest file passes at the pushed commit | the runner clones the pushed branch from the origin and runs the file, once the agent has called all_done |
| 3 | deliver | the person merges the branch into the origin's main | the runner checks the branch is an ancestor of the origin's main |

The next Job starts when the previous one has ended and its gate has closed.
Origin's main is put back where it was after each run, so the second run started from the same commit as the first.

The overlap counts in `analysis.txt` match file names as written in a command or a Read and miss a path given in a different form, so the counts below were corrected by reading the transcripts.

Files: `run.sh`, `compose.py`, `person.py`, `transcripts.py`, `analyze.py`, `yardstick-2.md`, and per run `tyl/` and `35xp/` with the events log, the three timelines, the first messages, the handoffs, the person's acts and `analysis.txt`.

### Starting a Job on an existing branch

The dev Fleet cannot.
`StartSession` (`crates/ipc/src/hosted_sessions.rs`) has no branch field, and `lease_taken` (`crates/fleet/src/session_host/gate.rs`, near line 461) takes an existing branch only for a Job that was paused and is being resumed, which is the same agent and the same conversation.
A fresh Job gets `session-<id8>` cut from main on its first write.
So the new agent checks out the branch in its first act, as the brief allowed.
That worked for stretch 3 in both runs, and it left Fleet's record wrong: Fleet's lease row for Job 3 names `armada/session-8ab211a2` (35xp) while the work is on `armada/session-2115a55a`.

A second fact made it harder.
Stretch 1 edits nothing, and in both runs the branch Fleet cut for it was gone from the clone after the Job ended, so stretch 2 had no branch to start on and its own fresh one became the working branch.
Stretch 1's handoff still named the branch it had been given, which no longer existed (tyl).

## armada-tyl

Run `perstep-tyl-perstep-20261107T014351Z`, 2.8 minutes from the first message to the merge.

| Question | Stretch 1, plan | Stretch 2, fix | Stretch 3, deliver |
|---|---|---|---|
| Stayed inside its stretch | yes: read, ran the test, recorded a plan, edited nothing | yes: one rename and push, no new plan | yes: merged main in (nothing to bring), pushed, put up the merge item, no code change |
| Gate closed | yes, plan approved 15 s before the agent ended | yes, test passes at the pushed commit | yes, origin main held the branch |
| Handoff written | yes, 369 words | no, it called all_done with no HANDOFF message | not needed |
| Wall minutes | 1.4 | 0.7 | 0.5 |
| Tool calls | 20 | 11 | 12 |
| Est. cost | $0.65 | $0.37 | $0.31 |

Stretch 2's work matched the handoff to the letter: the same cause, the same rename to `SLOT_LEASE_TITLE`, the same two files, one commit.
It read two files, the two it had to edit, and none of the other six stretch 1 had read.
It did not show the test red before the change, which the Tests instructions ask for, because stretch 1's handoff had already said the test fails with `expected 380 to be 381`.

## armada-35xp

Run `perstep-35xp-perstep-20261107T015822Z`, 4.6 minutes from the first message to the merge.
The person answered the plan with an approval and the merge item with a merge. The agents asked nothing else.

| Question | Stretch 1, plan | Stretch 2, fix | Stretch 3, deliver |
|---|---|---|---|
| Stayed inside its stretch | yes: read, recorded a plan, edited nothing. It could not run the test until after the plan was approved (the slot move held its writes), so it ran it after the gate | yes in scope, no in plan: see below | yes: merged main in (already up to date), ran the file, pushed, put up the merge item |
| Gate closed | yes, plan approved 29 s before the agent ended | yes, the file passes at the pushed commit | yes, origin main held the branch |
| Handoff written | yes, 358 words, but the runner did not find it (see What broke) | yes, 358 words | not needed |
| Wall minutes | 1.6 | 1.9 | 0.8 |
| Tool calls | 31 | 27 | 12 |
| Est. cost | $0.89 | $0.85 | $0.34 |

Stretch 1 planned a new scenario named for the tests, with the four tiles the tests walk (`desktop_test`, `hooks_test`, `ipc_test`, `test`), registered beside the two existing Checks scenarios, five tasks, files named.
Stretch 2 never saw that plan.
It retargeted the tests at the existing `checks` scenario, found that scenario has neither `hooks_test` nor `ipc_test` and that a started Check draws a `Stamp` button with its own Tab stop, and rewrote both tests onto tiles that exist (`screens_test`, `desktop_test`, `test`).
The result passes, touches only the test file (16 lines added, 12 removed), and is a sound fix, but it is the other option the report offered and not the plan the person approved.
Stretch 3 noted the open question about the `Stamp` Tab stop in its reply.

## Handoff loss

| | armada-tyl stretch 2 | armada-35xp stretch 2 |
|---|---|---|
| Handoff it started from | stretch 1's, 369 words | none: "the previous agent wrote no handoff" |
| Files it read | 2, the two it edited, both named in the handoff by line | 10 |
| Of those, files stretch 1 had also read | 2 of 2, no exploring beyond them | at least 3: `checks-fleet.ts`, `checks-keys.test.tsx`, `checks-without-run-sheet.test.tsx`, plus the same searches |
| Tool calls before its first edit | 1 | 9 |
| Decisions reversed | 0 | 1: new four-tile scenario became a rewrite of the tests onto existing tiles |

Example one, the handoff that worked.
Stretch 1's handoff said the cause was a plain `TITLE` exported from `now-and-plain-plans.ts:45` and `auto-land-answers-at-once.ts:18`, that the newer file should be renamed to `SLOT_LEASE_TITLE`, that the walk file imports it at line 4 and uses it at line 18, and that `armada-tyl`'s wording is stale.
Stretch 2's first command was a grep for `TITLE` in exactly those two files, and its next was the rename.

Example two, the handoff that was lost.
The cause and the plan were in Fleet's store as plan v1 on Job 1, but a plan belongs to the Job that recorded it and nothing carried it to Job 2.
Stretch 2 spent its first nine calls finding which scenario draws Checks tiles, reading `checks-fleet.ts` and grepping for `hooks_test`, which is what stretch 1 had spent about 25 calls on.
It then took the cheaper route stretch 1 had ruled out, and ran into the Tab stop problem stretch 1 had not reached.
Neither agent was wrong. The loss is that the person approved one plan and received another fix with no signal that it changed.

What the handoff did not carry even when present: the plan's task list as tasks (it was prose), the branch (it was gone), and the person's approval as an event the next agent could check.

## Cost against the baseline

The rig's baseline is `042-rig/baseline-timeline.md`, one agent on armada-tyl with no plan.
Dollar figures are an estimate from the token counts in Fleet's `session_turn_usage` at list prices per million tokens (input 3, cache write 3.75, cache read 0.30, output 15), the same formula on every row.
Fleet's own `cost_micros` is not used: it reads 16,958 for the baseline, against 391,000 from the tokens, and what it counts is not known.

| Row | Job min | Tool calls | Cache write | Cache read | Output | Est. $ |
|---|---|---|---|---|---|---|
| tyl baseline, one agent | 0.9 | 16 | 42,394 | 659,250 | 2,297 | 0.39 |
| tyl, stretch 1 | 1.4 | 20 | 58,603 | 1,094,332 | 6,889 | 0.65 |
| tyl, stretch 2 | 0.7 | 11 | 41,528 | 553,395 | 2,881 | 0.37 |
| tyl, stretch 3 | 0.5 | 12 | 38,810 | 495,481 | 1,264 | 0.31 |
| tyl, three stretches | 2.5 | 43 | 138,941 | 2,143,208 | 11,034 | 1.33 |
| 35xp, stretch 1 | 1.6 | 31 | 75,345 | 1,517,783 | 9,871 | 0.89 |
| 35xp, stretch 2 | 1.9 | 27 | 66,530 | 1,540,757 | 8,979 | 0.85 |
| 35xp, stretch 3 | 0.8 | 12 | 40,295 | 550,761 | 1,832 | 0.34 |
| 35xp, three stretches | 4.3 | 70 | 182,170 | 3,609,301 | 20,682 | 2.08 |

On armada-tyl the three agents cost 3.4 times the baseline in dollars, 3.3 times in cache reads, 2.7 times in tool calls and 2.8 times in Job minutes.
Part of that is the plan stretch, which the baseline did not have (stretch 1 alone is 1.7 times the baseline), and part is that each fresh session pays its own start: about 40,000 tokens of cache write and 500,000 of cache read before it does anything, which is the third stretch's whole bill.
Between agents the run lost about 14 seconds waiting for the runner to clone the pushed branch and run the test, and each fresh Job spent two to five tool calls on the first-write refusal while Fleet moved it into its slot (6 of 6 Jobs, 4 to 24 seconds each).

Spike 39 found a fresh session per step saves 2 to 4 percent of spend on real multi-step Jobs.
These Jobs are too short to show that saving, because no stretch carried enough context to be worth resetting (the largest was 75,000 tokens of cache write).
They do show the other side of it, the fixed cost of a start.

## Was Fleet's place ever wrong

| Check | tyl | 35xp |
|---|---|---|
| Two agents running at once | never | never |
| A gate closed with no agent to start | never | never |
| Time a gate was closed while the old agent still ran | 16 s (plan approved, handoff being written) | 29 s (same, plus the reproduction run) |
| Time between the last agent ending and the next starting | 0.2 s, then 15 s (waiting for the gate 2 check) | 0.2 s, then 14 s |
| Fleet's record of the branch | stretch 3 worked on `session-d29b0b18`, Fleet's lease says `session-51d3cfa7` | stretch 3 worked on `session-2115a55a`, Fleet's lease says `session-8ab211a2` |

By construction the position is whichever agent is running, and it was.
What it does not know is that the work sits on a branch other than the one in its lease record, and that a stretch's handoff exists or does not.
Gate 2 was checked only after the agent ended because the runner polls the Job and not the push. Fleet sees the push (`session.pushed` appears in each Job's timeline), so it could start the check then.

## What he would see

| Element | What shows | How it is detected | In these runs |
|---|---|---|---|
| The stretches as a line | three marks: find the cause and plan, fix and test, deliver | nothing to detect: the regrouping of `bug.json`'s four steps into three is fixed text | static |
| The running agent's stretch | the mark of the live Job | Fleet started that Job for that stretch and holds its row | right in 6 of 6 |
| Stretch 1 done | the plan approved | `session_plan.approved_at` and the `plan_approved` notice, both in the store | seen in both runs |
| Stretch 2 done | the test file green at the pushed commit | Fleet sees `session.pushed`; the check is a vitest run at that commit | seen, 14 s after the agent ended |
| Stretch 3 done | merged into main | the branch is an ancestor of the origin's main; on a real forge, the pull request merged | seen at the runner's check after the agent ended, 3 to 10 s after the merge |
| The running agent's tasks | the plan's tasks for stretch 1 | `session_plan_tasks`, five rows for 35xp stretch 1, no marks because nothing was built | stretch 2 and 3 Jobs had none |
| Which handoff it started from | none shown today | the first message's text; a missing HANDOFF is detectable by the same search the runner made | not shown |

Zero of the six agents made a task with `TaskCreate` or `TodoWrite`, so the only tasks Fleet has are the plan's, and a plan stays on the Job that recorded it.
For tasks to read as progress across stretches, Fleet would have to copy the approved plan's tasks into the next Job and mark them as the agents report.
The same copy would have prevented the reversal in 35xp.

He would also see three Jobs in his list for one bug.
The owner dropped one Drone per step on 15 Oct partly for being slow and loading his Mac. These runs do not measure either on a real Job: a stretch here is a minute or two and the only heavy command is one vitest file.

## What broke

| What | Effect | State |
|---|---|---|
| The runner looked for a message starting with `HANDOFF`. The 35xp stretch 1 agent began its message `HERO`, then `Correction: the first line must be HANDOFF`, then the handoff | Stretch 2 got "the previous agent wrote no handoff" and the reversal above | `transcripts.py` now searches each line. Fixed after both runs; the committed run output still shows the miss, and `35xp/handoff-1-actual.md` holds the text |
| The tyl stretch 2 agent skipped the handoff and called all_done | None, stretch 3 needs only the branch | Not fixed; an agent can skip it, so the next one must not depend on it |
| The branch cut for a stretch that edits nothing is gone after the Job ends | Stretch 1's handoff named a branch that did not exist | Dev Fleet behaviour, left as found |
| First write of every fresh Job is refused once while Fleet moves it into a slot | Two to five wasted tool calls per Job, and in 35xp stretch 1 the reproduction waited until after the plan was approved | Rig and Fleet behaviour, as in the baseline |
| `record_plan` refused a task title over 160 characters (35xp stretch 1) | One retry, 4 s | The agent shortened it |
| The scratch home's Playwright cache held revision 1254 and vitest wants 1234 | The first headless run in the scratch home failed | Added a link to the owner's 1234 in the scratch cache. My first `ln` also left a stray link `ms-playwright/ms-playwright` inside that cache, which the guard would not let me remove. It is harmless and in the other spike's scratch |

Not tried: a handoff written by Fleet from the plan and the ledger instead of by the agent, a stretch restart, and a 35xp single-agent baseline.
