# The yardstick Job

One small real bug, the same for every variant. The text under "Brief" is sent as the first message of the Job, unchanged. Everything else on this page is for the person running the spike.

## Brief

A test on main is red and has been for days: `apps/desktop/src/renderer/src/mock/scenario.test.ts`, the test "every row exported from scenarios/ has a name of its own, and reaches the list". It fails with `expected 380 to be 381` at the line that compares the size of a set of export names with the number of names. Find why and fix it.

The issue about it is armada-tyl, filed on 26 Oct and since partly overtaken, so do not trust its wording:

> apps/desktop/src/renderer/src/mock/scenario.ts globs every export of scenarios/*.ts as a Scenario, but several files there export plain values (TITLE and ASKED in now-and-plain-plans.ts, NOTES_JOB in job-notes.ts, shipSessionsStarting in cockpit-ship.ts). On main, 'every scenario has its own name' fails with expected 269 to be 278, and the Studio test reads holds of undefined. Either move the helpers out of scenarios/ or filter the glob to Scenario-shaped exports.

What I want: the cause named in a sentence, a fix in one or two files, the whole of `scenario.test.ts` green, and the change committed on your branch and pushed to `origin`. There is no forge here, so no pull request: pushing the branch is the end of it.

How to run the test. A fresh slot has no dependencies, so run `armada run bootstrap` once in your slot (a few seconds, it copies them). Then:

```bash
cd apps/desktop && node_modules/.bin/vitest run src/renderer/src/mock/scenario.test.ts
```

Run only that one file. Do not run cargo, do not run pnpm install, and do not run the other test files: the machine is shared.

## What the right answer looks like

The cause. `scenarios/` files may export plain values beside their rows (commit 5f6e272ac2), and `scenario.ts` already filters rows with `isRow`. But the last test still counts every export name of every file, rows or not, and two files export a plain `TITLE`: `scenarios/auto-land-answers-at-once.ts:18` and `scenarios/now-and-plain-plans.ts:45`. The set is one smaller than the list. Nothing is lost at runtime, because `TITLE` is not a row.

Two fixes are both right:

| Fix | Files | Note |
|---|---|---|
| Count duplicates only among exports that pass `isRow` | `scenario.test.ts` | Keeps the rule the comment above the test states: a name two files both export would overwrite a row |
| Rename one of the two `TITLE` exports, and its importers | one scenario file, plus whoever imports it | Leaves the test as written |

A wrong fix would delete or loosen the assertion so it no longer catches two files exporting the same row name, or edit `scenario.ts` to hide an export.

Done means: `scenario.test.ts` passes (9 tests), `git diff origin/main --stat` shows at most two files, the branch is on `origin`, and the reply says the cause.

## Why this one

| Pick | Why |
|---|---|
| armada-tyl (this) | Red at the base commit, 4 s for the one test file, no build and no browser. A cause to find that the issue text gets half wrong, so the agent has to read the test and not trust the ticket. A one or two line fix with two valid shapes. Nothing for the owner to look at. |
| armada-w1k, a Job's Now beat reads only "Agent" while the agent waits on a person | `accountNowOf` in `apps/desktop/src/renderer/src/session-activity.ts:29` reads the account line only while a turn runs. Real logic and a unit test, but what the beat should say while waiting is a design decision, and the result is something the owner reads on screen. |
| armada-35xp, `checks-keys.test.tsx` mounts the deleted job-checks scenario | Red at the base, one file. The fix is to retarget the tests at another scenario or delete them, which is a judgement about what the tests are for, and a tsx mount test needs a browser-like environment, so it is heavier than the pick. |

Left out on purpose: armada-1793 and armada-gqq3 need a form field or a card behaviour the owner would walk visually. armada-5ux and armada-mjc are Rust and would need a build. The four tests that fail in `claude plugin test plugins/armada` at this base are not filed as bugs.

## What the clone gives the agent

Base commit 3fee6c4c46 plus one rig commit: a one-check Manifest, no `.mcp.json`, no tracker. `origin` is a bare repository in the scratch directory. `armada run bootstrap` copies `node_modules` from a template tree so the test runs without the network.
