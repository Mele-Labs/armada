# A Job that writes its own workflow, as its plan

Status: spike, run once on armada-35xp. Nothing here lands. Branch `spike/wf-own-workflow`, on the rig from `042-workflow-mechanisms-rig.md` and the gate record from `044-template-as-seeded-tasks.md`.

## Conclusion

The mechanism works for the part Fleet can check and says little for the part it cannot.
The agent wrote a sensible two group plan, tagged both gates the way the brief asked, and Fleet closed both gates on the right facts.
Every one of its five edits fell inside the live group's paths, so path placement worked, but the run was too small to test it: one file, one real group.

| Question | Answer |
|---|---|
| Does `record_plan` keep an extra `gate` field on a group | No. The route answers 200 and drops it. The plan read back has no `gate`, and the store has no column for it |
| So where does the gate live | At the end of the group's `gives`, as `[gate: checks_green]`. The agent followed this on both groups. It costs about 20 of the 160 characters `gives` is allowed, and the person would see the tag in the text |
| What did the agent choose | Two groups: name the cause (gate `plan_approved`), then retarget, run, commit and push (gate `checks_green`). It did not pick `merged`, `walk_approved`, `document_approved` or `none` |
| Did the groups' paths place its edits | Five of five edits fell inside the live group's paths. None fell outside every group, none in a later group. Both groups' paths held the file, because group 1 named the whole `mock` directory |
| Did the live group ever disagree with what the agent was doing | Not on its edits. For the 28 seconds before the plan existed there was no group at all, and the record kept group 2 live for 18.7 seconds after the agent had finished |
| Were its task claims true | There were none. It never created a task, though the plan approval notice told it to. Fleet's own plan progress (`worked_at`, a task's `edited`) also stayed empty after five edits under the path |
| Was the gate record ever wrong | No. Plan approved closed 0.96 s after the approval. Checks green closed on the pushed commit `d0551642c9`, 18.7 s after the push |
| Biggest finding about my own side | The tracker ran the Check inside its poll loop. For those 18.7 seconds it saw nothing else, so the commit and the push were logged late, against no live group. The Check has to run beside the loop |
| Biggest caveat about the run | The agent found a finished fix on another Job's branch in the shared clone and wrote the same file, byte for byte. The investigation and the edits are therefore not a fair sample of how it would have worked alone |

## Real-agent sessions started

One. Session `9a3470ee-f894-40a5-b957-3fbe902a460f`, model sonnet, armada-35xp.
No probe session and no rerun.
The route test for the `gate` field and the tracker's smoke test ran against an ended session of spike 42 (`c8e1d57d`), which started no agent.
That session now holds two plan versions in the dev store that nobody asked for.

## Whether the route keeps a `gate` field

Read first, then tried.

| Where | What it shows |
|---|---|
| `crates/ipc/src/session_plan.rs`, `PlanGroupInput` | Fields: `id`, `title`, `agent`, `paths`, `issue`, `gives`, `lands_in`, `own_job`, `tasks`. No `deny_unknown_fields`, so serde ignores a field it does not know |
| `checked()` | Rebuilds each group from those fields only |
| `crates/api/src/sessions.rs`, `record_plan` | Decodes `RecordPlan`, then reads `seen` and each group's `seen` beside it from the same body. A field Fleet wants has to be added there or on the type. `gate` is neither |
| The dev Fleet, `POST /sessions/plan` with `"gate": "checks_green"` on a group | 200. `GET /sessions/one` shows the group without it. `session_plan_groups` has no column for it |

So the field is dropped without a word.
The brief therefore carries the gate in `gives`, as the last thing in the line.
The agent's own `record_plan` call is in the transcript with whatever it sent, so a field would have been readable there, but that is the agent's call and not the recorded plan.
The tracker reads the recorded plan, as asked.

## The tracker

`docs/spikes/047-own/track.py`, started from `044-seeded/gates.py`.
No template is seeded.

