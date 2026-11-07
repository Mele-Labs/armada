HANDOFF

**Where the work is**
- Branch `armada/session-03703ac7`, slot 1 at `.armada/slots/slot-1`. No commits and no edits. The tree is clean apart from the `.mcp.json` that was already untracked.
- I ran `armada run bootstrap` once. I ran `cd apps/desktop && node_modules/.bin/vitest run src/renderer/src/mock/scenario.test.ts`, which fails with `expected 380 to be 381` at `scenario.test.ts:89`. I also ran the duplicate-export grep in `mock/scenarios/`, which prints only `TITLE`.
- Plan v1 is recorded and approved: group g1, tasks t1–t4.

**What I decided and why**
- **Cause:** `TITLE` is exported from two files in `apps/desktop/src/renderer/src/mock/scenarios/`:
  - `now-and-plain-plans.ts:45` ("Email notifications page")
  - `auto-land-answers-at-once.ts:18` ("Split the slot lease into two steps")
  - The test requires every export name across the folder to be unique. The second file came in with f2846e647d on 2026-11-04.
- **Not the cause:** the armada-tyl wording is stale. `isRow` (`scenario.ts:46`) already filters the glob to Scenario-shaped exports. The 269/278 figures and the Studio-test claim no longer apply.
- **Fix:** rename the newer file's export, since it was added second. The new name is `SLOT_LEASE_TITLE`.
  - In `scenarios/auto-land-answers-at-once.ts`, change line 18 and the use at line 52 (`title: TITLE`).
  - In `walks/auto-land-answers-at-once.ts`, change the import at line 4 and the use at line 18 (`deckRow(TITLE)`).
  - `now-and-plain-plans` and its walk, which import `TITLE` too, stay as they are. No other file imports either `TITLE`.
- Nothing in Bridge changes, so there is no walk and no new test. The existing test is the proof.
- I stopped at the plan because my stretch brief said not to begin the fix. The approval message said to build it, but that is for the next agent.

**What is open**
- Do the four tasks t1–t4, in order. Then check that the scenario test passes all 9 tests and that the duplicate-export grep prints nothing.
- Update armada-tyl to say what was actually wrong. I couldn't check whether the job can reach the tracker, because `bd` calls are denied here.
- Unsettled: whether `docs/practices/list-files.md:42-46` should also say plain exports must be uniquely named. It now states the rule only for rows, while `scenario.test.ts:85-86` allows plain values. I don't expect to change the doc.
- Ask the owner about the doc question before editing it.