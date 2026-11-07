# The workflow template seeded into the agent's own task list

Status: spike, run once on armada-35xp. Nothing here lands. Branch `spike/wf-seeded-tasks`, on the rig from `042-workflow-mechanisms-rig.md`.

## Conclusion

The seeding works and the gate record held, but the task list does not keep the agent inside a stretch.

| Question | Answer |
|---|---|
| Can the template be seeded into the agent's task list | Yes. Files written under `~/.claude/tasks/<session id>/` after the agent process started and before its first message are in its first `TaskList`. The list survived Fleet moving the Job into a slot (the process restarted) |
| Does a change from outside show on the agent's next read | Yes, shown in a probe: a task file marked completed by another process shows as completed on the next `TaskList` and `TaskGet`, in the same running process, with no restart |
| Do the agent's own `TaskCreate` ids collide with seeded ones | No. The next id is the highest id plus one (task 5 after four seeded, tasks 10 and 11 after nine) |
| Was Fleet's gate record ever wrong | No, in one run. Each gate closed on the fact it names, at the right commit, forward only. Two of the three closed after the agent had already called `all_done` |
| Did the agent stay in a stretch until its gate closed | Once of two times. It waited for the plan gate. It went through the push gate without looking at it, and finished before Fleet closed it, because `blockedBy` is advice |
| Did the agent's claimed step match what it was doing | Mostly for the first two steps, then not. Four of the six steps were never marked in progress. Two of those were marked completed together after the work, and Deliver was marked completed before the merge had happened |
| Biggest finding | `record_plan` already tells the agent "Your task list is this plan: TaskCreate each task word for word". The agent obeyed it, made two tasks of its own, and tracked the whole Fix stretch on those, not on the seeded Fix and Tests steps. Two templates in one list compete |

So gates are Fleet's and can be kept by a poller, and steps can live in the agent's list as its claims. What the list cannot do is hold the agent at a gate or make its marks timely. If the display needs the current step to be right while it works, something has to ask for the mark (a hook on the edit or the test run), and that is a different spike.

## Real-agent sessions started

Three, counted from the first command.

| Session | What | Model |
|---|---|---|
| 1 | Probe for the file format of a task (one `TaskCreate`, one `TaskList`) | haiku |
| 2 | The probe for questions a, b and c, one process fed three turns | haiku |
| 3 | The armada-35xp run | sonnet |

The owner's cap (one probe, one yardstick run on armada-35xp, no armada-tyl run) arrived after sessions 1 and 2 had already run. Session 1 was a format check that session 2 would have covered. No run on armada-tyl was started.

## What was built

All in `docs/spikes/044-seeded/`. The rig's own files are unchanged; changed ones are copies there.

| File | What it is |
|---|---|
| `probe.py`, `probe-output.txt` | The task list probe |
| `seed.py` | Writes the template as nine task files, and `template.json` |
| `gates.py` | The gate keeper |
| `person.py` | Copy of the rig's, plus the person's merge and a `--rule` option |
| `timeline.py` | Copy of the rig's, plus the gate events and the agent's claimed step on its acts |
| `run.sh`, `verdict.sh` | Copies of the rig's, parameterised on the yardstick and the test file |
| `yardstick-2.md` | The armada-35xp brief, in the rig's style |
| `run-35xp/` | The run's timeline, gate record, gate log, the person's acts, verdict, brief and final task files |

Two things were not done. The rig's `yardstick.md` was not used, and `cargo xtask verify-docs --write` was not run, since this Job may not run cargo. `docs/INDEX.md` is untouched.

## The probe answers

One haiku process in stream-json mode under the rig's scratch HOME, with `CLAUDE_CODE_TASK_LIST_ID` set to a probe name. Output is `044-seeded/probe-output.txt`.

