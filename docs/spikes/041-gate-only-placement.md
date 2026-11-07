# Spike 041: If Fleet had placed a Job only on gates it saw close, how many disputed placements would have been right?

Measured on 6 Nov 2026 from the owner's store, read only. The replay is `docs/spikes/041-replay.py` and its output is `docs/spikes/041-output.txt`. Nothing here lands on main.

## Conclusion

Placing a Job only on gates would have removed most of the disputes and would have put most Jobs on the wrong step. Both halves are in the numbers below, and the second half is the bigger one.

Of the 205 disputed notices, 50 are plan task marks, which no gate could have sent, and 23 more are edit nudges. Gate-only sends none of the task marks and none of the edit, test, push or turn end nudges, because each rests on a fact that is no gate. What it can still send is the "you are on its X step" notice, 78 of the 205.

| | Strict | Strict + own push | Strict + own push + a later gate passes the steps before it |
|---|---|---|---|
| Notices gate-only would not send, of 205 | 157 | 162 | 192 |
| Disputes avoided (not sent, or sent and right), of 205 | 103 | 99 | 142 |
| Disputes whose reason names a step: step shown right, of 133 | 41 (31%) | 37 (28%) | 77 (58%) |
| Same, by hand on 42 labelled | 12 (29%) | 11 (26%) | 21 (50%) |
| For comparison, the old rail: on the same 133, and by hand on the 42 | 26 (20%) | 9 (21%) | |
| Control Jobs held back on a step, of the 92 with no dispute | 74 | 71 | 45 |
| Of the 73 control Jobs whose pull request merged, still showing a step at the end | 73 | 70 | 49 |

The third column is not one of the two variants the brief asked for. I added it because the first two cannot leave a step whose end is no gate, so they hold nearly every Job on its work step for good. It adds one rule: a gate that closes on a later step passes every earlier step except a walk. That rule is itself a placement by inference, a small one.

Three results to carry away.

1. On the disputed placements strict gate-only is a little better than the old rail (41 right of 133 against 26), but the disputes are the old rail's worst moments by construction. The test is the Jobs nobody disputed. There strict ends 74 of 92 held back on a step, including all 73 whose pull request merged. Refine, Implement, Fix and Tests end on a push, a walk shown or a Check run, none of which is a gate under the brief's list, so a Job that reaches one stays.
2. A push cannot be the gate that ends a work step. On the push disputes strict shows the right step on 13 of the 26 that can be judged and the push variant on 6, because the agent pushes work in progress to its own branch and gate-only reads that as the end of the work.
3. The walk is the other gate that fails. A walk that nothing would show is passed over by the old rail on a plan flag and a finished task list, neither of which is a gate. In the control group that holds 33 of the 43 Jobs the third variant leaves on a walk step, because no window was ever shown.

## What the population is

The brief says 205 disputes across 73 of 165 Jobs. The split is a little different.