| Part | What it does |
|---|---|
| The line | The recorded plan's groups, from the dev store. A group's gate is the tag at the end of its `gives`. The first group is `plan_approved` whatever it says. A group with no tag or an unknown one is taken as `none` and logged |
| Gate facts | `plan_approved`: `session_plan.approved_at`. `checks_green`: vitest on each touched test file that exists at the pushed commit, run by the tracker in a scratch checkout of the bare origin, again at each new tip. `merged`: the tip is an ancestor of `integration-<id8>` on the origin. `walk_approved` and `document_approved`: an approved walk or document in the ledger, written from the table's states and not exercised. `none`: closes on the first edit under the next group's paths that is not also under this group's, or when the Job is done |
| Forward only | Gates close in group order, only the live group's gate is evaluated, and a closed gate is never reopened or re-derived |
| The live group | The first group whose gate has not closed |
| Activity | Read from facts and not from the agent. Edit and Write rows Fleet kept for the Job (`session_rows`), placed against every group's paths as in the live group, in an earlier group, in a later group, or outside every group. Commits on the Job's branch in the clone. The branch tip on the origin as a push |
| Claims | The agent's task list as Fleet saw it (`session_build_tasks`), logged separately with the live group at that moment, the group the task belongs to, and a flag when the task is ahead of the live group or completed with no edit under its group's paths |
| A plan recorded again | The closed gates are matched against the new groups in order by gate kind. The matching prefix is kept, the rest moved to `orphaned`, the live group recomputed, and all of it logged |

Tested before the run on the ended session `c8e1d57d`: a plan approved closed its gate in 0.8 s, a second plan with four groups kept the closed gate and moved the live group to group 2, a `none` group closed because the Job was done, and `checks_green` ran vitest on the branch's pushed commit and closed.
`merged`, `walk_approved`, `document_approved`, and a `none` group closed by an edit were not exercised.