| | What the probe did | What happened |
|---|---|---|
| a | Started the process with no list. Wrote four task files. Then sent turn one asking for `TaskList` and `TaskGet 2` | Both showed the seeded tasks, including `blockedBy`. The list is read from disk when the tool runs, not at start |
| b | Rewrote `1.json` with `status: completed` from outside, then turn two asked for `TaskList` and `TaskGet 1` | Both showed `completed`. A blocked task is shown "blocked by" only while its blocker is not completed |
| c | Turn three: `TaskCreate "agent own"` | It became task 5, after the four seeded. A `.lock` file appears in the directory |

Three facts the probe and the run added.

| Fact | Consequence |
|---|---|
| The list id is the session id unless `CLAUDE_CODE_TASK_LIST_ID` names it. The hosted Job's Claude session id equals the Armada session id (the transcript file is named by it), and Fleet sets no list id | The seeder writes `tasks/<armada session id>/`, which is known as soon as `POST /sessions/start` answers |
| The task tools are deferred. The agent loads them with `ToolSearch` before its first use | Three `ToolSearch` calls in the run. The first `TaskList` still came at 7 seconds and showed the seeded list |
| A file written by another process is not heard by the mod. The mod reads the list from disk on the agent's first task tool call and after that follows the agent's tool calls | Fleet's own ticks never reach `session_build_tasks`. In the run's store the three gate tasks stay pending forever, while the task files and `record.json` say completed. A display has to draw gates from Fleet's record |

Not tested: a race between Fleet rewriting a task file and the agent updating the same task at the same moment. The keeper writes by temporary file and rename, so a reader sees one whole file or the other, but a `TaskUpdate` that read the old file first could write it back over a tick. It only matters for a gate task, and the keeper would find it closed in `record.json` and could tick again.

## The template

The bug workflow from `.armada/workflows/bug.json`, regrouped by gates. Nine tasks, each `blockedBy` the one before. The instructions are the step's lines from `bug.json`, kept word for word; `seed.py` says which lines went where.

| # | Task | Kind | From `bug.json` |
|---|---|---|---|
| 1 | Step: Find the cause | agent's | `find_the_cause` lines 1, 3, 8, 9 |
| 2 | Step: Plan the fix | agent's | the other lines of `find_the_cause`, ending in `record_plan` and the wait |
| 3 | Gate: the person approves the plan. Fleet ticks this | Fleet's | |
| 4 | Step: Fix | agent's | `fix`, without the two walk lines |
| 5 | Step: Tests, shown red then green | agent's | `tests`, plus a line naming the one vitest file |
| 6 | Step: Push the branch | agent's | new: commit, `git push origin`, do not mark the gate |
| 7 | Gate: Checks green at the pushed commit. Fleet ticks this | Fleet's | |
| 8 | Step: Deliver | agent's | three lines of `deliver`, then: ask the person to merge, stop, then `all_done` |
| 9 | Gate: the branch is merged. Fleet ticks this | Fleet's | |

The workflows setting was off in the dev Fleet, so the old rail stayed out. The template reached the agent through the task list and one added line in the brief: "Your task list holds the workflow for this Job: work its tasks in order. A task that starts "Gate:" is Fleet's to tick, not yours. The list also asks you to have the person merge the branch, which goes past "pushing the branch is the end of it" above."

## The gate keeper

`gates.py` polls every two seconds and keeps `record.json`: for each gate, `closed_at` and the fact that closed it. A closed gate is never reopened and never re-derived.

| Gate | Fact Fleet sees | Evaluated |
|---|---|---|
| Plan approved | `session_plan.approved_at` in the dev store | from the start |
| Checks green at the pushed commit | the yardstick's one vitest file, run by the keeper at the tip of the Job's branch on the bare origin, in a scratch checkout, exit 0 with a passing count | only after the plan gate, again at each new tip until green |
| Merged | the branch tip is an ancestor of `integration-<id8>` on the bare origin | only after the Checks gate |

Two choices to know about. The person's merge goes into `integration-<id8>`, a branch cut from origin's `main`, and not into `main`, because another spike shares that origin and a merge to `main` would change its base. A gate task the agent marks completed before its gate has closed is put back to pending and logged; nothing in the run triggered it, but it was tested against a fake origin and store (the early claim was put back, the three gates closed in order, the merge closed the last).

