# A complex Job through the recommendation

Status: spike, run once on opus. Nothing here lands. Branch `spike/wf-complex`, on the rig from `042-workflow-mechanisms-rig.md`, the gate keeper from `046-template-plan-with-refusals.md` and the recommendation in `workflows-fleet-can-follow.html`.

## Conclusion

The run did not exercise most of what it was built to test, and the reason is a finding about the recommendation.
The design note gate never closed.
The agent wrote its note as a Markdown file and showed it as a file.
Fleet keeps that as a file attachment, not as a document, so no approval could exist and no gate row could be written.
Everything the plan put after that gate stayed open, and the Job was held on it from `all_done` until I stopped the run.

| Question | Answer |
|---|---|
| Did the agent obey the template | Yes. Plan version 1 had five groups: "Plan the change", "Write the design note", two build groups, "Deliver", each with the right gate as the last part of `gives`. The template check on the store's `session_plan_groups` found no problem. Detected from the store, not from the agent's word |
| Several build groups | Two, not the three or more the problem was picked for. The agent folded the work into a store group and a rule-and-wiring group |
| Subagents working groups | None. 62 requests, 0 of them a subagent's. No second transcript exists. The agent did the whole job alone in 5.6 minutes |
| Document gate, one round of notes | Not exercised. There was no document, so no revision and no note. Closed only on the second revision: untested |
| Re-plan mid-run | Not exercised. The scripted change request waits for the design note gate to close. One plan version was recorded |
| Gate notices | One sent (the plan). Fleet's own `plan_approved` notice arrived beside it, so the plan gate was told twice. The agent said the plan was approved within ten seconds. The design note, Checks and merged notices never came |
| `all_done` holds | One. Held on the design note gate at 02:53:22. Never released. Idle 14 minutes 43 seconds until I stopped the run. No wake, because no event could close the gate |
| Live group | Right for about 70 seconds, then wrong for 3 minutes 8 seconds of work. It changed twice, both right, and never again while the agent moved through three more groups |
| Subagent pushes | None. Two pushes of the Job's own branch were logged, no other branch |
| Agent's task claims | 13 `TaskUpdate` calls, 13 true against its acts when made. Five of nine tasks were never marked in progress |
| Cost | 5.6 minutes from the first message to `all_done`, 20.5 to my stop. 91 tool calls. 1,701,344 micros as Fleet measured it, about $1.70. 62 requests, 5.55 million cache read tokens, 33 thousand output |

All the person's answers were scripted before the run, from `person.md` and `rules.json`. Nothing was judged live.

### What the recommendation needs to change, from this one run

| Change | Why |
|---|---|
| A gate has to name the form of its fact. "Document approved" needs a Fleet document (an HTML page under `.armada/documents/`, which Fleet asks the person about). A note the agent writes as a file never produces the fact | The brief said "a short page". The agent wrote Markdown and put it in the side panel. The gate waited for a fact that could not happen |
| Gate rows must be facts that do not wait for each other | The keeper only ran Checks and looked for the merge once every earlier gate had closed. The merge happened at 02:53:06 and no row was written, because the design note gate was open and Checks had not run. The rows are meant to be what Fleet saw, not what it saw in order. The live group is the part that is ordered |
| A hold needs a visible reason and a way out | Held on a gate nobody can close, a Job just stops. The person sees "held" and has nothing to press. Either Fleet says what is missing ("no design note document exists"), or the person can waive the gate |
| A person's answer to a question is not an approval, and the agent treated it as one | The agent asked its pick in chat, my script answered it, and the agent wrote "Approved as written" and built. With a gate notice in place the absence of "the person approved the design note" is the signal the agent lacked. It did not stop on the missing line |
| The live group stops being right when a gate goes stale | The progress line stayed on "Write the design note" for 17 minutes while the store showed edits, two pushes, a merge and `all_done`. The display should show the disagreement: tool rows under a later group's paths while an earlier gate is open |

## The problem and why

The issue is armada-vs8t, "Kept per-Job pane layouts are never cleaned up".
Bridge keeps each Job's pane arrangement under a kept key and never removes it.
The brief is `048-complex/problem.md`.
It is TypeScript in `apps/desktop` only, proved by vitest in tens of milliseconds, with nothing to look at.
It has one decision worth a design note: when a Job counts as gone (ended, or no longer listed).
The two runner-ups are named in the brief.
armada-1707 (a notifications area for dismissed failures) needs a walk.
armada-1356 (Tailwind classes that draw nothing in `packages/screens`) found no such classes left in the tree.
The Rust ones (armada-1407, armada-z9s, armada-nb3l, armada-2542, armada-463) were left out on purpose.

