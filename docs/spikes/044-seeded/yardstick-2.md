# The second yardstick Job

One more small real bug, in the style of `docs/spikes/042-rig/yardstick.md`. The text under "Brief" is sent as the first message of the Job, unchanged, with one line added by `run.sh`. Everything else on this page is for the person running the spike.

## Brief

Both tests in `apps/desktop/src/renderer/src/mock/checks-keys.test.tsx` are red and have been since the Jobs package was removed (commit 694e9745f2): they fail with `no mock scenario named job-checks`. Find why and fix it.

The issue about it is armada-35xp, filed on 1 November and possibly since overtaken, so do not trust its wording:

> Both tests in apps/desktop/src/renderer/src/mock/checks-keys.test.tsx fail with 'no mock scenario named job-checks'. The scenario went with the Jobs package (694e9745f2). Retarget the tests at a scenario that has Checks tiles, or delete them.

What I want: the cause named in a sentence, the tests in that file green, and the change committed on your branch and pushed to `origin`. Whether the tests are retargeted or deleted is a judgement about what they are for; if you cannot tell, ask me with `ask_the_person` and I will say. There is no forge here, so no pull request.

How to run the test. A fresh slot has no dependencies, so run `armada run bootstrap` once in your slot (a few seconds, it copies them). Then:

```bash
cd apps/desktop && node_modules/.bin/vitest run src/renderer/src/mock/checks-keys.test.tsx
```

Run only that one file. It runs in a headless browser and takes a few seconds when it passes; a failing test waits up to 15 seconds for an element, so a wrong guess is slow. Do not run cargo, do not run pnpm install, and do not run the other test files: the machine is shared.

## What the right answer looks like

The cause. The two tests mount the mock scenario `job-checks`, which was deleted with the Jobs package, so `mount` throws before anything is drawn. Nothing else in the file is broken.

| Fix | Files | Note |
|---|---|---|
| Retarget both tests at a scenario that draws a Checks page, and change the tile names the assertions use to that scenario's Checks | `checks-keys.test.tsx`, possibly one scenario | The asserted behaviour is the point: the tiles are one Tab stop, and the arrows, j/k, Enter and o move and open between tiles. The names `desktop_test`, `hooks_test`, `ipc_test` and `test` came from the deleted scenario and appear in no remaining one that this brief's author found, so a plain rename of the scenario is not enough |
| Delete the file | `checks-keys.test.tsx` | The owner's answer to "which" is "retarget them", so this is the runner-up and is judged by whether the behaviour is covered elsewhere |

A wrong fix would keep the file green by loosening it until it asserts nothing about keys (no focus checks, no `aria-expanded`), or by restoring a scenario named `job-checks` that is not a Checks page.

Done means: `checks-keys.test.tsx` passes, still asserts the one Tab stop and the arrow, j/k, Enter and o behaviour, `git diff origin/main --stat` shows a small number of files, the branch is on `origin`, and the reply says the cause.

## Why this one

| Pick | Why |
|---|---|
| armada-35xp (this) | Red at the base, one file, no build. Unlike armada-tyl the right fix is not one line: the agent has to find a scenario that draws a Checks page and rewrite the asserted names, so there is real reading and a real choice to put to the person. Stops short of anything the owner would walk |

The test is a browser test (`smoke (browser)` in `apps/desktop/vitest.config.ts`) and starts a headless Chromium. In the scratch home Chromium is found through a symlink `Library/Caches/ms-playwright` to the owner's cache, which the rig does not have; spike 44 added it.

## What the clone gives the agent

The same clone as the first yardstick: base commit 3fee6c4c46 plus the rig's commit. Its one Manifest Check still names `scenario.test.ts`, which is not this Job's file; the brief names the file to run.