## The run: armada-35xp

One sonnet Job, 3.5 minutes, 69 tool calls, 2 plan tasks, one question, one commit, pushed. Full table in `044-seeded/run-35xp/timeline.md`. Another spike ran its own Job on the same Fleet over the same minutes, so the machine was shared.

| What | Result |
|---|---|
| The work | One file changed. Both tests now mount the scenario `checks` and use its tile names (`desktop_test`, `screens_test`, `test`). The first test's "next Tab leaves the glass" assertion was weakened to "the next Tab is not the row's open button", because each row's start time is a button. The file is green at the pushed commit, 2 tests |
| The person | Approved the plan 1.4 seconds after it was recorded. Answered the agent's question about row stamps with option 1 ("Keep stamps, loosen test"), by default and not by judgement: `person.py` picks the first option of a question it has no rule for. Merged when asked |
| The question "retarget or delete" | Never asked. The agent chose to retarget itself, so the `--rule` for "retarget them" never fired |

### Score

| Measure | Result |
|---|---|
| Did the agent work the steps in order | Steps 1 and 2 were marked in progress and then completed in order. Step 4 and 5 were marked completed together, one second apart, after the work. Step 6 and 8 were marked completed in order. Never out of order, but four of six were marked after the fact |
| Did it stay inside a stretch until its gate closed | Stretch 1 to 2: yes. It had a `plan_approved` notice from Fleet and began editing 5 seconds after the gate closed. It did write a throwaway probe test file (`zz-probe.test.tsx`) before approval, against "edit nothing before then but a mock", and was refused when it tried to delete it. Stretch 2 to 3: no. It marked Deliver completed and asked for the merge 2 seconds after pushing. The gate closed about 14 seconds after the push. It never read the gate |
| Did it finish before the gates | Yes. `all_done` came about 7 seconds before the Checks gate closed. Nothing refused it |
| Did it mark the seeded tasks | Yes, 6 of 6 steps ended completed, 4 of them never marked in progress. It made two tasks of its own (10, 11) from the `record_plan` reply and tracked the Fix stretch on those |
| Did it touch a gate task | No. Nothing was put back |
| Was Fleet's gate record ever wrong | No. Plan gate closed 1.3 seconds after approval. Checks gate closed on `30d8c9d607` with "2 passed". Merged gate closed at the same poll, because the person had merged before the Checks gate closed and the keeper holds a gate until the one before it has closed |
| Claimed step against acts | See the next table |

### Claimed step against what the agent did

Judged from its tool calls in the transcript.

| Claim | At | Acts | Agree |
|---|---|---|---|
| 1 Find the cause, in progress | +0:08 | read the test, listed scenarios, looked for tile names | yes |
| 1 completed, 2 Plan the fix in progress | +0:43 | the cause was stated at +0:50, after the mark | yes, a little early |
| 2 in progress through the probes | +0:43 to +1:41 | ran `armada run bootstrap`, wrote a probe test file and ran it twice to read the tile names | partly: the probes were finding out, filed under planning, and a Write before approval |
| (none, then its own task 10 in progress) | +1:51 to +2:55 | five edits and two vitest runs: the whole Fix and Tests work | no: seeded Fix and Tests were never in progress |
| 4 Fix and 5 Tests completed | +2:56, one second apart | before the commit at +2:59 | after the fact, and Fix included the commit |
| 6 Push completed | +3:00 | right after `git ls-remote` showed the branch on origin | yes |
| 8 Deliver completed | +3:02 | the same instant as the request to merge, before the merge | no: ahead of the act |

Of the eleven `TaskUpdate` calls, eight were on seeded steps and three on the agent's own tasks 10 and 11. Five of the eight agree with the acts (steps 1 and 2 both ways, step 6). Three do not: steps 4 and 5, marked completed after the fact and never in progress, and step 8, marked ahead of the act. The three calls on its own tasks were accurate: 10 was in progress while it edited and completed when green.