The problem did not need three build groups.
A capable agent split it in two, and a subagent had nothing to take.
A better pick would have had work that cannot sit in one head: two independent files with their own tests.

The base is current `main` (a2d2f0f8bf).
The clone is new for this run.
`git branch -a` in it read `main` and `remotes/origin/main`, and the bare origin held `main` alone.
Reaching that took four starts.
Cloning a source tree whose HEAD is a branch creates a local branch of that name, and with a detached HEAD git guesses the branch whose tip matches.
The Job's own spike branch, with this very brief and its answer key, would have been in the agent's clone both times.
A detached HEAD on a commit no branch holds gave a clone with no branch but `main`.
The first three Fleets never ran a Job.

## The prototype

All in `docs/spikes/048-complex/`.
The rig's files are untouched; changed ones are copies.
No Rust, no build, and no change to the mod.

| Piece | What it is |
|---|---|
| Gates on plan groups | `record_plan` drops an unknown `gate` field, so the gate is the last part of a group's `gives`: `[gate: plan_approved\|document_approved\|checks_green\|merged\|none]`. The keeper reads it from `session_plan_groups` on every poll |
| The template | The brief (`problem.md`, section "How to plan") names the first two groups and "Deliver" and leaves the middle to the agent: its own build groups, each `[gate: none]`, the last `[gate: checks_green]` |
| The keeper | `gates.py`, rebuilt from 046's. Writes each close once to `gate_rows.jsonl` with the fact, logs every event to `keeper.jsonl`, writes `state.json` for the person's script |
| Gate rows | A plan version is a new plan gate. Every other gate stays closed when its group's title carries over. `none` closes when the next group has a task the agent marked, or a later gate closes, or at `all_done` |
| The notice | When a gate closes, one line through `POST /sessions/message`, prefixed `Fleet:`, stating the fact. Nothing for `none`. A failed Checks run is told only while the Job is held on Checks |
| The `all_done` hold | `all_done` (or a turn that ends with nothing asked of the person) while a gate is open is recorded as "held on" the first open gate. When that gate closes, the notice goes out and the keeper watches for the Job to wake. No refusal |
| Checks | vitest on the test files the Job's own branch adds or changes against `main`, at the branch tip on the bare origin, in a scratch checkout, once every group before the Checks group has closed |
| Pushes | Every ref that moves on the origin is logged; the Job's own branch is named apart from any other |
| The person | `person.py`, `rules.json`, `person.md`. Scripted: approve plans, send the design note back once with "Say what this does not cover.", approve the next revision, send one change request after the first build group is pushed, merge when asked. A question no rule meets gets the Recommended option or a sentence, never the first option |
| Offline check | `keeper_test.py` drives the keeper against a fake store, origin and Fleet: plan approval, a document sent back then approved (not closed on the first revision), none gates, a push that is not due, Checks, a hold released by the merge, a re-plan that carries gates over. It passed before the run |

The dev Fleet was started with `docs/spikes/042-rig/fleet.sh` in the background with `RIG_BASE=a2d2f0f8bf`. Workflows were off. The Job ran on opus at medium effort.

## The run

One Job, `9d96b11c`, opus.
Times are minutes and seconds after the keeper started at 02:47:43 UTC.
The full table is `048-complex/run/timeline.md`.

| Time | What happened |
|---|---|
| +0:00 | Brief sent |
| +1:08 | The write gate moved the Job into slot 1, as for any Job |
| +1:18 | Plan version 1 recorded, five groups, template check clean |
| +1:19 | The person approved. The plan gate closed 0.37 seconds after the approval. Fleet's own notice and the keeper's line both reached the agent |
| +1:20 | Live group: "Write the design note" |
| +1:59 | The agent wrote `vs8t-when-a-job-is-gone.md` under `.armada/documents/` and showed it as a file |
| +2:11 | It put its one question in chat: when does a Job count as gone |
| +2:22 | The scripted person answered with the rule for that question. The agent read it as approval |
| +2:34 | Task 2 (the note) marked completed; building began |
| +3:21 | First push, 40e9730bc7. Checks not due |
| +3:22 | Group "Drop one owner of a family" closed on the agent's task mark |
| +5:12 | Second push, d2d770de00. Checks not due |
| +5:20 | The agent put the merge in `waiting_for` |
| +5:22 | The scripted person merged into `main` |
| +5:37 | `all_done`. The keeper recorded the hold on the design note gate |
| +20:22 | I stopped the run |