| | Count |
|---|---|
| Rail disputes | 205, across 126 sessions |
| On the 165 rail Jobs whose `session_rail.visits` is non-empty | 139, across 73 Jobs |
| On sessions whose `visits` column is empty but whose timeline holds their steps | 63 |
| Task marks on plain Sessions that follow no workflow | 3 |
| Sessions with a step history on the timeline (`session.step` entries) | 300, of which 298 rebuild to a workflow version |
| Control: rail Jobs with `visits` and no dispute | 92 (the brief's), 176 counting all 300 |

Those 135 sessions have no `visits` but do have `session.step` entries on the timeline, and for the 164 Jobs that have both, the two give the same step sequence on 157. I read the timeline, so the replay covers all 202 workflow disputes, not 139. Two Jobs changed workflow part way (`17764ae1`, `859c7ae0`) and could not be rebuilt, so 2 of the 205 are left out of every count as `U`.

## Method

For each of the 205 notices:

1. Its time is the `at` of the notice's own row in `session_rows`, not the dispute's `at`.
2. Its Job's workflow is the one named in the notice text or `session_started`. The workflow file in force is the latest `git log` version of `.armada/workflows/<name>.json` before the Job started whose step labels contain every label the Job's rail named. Every version since 4 Oct is available. The step flags are read as `core-model` reads them: a step records the plan when its product is `plan`, and so on.
3. Each step's end is a port of `rail_ends::ends_of` (`crates/fleet/src/rail_ends.rs:141-170`, with `entry` at `:193-205`). Gate-only holds the Job on the first step whose end it has not seen close, and past the last step a Job is done.
4. What counts as a gate, by variant:

| Fact | Where the replay reads it | Strict | + own push |
|---|---|---|---|
| Plan approved | Fleet's own thread line "The owner approved plan vN", and `session_plan.approved_at`. The newest recorded version must be the approved one, so a plan recorded again waits again | yes | yes |
| Walk approved | The person's "Approved <address>" message, with every window shown since the plan's approval approved, newest of each page | yes | yes |
| Sketch, draft, document approved; an idea picked | "Approved sketch:" message, `draft` rows with `settled`, `artifact` attachments with `settled_at`, ask answers to an option with a preview | yes | yes |
| Pull request opened, merged | `pr.opened`, `pr.merged` on the timeline, for the pull requests the Job's ledger holds | yes | yes |
| Checks green | The latest run of every Check in `session_check_runs` passed, or `pr.checks` "Checks passed" | yes | yes |
| Push to the Job's own branch | `session.pushed`, kept when its branch is the Job's own (named for it, or the head of its pull request), and only at or after the last walk Approve where a walk step comes before | no | yes |
| Not a gate in either | edit, commit, walk shown, plan recorded, a reply, turn end, tasks finished, an open task mark | no | no |

5. Whether gate-only would send the notice. Only a "you are on its X step" notice can be sent, and only when gate-only holds the Job on exactly step X at the notice's time. The edit, test, push, turn end and task mark notices each rest on a fact that is no gate, so none is sent.
6. Whether the step gate-only shows is right. The truth is where the agent's `why` says the Job really was, read as a role: plan, walk, build, tests, deliver or done. Tests count as building. The reading was automatic, a pattern judge over the `why` text, and then checked by hand on 60 (below).

Each dispute then falls in one class per variant.

| Class | Meaning |
|---|---|
| A | Gone: no notice, and the step shown is right or the reason names none. All 50 task marks are here |
| B | Gone but the step shown is wrong: no notice, yet gate-only's step is not where the reason puts the Job |
| C | Sent and still disputed: the same step again, and the reason says otherwise or disputes something else |
| D | Sent and right |

## Results, by variant

Counts are disputes. "Avoided" is A plus D, "still wrong" is B plus C.

Strict:

| Notice | n | Avoided | Still wrong | Notice still sent (C and D) |
|---|---|---|---|---|
| On its X step | 78 | 10 | 67 | 46 |
| Pushed while tasks open | 32 | 19 | 13 | 0 |
| Edited before the plan | 23 | 21 | 2 | 0 |
| Edited a test before the walk | 4 | 1 | 3 | 0 |
| Turn ended on a step | 18 | 2 | 15 | 0 |
| Task marks | 50 | 50 | 0 | 0 |
| Total | 205 | 103 | 100 | 46 |

Strict + own push:

| Notice | n | Avoided | Still wrong | Notice still sent (C and D) |
|---|---|---|---|---|
| On its X step | 78 | 13 | 64 | 41 |
| Pushed while tasks open | 32 | 12 | 20 | 0 |
| Edited before the plan | 23 | 21 | 2 | 0 |
| Edited a test before the walk | 4 | 1 | 3 | 0 |
| Turn ended on a step | 18 | 2 | 15 | 0 |
| Task marks | 50 | 50 | 0 | 0 |
| Total | 205 | 99 | 104 | 41 |

Strict + own push + a later gate passes the steps before it:

| Notice | n | Avoided | Still wrong | Notice still sent (C and D) |
|---|---|---|---|---|
| On its X step | 78 | 49 | 28 | 11 |
| Pushed while tasks open | 32 | 19 | 13 | 0 |
| Edited before the plan | 23 | 18 | 5 | 0 |
| Edited a test before the walk | 4 | 1 | 3 | 0 |
| Turn ended on a step | 18 | 5 | 12 | 0 |
| Task marks | 50 | 50 | 0 | 0 |
| Total | 205 | 142 | 61 | 11 |

Each table leaves out one on-step and one turn end dispute (`U`, a Job that changed workflow).

Strict and the push variant send most of the on-step notices again, and not because they place Jobs well: a Job stuck on Implement or Discuss is still on the step the notice names. Of the 78 on-step disputes, 46 and 41 are sent again on the same step, and for 43 and 38 of them the agent's reason says the Job is somewhere else, typically "merged as #2874, there is nothing left to implement".

Where the old rail was right and gate-only is wrong (the "new wrong" column the brief asked for), the disputes show almost none: 0 for strict, 0 for the push variant and 4 for the third. That count is small because the old rail was disputed by construction. The control group is the real test.

### Where the old rail's placement was right and the claim was wrong

20 of the 22 edit nudges that could be judged sit on a Job whose placement the old rail got right (the Job really was on its plan step, or the edits really did follow an approved plan). The dispute is with the nudge's claim, "you edited before the step", which is not a placement at all. 23 of the 205 are this kind. Gate-only would not have made the claim, and would not have changed the placement either.

## The control group

The 92 rail Jobs with no dispute stand in for "the old rail got it right". That is generous to the old rail, since an agent does not dispute every wrong notice and nobody disputes a wrong step shown on the row.

| Variant | Same step as the old rail at the end | Held back on an earlier step | Ahead of the old rail |
|---|---|---|---|
| Strict | 16 | 74 | 2 |
| Strict + own push | 18 | 71 | 3 |
| Third variant | 42 | 45 | 5 |

What holds the 74, 71 and 45 back, by the first step whose end is no gate.

| Variant | Held on |
|---|---|
| Strict | plan recorded 28, a walk to be approved 19, walk shown 10, push 11, a walk approved 5, plan approved 1 |
| Strict + own push | plan recorded 28, a walk to be approved 19, walk shown 10, push 8, a walk approved 5, plan approved 1 |
| Third variant | a walk to be approved 43, a step with no end 2 |

The 28 held on "plan recorded" are all the 27 Oct to 2 Nov feature workflow, whose Discuss step ends when a plan is recorded, an act of the Job.

Over the control Jobs' life, gate-only is on an earlier step than the old rail for 59% of Job hours (strict), 57% (push) and 32% (third), across 191 Job hours with gaps over six hours left out.

Why the third variant still holds 43 Jobs on a walk: 33 never showed a window (nothing to see, the walk is passed over on the plan's flag), 4 showed windows and none was approved, and 6 showed several and approved some. The 6 may be approvals my reading missed. The 33 are not: no gate closes on a walk nobody needed.

Counting all 176 Jobs without a dispute, the same figures are 118, 110 and 49 held back.

## How good the automatic judgement is

The judge reads the agent's `why` and says which step the Job was on. I labelled 60 by hand, 30 drawn at random (`random.Random(41)`), then fixed the judge's misses, then drew 30 more (`random.Random(42)`, excluding the first 30) and froze the judge before reading them.

| | First 30 | Second 30 |
|---|---|---|
| Disputes that are task marks, and so name no step | 9 | 8 |
| Disputes where the hand reading names a step | 20 | 22 |
| Judge matches the hand label exactly | 22 on the first version, 29 on the second (no longer independent) | 24 |
| Outcome class the same as by hand, strict | 30 of 30 (not independent) | 26 |
| Same, push | 30 of 30 (not independent) | 26 |
| Same, third variant | 30 of 30 (not independent) | 28 |
| Step shown is right by hand: old rail, strict, push, third | 4, 4, 5, 9 of 20 | 5, 8, 6, 12 of 22 |

On the independent 30 the judge was optimistic about avoided disputes: by hand, strict avoided 16 and the third variant 20; by the judge 18 and 22. So read the 103, 99 and 142 as about 7 points high (2 in 30), and the 133-dispute right counts as close to the hand figure (29%, 26%, 50% by hand against 31%, 28%, 58% by the judge).

On the second 30 its misses were a finished Draft step read as a plan (row 12), a Job working under an approved plan whose later version was recorded read as a plan (row 29), and three reasons it returned nothing for (rows 13, 18 and 22). It never turned a hand "none" into a step.

The 60 hand labels are in `HAND` and `HAND2` in the script, each with a few words on what the reason said. They are my reading of the agent's words, not the owner's.

## Gates the shipped workflows need that Fleet cannot see

`rail_ends::End` lists every end (`crates/fleet/src/rail_ends.rs:13-33`). `ends_of` (`:141-170`) and `entry` (`:193-205`) choose which applies to a step, and `passed` in `crates/fleet/src/rail.rs:346-371` is the placement rule beside them.

| End | Defined | Store fact today | A gate Fleet saw close? |
|---|---|---|---|
| Plan approved | `rail_ends.rs:15` | `session_plan.approved_at`, which keeps the newest version's approval only. The earlier versions are in Fleet's thread line "The owner approved plan vN" | Yes, with the thread line |
| Walk approved (all, or any) | `:16`, `:19`; read at `rail_keeping.rs:135-160` | A person's message "Approved <address>" (`walking.rs:22`) beside the window row. A dismissed window is in `session_waiting_dismissed` | Yes. A walk replaced or dismissed is read from other tables |
| Sketch approved | `:20` | A person's message "Approved sketch: " (`sketching.rs:203`), no structured row | Yes, by text prefix |
| Draft approved | `:21` | `draft` rows with `settled.how` approved or fixed. 13 rows in the store | Yes |
| Idea picked | `:22`; `rail_keeping.rs:162-191` | An `ask` row whose option has a preview, and the `:said` answer message | Weakly, parsed from the answer text. 60 asks carry a preview |
| Document approved | `:23` | `ledger_attachments` `artifact` with `settled='approved'` and `settled_at` | Yes |
| Pull request merged | `:14` | `pr.merged` on the timeline, `ledger_attachments` `pr` state | Yes |
| Pull request opened | `:30` | `pr.opened` on the timeline, which can trail the real open by seconds | Yes, late |
| A Check run | `:28` | `session_check_runs`, `pr.checks` | A run, yes. Green is a rule over the latest run of each Check |
| Pushed | `:25` | `session.pushed`, with a branch on 268 of 474 pushes | Partly. 206 pushes carry no branch, so a Job's own cannot be told from a subagent's |
| Committed | `:29` | `session.committed` | No. 10 rows in the whole store |
| Edited | `:27` | `session.edited` | Seen, not a gate |
| Walk shown | `:26` | `session.walked`, window rows | Seen, the Job's act, not a gate |
| Plan recorded | `:24` | `session_plan.recorded_at` | Seen, the Job's act, not a gate |
| Replied | `:17` | The person's messages | Seen, but a reply is not an approval |
| A lifecycle state (`job: landed`) | `:33` | `lifecycle_instances`, `lifecycle_moves` | Seen by Fleet and left open in this replay. Only orchestrate's Oversee uses it |
| Tasks finished | `rail.rs:346` (`passed`), `:357-358` (`follows_plan`) | `session_plan_tasks` as listed by the agent or marked by Fleet from edits | No. The 50 task disputes are this |

Of the 31 steps in the nine workflows as shipped today, 11 end on a gate under the brief's list and 20 do not. The 20 are bug Tests, code_review Read the diff, Assess and Deliver the review, design_plan Draft and Present, feature Refine, orchestrate Assess, Dispatch, Oversee and Summarise, prototype Build and Write up, refactor Restructure, revert Undo the change, and all five uat steps. The list from the replay is in `041-output.txt`. In the three workflows used most, bug, feature and refactor, the one step with no gate is the work step.

## What contradicts "Fleet should move a Job only on gates it saw close"

1. A work step has no gate. Under the brief's list a Job that reaches Refine, Implement or Fix never leaves it, so the hypothesis cannot hold as stated. The version that works needs "a later gate passes the earlier steps", which is the rail's own pattern of reading a step from what came after it.
2. A walk with nothing to show has no gate. 33 of the 92 control Jobs never showed a window. Gate-only holds them on the walk after they merge, while the old rail passes them over.
3. A gate can close a few seconds after the notice. `pr.opened` came 28 to 42 seconds after four turn end notices whose agents said the pull request was already open. Of the placements gate-only gets wrong, 8 of the third variant's are right ten minutes later. That is lateness, not error, but a row showing the wrong step for a minute is one an agent can dispute.
4. The push is not a gate. On the 26 push disputes that can be judged, strict shows the right step on 13, the push variant on 6, the third variant on 13 and the old rail on 2. The agents say the pushes were work in progress, and a rule that moves a Job on at a push moves it on at those.
5. Some disputes are about content. 8 of the 78 on-step reasons name no step: the owner stopped the Job, or the instruction is what is disputed ("the step says write no tests yet, the brief says a failing test first"). A gate-only notice carries the same instruction, so these persist whenever a notice is sent.
6. Lost: a true skip report. Gate-only cannot say "you edited before the plan was approved", because an edit is no gate. This data cannot say how many of the 23 edit nudges were right about the skip, since only disputed notices are in it.

## What this replay cannot tell

1. Whether the agents were right. A dispute is the agent's word. Where the store has no fact either way, the judge and I took the agent at its word, and an agent that disputes can be wrong.
2. How a gate-only Fleet would behave on Jobs that were never disputed. The control group assumes no dispute means a right placement, and an agent does not dispute every wrong one.
3. The dispute rate of notices gate-only would send that the old rail did not. It would send an on-step notice at every gate it saw close, including ones the old rail never sent. Their dispute rate is not in this data.
4. The old rail's code changed during the 22 Oct to 7 Nov window (walk before test on 27 Oct, Deliver held behind a walk on 30 Oct, one approval moves a Job on on 3 Nov). The "old" placement is whatever ran then, not the rail as it stands now. Commit 6e6d9ea62f removed the notices on 5 Nov, so the disputes end there.
5. The step a Job was on is rebuilt from `session.step` timeline entries. Where those lag the rail they inherit the lag.
6. Approvals are read from message text and rows. A replaced or dismissed walk (the rail reads `session_waiting_dismissed` and the newer-window rule) is approximated by the newest window of each page. That could account for the 6 control Jobs held on a walk with some windows approved.
7. A person's skip or jump press (two Jobs) is a gate Fleet saw, and the replay does not read it.
8. Only the plan approvals Fleet announced in the thread are known for versions before the newest. A Job whose thread line is missing reads as unapproved.
9. The judge. See the table above: on the independent 30 it names a different step than I do on 6 of the 22 step disputes, and it is more generous than I am about avoided disputes.
10. The workflow version per Job is the one in force at its start whose labels fit. A Job that outlived a change of workflow file is placed on the old steps.

## Reproduce

```bash
python3 -I docs/spikes/041-replay.py > docs/spikes/041-output.txt
```

It reads `~/Library/Application Support/Armada/armada.db` with `mode=ro` and the repository's `git log`, and writes nothing. A different store or a later `git log` gives different counts.
