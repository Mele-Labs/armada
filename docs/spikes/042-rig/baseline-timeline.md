# Baseline timeline

One run of the yardstick Job (`yardstick.md`) with no workflow, on the dev Fleet, by a real Claude Code agent on sonnet at medium effort. Made by `run.sh baseline`. The person's acts file is empty: the agent never asked for anything. This is the fourth start of the baseline; the first three were stopped within seconds because the rig was wrong (the proposer picked the `bug` workflow while `features.workflows` was still on, or `settings.json` was written in the wrong shape), and none of them edited anything.

## What it left

```
stopped because: done
branch: armada/session-c8e1d57d
origin has the branch: yes, at b66c69a566
commits on it past main: 1
files changed against main:
   apps/desktop/src/renderer/src/mock/scenario.test.ts | 6 +++++-
   1 file changed, 5 insertions(+), 1 deletion(-)
scenario.test.ts at the pushed commit:
   Test Files  1 passed (1)
        Tests  9 passed (9)
```

## Timeline

Run of session c8e1d57d-86fe-4cfe-97c9-cc567a45d528, first event 2026-11-07 01:32:41 UTC. Times are minutes:seconds after it.

| time | source | event |
|---|---|---|
| +00:00 | store | event session.started: Started |
| +00:00 | store | person's message: A test on main is red and has been for days: `apps/desktop/src/renderer/src/mock/scenario.test.ts`, the test "every row… |
| +00:04 | agent | message to agent: A test on main is red and has been for days: `apps/desktop/src/renderer/src/mock/scenario.test.ts`, the test "every row exported from scenarios/ has … |
| +00:08 | agent | Bash armada job status |
| +00:11 | agent | Bash armada run bootstrap |
| +00:18 | store | slot 1 leased on armada/session-c8e1d57d |
| +00:18 | store | event slot.leased: Slot 1 leased |
| +00:18 | agent | tool refused or failed: PreToolUse:Bash hook error: This job writes only inside a worktree leased for it, and it had none until this write. Slot 1 is leased on branch armada/session-c8e1d57d at… |
| +00:20 | agent | says: The job is being moved into its leased worktree slot. I'm making no more calls this turn and will repeat the … |
| +00:20 | agent | message to agent: You are now in $SCRATCH/repo/.armada/… |
| +00:23 | agent | Bash armada run bootstrap |
| +00:34 | agent | Edit $SCRATCH/repo/.armada/slots/slot-1/apps/desktop/src/renderer/src/mock/scenario.test.ts |
| +00:34 | store | event session.edited: Edited 1 file |
| +00:36 | agent | Bash cd apps/desktop && node_modules/.bin/vitest run src/renderer/src/mock/scenario.test.ts 2>&1 / tail -15 |
| +00:37 | agent | tool refused or failed: Exit code 1 (eval):cd:1: no such file or directory: apps/desktop |
| +00:38 | agent | Bash cd $SCRATCH/repo/.armada/slots/slot-… |
| +00:44 | agent | says: Test is green (9/9). Committing and pushing now. |
| +00:45 | git | commit b66c69a566 Count only rows when checking scenario export names are unique [origin/armada/session-c8e1d57d, armada/session-c8e1d57d] |
| +00:46 | agent | Bash git add src/renderer/src/mock/scenario.test.ts && git commit -q -m "Count only rows when checking scenario export names are unique Two scenario files… |
| +00:48 | agent | Bash git ls-remote origin armada/session-c8e1d57d |
| +00:52 | agent | all_done {"body": {"summary": "Fixed the red scenario.test.ts uniqueness test, which counted plain TITLE exports from two scenario files as duplicate rows; it now counts only row… |
| +00:52 | store | all_done: Fixed the red scenario.test.ts uniqueness test, which counted plain TITLE exports from two scenario files as duplicate … |
| +00:52 | store | thread row release: {"slot": 1, "branch": "armada/session-c8e1d57d"} |
| +00:52 | store | event slot.released: Slot 1 given back |
| +00:53 | store | event session.ended: Ended |

Summary: 0.9 minutes, 16 tool calls by the agent (Bash 14, Edit 1, mcp__armada-fleet__all_done 1). Fleet's store: 16 tool rows, plan versions 0 (approved 0), plan task marks 0, agent tasks seen 0, step and gate events 0, notices to the agent 0, stops 0. Person's acts 0, commits 1.