Other scripts: `person.py` (the 044 copy, with one added rule), `timeline.py` (the 044 copy with the tracker's events as source `track`), `run.sh` (the 044 copy; no seeding, the tracker started before the first message), `brief-gates.txt` (the added paragraph), and `track.py show`, which rebuilds the progress display from `track.jsonl` as of a time.

The brief is `044-seeded/yardstick-2.md` unchanged, with `brief-gates.txt` appended.
Workflows were off in the dev Fleet.

## The person's acts were scripted

The person approved the plan the moment it appeared, and would have merged when asked.
For a design question the script scores each option's label and description for words that mean a smaller change (smallest, minimal, only, just, keep) against words that mean a larger one (delete, rewrite, refactor, restore, add, also, all) and takes the highest, the first on a tie.
The agent asked nothing, so that rule never fired.
The only act in the run was the plan approval.
Nobody was asked to merge, because the agent chose no `merged` gate and the brief ends at the push.

## The run

One sonnet Job, 1.7 minutes by the timeline, 22 tool calls.
Another spike ran its own Job on the same Fleet in the same period.
Full table in `047-own/run-35xp/timeline.md`.

| Time | What |
|---|---|
| +0:00 | First message. Tracker running, no plan yet |
| +0:05 to +0:28 | Thirteen read and Bash calls. `armada run bootstrap` twice. Fleet moved the Job into slot 1 |
| +0:28 | `record_plan`: two groups |
| +0:31 | The person approved. Fleet's notice told the agent the plan was approved and that its task list is the plan |
| +0:32 | Tracker closed `plan_approved`, 0.96 s after the approval. Live group is 2 |
| +0:42 to +0:51 | Five Edits on `checks-keys.test.tsx`, each placed in the live group. Detected 0.2 to 2 s after the row's time |
| +0:53 | vitest run by the agent, 2 passed |
| +1:03 | Commit and push (02:14:18) |
| +1:04 | `all_done` (02:14:20) |
| +1:21 | Tracker closed `checks_green`: 2 passed at `d0551642c9`, 9 s of vitest in a cold checkout. 18.7 s after the push and 17 s after `all_done` |

The result: one file, 16 lines added and 12 removed, 2 tests green at the pushed commit by the verdict's own run.

## The workflow it chose

From `047-own/run-35xp/record_plan_input.json`.

| # | Title | Gate | Paths | Tasks |
|---|---|---|---|---|
| 1 | Name the cause and the target | `plan_approved` | `apps/desktop/src/renderer/src/mock` | Confirm job-checks is gone and the checks scenario has Checks tiles. Decide retarget over delete |
| 2 | Retarget the tests, run, commit, push | `checks_green` | `apps/desktop/src/renderer/src/mock/checks-keys.test.tsx` | Mount the checks scenario and use tiles it really has. Run the file until both tests pass. Commit and push to origin |

Was it a sensible shape for the work: yes, for a one file fix.
A plan gate before the edits and a Check at the end is what the work calls for, and it left out a walk, a document and a merge, none of which this Job has.
It named `armada:explorer` and `bridge-engineer` as the groups' agents and used neither.

Three things about the shape.

| What | Why it matters |
|---|---|
| The plan came after the investigation | Group 1 is the stretch that ends at the plan's approval, but the agent recorded the plan 28 seconds in, with the cause already named. The group was nearly over before it existed, so the first group's activity cannot be placed by any plan |
| Group 1 names a directory | It edits nothing, but its paths hold the file group 2 edits. A group that only reads still gets paths, and a directory prefix overlaps its neighbours |
| `gives` ran near its limit | 152 of 160 characters with the tag. A longer sentence plus the tag would have been refused |

## Score

| Measure | Result |
|---|---|
| Groups, gates, paths | Above. Both gates tagged in the right place, and the first one matched what the brief demands |
| Edits inside the live group's paths | 5 of 5 |
| Edits outside every group | 0 |
| Edits in a later group | 0 |
| Edits before the plan was approved | 0. A probe file was written before approval in spike 44. Not this run |
| Edits that sat under two groups | 5 of 5, since group 1's directory holds the file. They count as inside the live group. By the narrowest path they would also go to group 2 |
| Did the live group disagree with the agent | On edits, never. On time, twice. There was no group for the first 28 seconds. After the push, group 2 stayed live for 18.7 s, and the agent had finished 17 s before the gate closed |
| Task claims | None. `session_build_tasks` is empty. The agent ignored "TaskCreate each task word for word" in both the `record_plan` receipt and the approval notice |
| Fleet's own plan progress | `worked_at` empty on both groups and no task `edited` after the run ended. Five edits sat under group 2's path. I did not look for why |
| Gate record | Right twice, and nothing was reopened. Plan approved at 02:13:47.650. Checks green at 02:14:37.273 on `d0551642c9`, the commit the verdict ran |
| Commit and push placement | Logged at 02:14:37, 19 s after they happened, with no live group (it had just closed). Judged by their own times, both fell in group 2. The commit's placement `in_earlier` in `track.jsonl` is the tracker's lateness and not the agent's |

## What he would see

```
  Plan approved ───────── Checks green
    closed 02:13:47          open, live

  Live: Retarget the tests, run, commit, push
    5 edits under its paths, last 02:14:09
    0 commits, 0 pushes so far
    Agent says: nothing (it kept no task list)
```

`track.py show --at` rebuilt this at four times in the run, in `047-own/run-35xp/display-replay.txt`.

| Element | How it is detected | What this run showed |
|---|---|---|
| The line: its groups, in order | The recorded plan | Right. Before `record_plan` there is no line. Show "No plan yet" for the investigation, 28 s here |
| Each gate's state | The facts above, kept in `record.json` | Right twice. 0.96 s for the plan, 18.7 s for Checks. A person watching sees "open" for the push to Checks gap |
| The live group | The first group with an open gate. No word from the agent | Right until the push. After it, the record is a Check behind the agent, which is true to Fleet's facts |
| Activity under it | Edit and Write rows by path, commits from git, pushes from the origin | Edits 0.2 to 2 s behind. Commits and pushes were 19 s late because of the blocking Check, a fault of mine and not of the method |
| An edit that does not fit | Path against every group | Nothing fell outside here. This is the element the method could show that the others could not, and this run never made it appear |
| Agent says | The agent's in progress task | Blank. It made no tasks, so there is nothing to draw. The claim layer has to be allowed to be empty |

What it cannot show: which task in the live group the agent is on, whether a group the agent thinks is done is done, or anything about work before the plan.
Whether the tag in `gives` is drawn to the person or hidden is a choice the display has to make.

## What broke or surprised

| What | Detail |
|---|---|
| The tracker blocked on its own Check | `evaluate_live` runs vitest inside the poll loop, a cold checkout and 9 s of tests. Edits, commits and pushes are read after it in the same loop, so they waited. Found from the timeline. Not fixed and not rerun, since the owner allows one run. A real Fleet runs the Check beside the loop, and the edits are unaffected only because they came before the push |
| The agent copied another Job's fix | `git branch -a --contains` showed `armada/session-2115a55a`, a branch from the spike running on the same Fleet. The agent read `70925db32d`, tried `git checkout 70925db32d -- <file>` (the repository's hook refused it), then edited by hand. `git diff` between its commit and that one is empty. The rig's clone holds every Job's branch, so any parallel run can see the others. The edits are real, the discovery is not |
| It would not keep a task list | Fleet's notice and the plan receipt both say to TaskCreate each plan task. It made none. Spike 44 saw it obey when tasks were already seeded. Here, no seed, no tasks, so no claims to score |
| Fleet's own plan progress did not move | `worked_at` and `edited` stayed empty. If Fleet's own mark worked it would have been a second witness for the placement. Cause not found |
| The gate field is dropped | Above. Not an error, and the agent cannot know |
| Group paths overlap | Placement picks any group whose paths hold the edit and calls it inside if the live one is among them. With a directory on one group the check says yes for the wrong reason. A rule of the narrowest path, or of paths that may not nest, would remove that |
| The agent finished before the gate | As in spike 44. `all_done` came 17 s before Checks closed. Nothing refused it |
| Rig friction | The agent's first write was refused once while Fleet leased the slot, as in the baseline. A `git checkout` of another branch's file was refused by the repository's hook |

