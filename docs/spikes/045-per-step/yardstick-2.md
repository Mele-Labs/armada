# The second yardstick Job

A second small real bug for spike 45, in the style of `../042-rig/yardstick.md`. The text under "Brief" is the Job's report, unchanged. Everything else on this page is for the person running the spike.

## Brief

Two tests on main are red and have been since 1 Nov: both tests in `apps/desktop/src/renderer/src/mock/checks-keys.test.tsx` fail with `no mock scenario named job-checks`. Find why and fix it.

The issue about it is armada-35xp, filed on 1 Nov:

> Both tests in apps/desktop/src/renderer/src/mock/checks-keys.test.tsx fail with 'no mock scenario named job-checks'. The scenario went with the Jobs package (694e9745f2). Retarget the tests at a scenario that has Checks tiles, or delete them.

What I want: the cause named in a sentence, the tests retargeted at a scenario that has Checks tiles (or deleted, if no scenario can serve), the whole of `checks-keys.test.tsx` green, and the change committed on your branch and pushed to `origin`. There is no forge here, so no pull request: pushing the branch is the end of it.

How to run the test. A fresh slot has no dependencies, so run `armada run bootstrap` once in your slot (a few seconds, it copies them). Then:

```bash
cd apps/desktop && node_modules/.bin/vitest run --project 'smoke (browser)' src/renderer/src/mock/checks-keys.test.tsx
```

Run only that one file. Do not run cargo, do not run pnpm install, and do not run the other test files: the machine is shared. The test runs in a headless Chromium and takes a few seconds.

## What the right answer looks like

The cause. The two tests mount the scenario `job-checks`, which lived in the Jobs package that commit 694e9745f2 deleted. Nothing else in `checks-keys.test.tsx` is wrong, so the fix is the name the tests mount, or the tests going.

Done means: `checks-keys.test.tsx` passes, `git diff origin/main --stat` shows at most the test file and a scenario file, the branch is on `origin`, and the reply says the cause.

A wrong fix would loosen the tests until they pass with no Checks tiles on screen, or restore the deleted Jobs package.

When an agent asks which of retarget or delete, the person answers "retarget them".

## Why this one

Red at the base commit, one file, a few seconds in headless Chromium. Unlike armada-tyl the fix needs a judgement about which scenario has the tiles the tests name (`desktop_test`, `hooks_test`, `ipc_test`, `test`), so a plan has something real to say, and the agent reads more files.