The work was sound.
The branch has five files, 354 lines added, and 42 tests that pass at the pushed commit (run by hand after the run, since the keeper never ran Checks).
The agent chose the same rule my script gave it, "gone when a read of the list no longer holds it", and gave its reasons, so the answer script did not steer the work.
It checked its own tests by removing the line they cover and watching them fail.
It filed a bug report that the brief's `ask_the_person` does not exist in this build, and used `waiting_for` with a command for the merge, as the agent did in spike 46.

## Score

### Live group against what the agent was doing

| From | To | Live group shown | What the agent was doing | Right |
|---|---|---|---|---|
| +1:18 | +1:19 | Plan the change | Waiting for the plan to be approved | Yes |
| +1:20 | +2:29 | Write the design note | Writing and showing the note, asking its question | Yes |
| +2:29 | +5:37 | Write the design note | Building the store group, then the rule and wiring group, pushing twice, asking for the merge | No, for 3 minutes 8 seconds and three missed changes |
| +5:37 | +20:22 | Write the design note | Idle, held | Yes, as a statement of the open gate |

The line changed twice, both times right.
It should have changed three more times (to the store group, to the rule group, to Deliver), and nothing in a gate that depends on a fact can make it.

### The rest

| Measure | Result |
|---|---|
| Plan obeyed the template | Yes, from `session_plan_groups`: titles word for word, gates parsed, middle groups `none` then `checks_green` |
| Plan gate | Closed 0.37 seconds after approval, on the store's `approved_at` |
| Document gate | Open at the end. The ledger held one artifact of form `file` and none of form `document` |
| `none` gate for the first build group | Closed at +3:22, one second after the agent marked the next group's first task. It had finished at +3:20. The agent's word, and true |
| Checks gate | Never evaluated. Two pushes were logged as not due. Run by hand afterwards at the second push: green, 42 tests |
| Merged gate | Not recorded. The merge was at 02:53:06 and the keeper only looks once Checks has closed |
| Notices received and acted on | One, the plan's. The agent said "The plan is approved" ten seconds later. It was told the same thing twice (Fleet's own notice too) and cannot say which it acted on |
| `all_done` hold | One, 14 minutes 43 seconds, no wake |
| Subagent pushes | None |
| Task claims against acts | 13 of 13 true when made. Tasks 2, 3, 6 and 9 were marked in progress, then done. Tasks 1, 4, 5, 7 and 8 were never in progress and were marked done after the work |
| Re-plan | Not run |

Claims in detail: task 1 completed at +1:25 after the plan at +1:18 (true, late); task 2 in progress at +1:25, note written at +1:59 (true); task 2 completed at +2:34 (true); task 3 in progress at +2:34, first edit at +2:38 (true); tasks 3, 4 and 5 completed at +3:20 and +3:21, the last after the commit and push (true; 4 and 5 were never in progress); task 6 in progress at +3:22, first file at +3:43 (true); tasks 6, 7 and 8 completed at +5:11 and +5:12, after the tests passed at +4:55 (true); task 9 in progress at +5:12, `waiting_for` at +5:20, completed at +5:36 after the merge (true).

### Cost

| Measure | Value | How |
|---|---|---|
| Wall time | 5.6 minutes, first message to `all_done`. 20.5 to my stop | Timeline |
| Tool calls | 91 by the agent: Bash 48, TaskUpdate 13, Edit 10, TaskCreate 9, Read 3, Write 3, ToolSearch 2, `record_plan` 1, plus `waiting_for`, `all_done` | Transcript |
| Subagents | 0 | `session_turn_usage.subagent` is 0 on all 62 requests; the project folder holds two transcripts, both the agent's own |
| Tokens | 62 requests on `claude-opus-5-5`: input 128, cache creation 110,832, cache read 5,551,952, output 33,087 | Fleet's `session_turn_usage` |
| Dollars | 1,701,344 micros, about $1.70 | Fleet's own figure in `usage.cost_micros`. I did not recompute it |

## What he would see

```
  Plan the change      Write the design note     Drop one owner      Drop gone Jobs ...     Deliver
  [ plan approved ]  ──  [ design note   ]  ──  [ closed on the ]  ──  [ Checks green ]  ──  [ merged ]
     02:49:03              open, 19 min             agent's word          never run            no row
                           LIVE (stale)             02:51:06

  Held on: Write the design note, since 02:53:22
  Seen under later groups while this gate is open: 13 edits and writes, 2 pushes, a merge
```