## What this does not show

One run, one model, one small task with one file edited, so the placement result is five identical observations.
A plan with several groups, edits that cross a boundary, a plan recorded again mid-run, and a `none` group closed by its next group starting were not seen in the run.
The re-record mapping and a `none` group closed by the Job being done were shown only in the smoke test on an ended session.
The `merged`, `walk_approved` and `document_approved` gates were never exercised.
The agent's own contamination from another branch means its path through the repository is not representative.
Tool rows give Edit and Write paths only. A Bash `sed -i` or a subagent's edit would not show, and the Job did neither.
The Fleet binary is the copy of the owner's newest kept build used in spike 42, and the mod is this tree's `plugins/`.

## Files

All in `docs/spikes/047-own/`.

| File | What |
|---|---|
| `track.py` | The tracker and `show` |
| `run.sh`, `brief-gates.txt` | The run and the added paragraph |
| `person.py`, `timeline.py` | Copies of spike 44's with the changes named above |
| `run-35xp/` | `timeline.md`, `track.jsonl` (every tracker event), `record.json` (the state at the end), `record_plan_input.json` (the agent's call), `display-replay.txt`, `acts.jsonl`, `brief.txt`, `verdict.txt` |

`docs/INDEX.md` is untouched, and `cargo xtask verify-docs --write` was not run, since this Job may not run cargo.