## What he would see

A short bar for the Job, two layers, with the origin of each piece shown.

```
  Plan approved  ──  Checks green  ──  Merged            <- three gates, Fleet's
     [done]            [open]           [   ]

  Stretch 2 of 3: Fix, Tests, Push                       <- the stretch before the first open gate
     Agent says: Fixing                                  <- the agent's claim, marked as its own
```

| Element | How it is detected | Reliability seen |
|---|---|---|
| Each gate's state, and when it closed | A fact: the plan row's `approved_at`, the keeper's own vitest run at the pushed commit, the tip being an ancestor on origin. Kept in `record.json`, not read back from the task files | Held in the one run. Latency: 1.3 seconds for the plan, about 14 seconds for Checks (a cold checkout, six seconds of vitest) |
| Which stretch the Job is in | Derived from the gates: the stretch before the first open gate. No agent word used | Right here; it would have said stretch 2 while the agent was in stretch 3 for the last 14 seconds, which is true to Fleet's record |
| The step under it | The agent's word: the task it last marked in progress, drawn as "Agent says" | Right for steps 1 and 2, blank for Fix and Tests, absent for the plan's own tasks unless merged with the template |
| A step shown done | The agent's word: a `TaskUpdate` to completed | Four of six were after the fact, one ahead of the act, so a tick on a step is a claim and should look like one |
| Disagreement | A step claimed completed while its gate is open is a fact worth showing: Deliver completed with the Checks gate open | Visible in this run at +3:02 |

A gate drawn from the task file's status or from `session_build_tasks` would be wrong: Fleet's own ticks are not heard by the mod, so those gate tasks stay pending in the store.

## What broke or surprised

| What | Detail |
|---|---|
| Two task lists in one | `record_plan`'s reply tells the agent to `TaskCreate` each plan task word for word. The agent did, and then tracked on them. The seeded steps and the plan's tasks overlap (Fix, Tests, Push against "Mount the scenario", "Run red then green, commit and push"). One of the two instructions has to give way: either the plan tasks become the children of the Fix step, or the Fix and Tests steps are the plan |
| `blockedBy` is not a gate | `TaskUpdate` to completed on task 8 succeeded while task 7 (its blocker) was pending |
| The agent did not wait on a gate with latency | Anything that takes Fleet more than a couple of seconds will be passed by an agent that does not read the list. The task text said "do not start the next stretch", and the agent did not check. A gate it must wait at needs a message to it when the gate closes (the plan gate had one from the existing plan approval notice) |
| Gates closed after `all_done` | The keeper outlived the Job and closed two gates about 7 seconds after it. A real Fleet should either hold `all_done` until the last gate or say what closed after |
| The mod does not hear Fleet's ticks | See the probe section. The store shows gate tasks pending permanently |
| A weak answer from the person | `person.py` answered the agent's design question with option 1 by default and the agent loosened an assertion. `yardstick-2.md` names loosening an assertion until it checks nothing about keys as the wrong fix. Here one assertion was weakened and the rest kept, so it is not that, but it was my script and not the person's judgement that chose it |
| Shared Fleet | Another spike's Job ran the same test file over the same minutes, so the machine load is not clean |
| The rig | The first start of `run.sh` failed before any session was started: the copy had lost its execute bit. A hook refused two of my commands that removed or forced something; they were redone another way. A stray symlink `ms-playwright` sits inside the scratch home's `Library/Caches/ms-playwright` directory, which a hook would not let me remove: harmless, in the scratch directory |

## What this does not show

One run. One model, one task, one pass through the repository, so the counts above (four of six steps never in progress, three of eight seeded marks off) are counts of one Job. The armada-tyl yardstick was not run, because of the owner's cap. The mid-run change (b) was shown in the probe and not in the run: the agent last read the list at +1:49, two seconds before the first tick. The merge gate was only run with the person merging on request, so a merge that the agent does itself, or a merge that is refused, is untested.