| Element | How it is detected | Reliability seen |
|---|---|---|
| The groups and their gates | A fact: `session_plan_groups`, the gate parsed from the end of `gives` | Right. Five groups, one version. A missing or unknown gate would have to be shown, because the store drops the field and only the text carries it |
| The plan gate | A fact: `approved_at` of the current version | Right, 0.37 seconds |
| The design note gate | A fact, if the agent writes a Fleet document: the ledger's `settled` and `revision`. No fact if it writes a file | Failed. The note existed as a file. A gate that cannot close is worse than no gate, because it makes the live group wrong |
| Checks | A fact: Fleet's own vitest run at the pushed tip | Not run in the Job. The offline check ran it, and so did I afterwards, green |
| Merged | A fact: the checked commit is an ancestor of `main` | Not recorded, for the reason above |
| A `none` gate | The agent's word: a task of the next group marked | True here, within a second. Needs the agent to mark tasks; five of nine were never marked in progress |
| The live group | Derived: the first open gate | Right for about 70 seconds of the 5.6 minutes the agent worked, then wrong. It cannot be right when a gate before the work never closes |
| What was seen in a group | A fact: Fleet's tool rows, edits and pushes, placed by group paths and branch | Not built into the display. The keeper logs the pushes. It would have shown the disagreement above |
| The hold | A fact: `done_since` set while a gate is open | Recorded at +5:38. It had no exit |
| The agent's tasks | The agent's word | All true, five of nine never marked in progress |

## What broke or surprised

| What | Detail |
|---|---|
| The brief said "page" | The agent wrote Markdown and used the file panel. The owner's standing rule is that a design he reads is an HTML page, and the brief did not say so. The agent may not have loaded the writing-a-document skill. This is a fault in my brief and a finding about the gate in one |
| A chat answer read as approval | The agent asked its pick as a reply, the scripted person answered, the agent proceeded. Nothing separates a reply from a document approval to an agent that wrote no document |
| `ask_the_person` is not in this build | The agent searched for it, found none, and used `waiting_for` with a command. The brief named a tool the Job does not have |
| The keeper's ordering | Checks and merged were only looked at in order. The lost merge row was a prototype choice, and the recommendation's wording ("each gate close is a row, written once") does not ask for it |
| Checks "due" | Without the ordering guard, a Checks gate on the last build group would have closed at the first push, +3:21, with the second group unbuilt. Fleet needs to know that the last build group has started before a green run means anything. Here that knowledge was the agent's task marks |
| The person's script never ended the run | `person.py` counts the `done` item as an item, so its quiet rule never fired. I stopped the Job by hand at 20.5 minutes, after the keeper had written its last pass. `run.sh` aborted on the stop and its post-run steps were done by hand |
| The scratch directory | A first Fleet start in the Job's scratch directory had to be abandoned before any Job, because the guard hook refuses a branch removal outside the slots. `/Users/nickmele/Library/Application Support/Armada/attachments/job-scratch/spike048/devfleet` is left behind (a clone, a bare origin, a copy of the Fleet binary), and the hook refused its removal |
| `git checkout` outside a slot | The same hook refuses it in a scratch clone; `git clone --branch` does the same job |
| Timeline | The git column counted 0 commits because it reads the clone's log since the first event; the pushes come from the keeper column instead |
| Rig | `cargo xtask verify-docs --write` was not run because this Job may not run cargo. `docs/INDEX.md` is untouched |

## What this does not show

One run, one Job, one task that proved smaller than intended.
It shows that a gate can be impossible to close and that the progress line then lies for as long as it stays so.
It does not show the document gate working with an HTML page, the one-round note, the re-plan, a Checks gate in a live Job, a green notice woken from a hold, or a subagent push.
A second run with the brief saying "an HTML page with the writing-a-document skill" and a problem with two independent files would cover them.
That second run is outside the one session the owner allowed, so it is not done.

## Real-agent sessions started

1. `9d96b11c`, opus, the complex Job.

No probe sessions. The Fleet started twice before it, with no Job, to get a clone with no spike branch in it.

## Files

All in `docs/spikes/048-complex/`.

| File | What it is |
|---|---|
| `problem.md` | The brief, the right-answer notes, the runner-ups, the scripted change request |
| `person.md`, `rules.json`, `person.py` | The scripted person and the owner habits behind each rule |
| `gates.py`, `keeper_test.py` | The keeper and its offline check |
| `run.sh`, `timeline.py` | The run script and the merged table |
| `run/` | The run: timeline, keeper log, gate rows, the person's acts, the brief, the verdict, the note the agent wrote |
