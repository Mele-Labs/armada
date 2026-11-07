HANDOFF

**Where the work is**
- Branch `armada/session-c5d4e9e4`, slot `.armada/slots/slot-1`, at `a63f323a08`. The tree is clean and nothing is edited.
- I ran `armada run bootstrap` in the slot. I then ran `cd apps/desktop && node_modules/.bin/vitest run --project 'smoke (browser)' src/renderer/src/mock/checks-keys.test.tsx`. Both tests fail with `no mock scenario named job-checks`, thrown at `mock/mount.tsx:187`.
- The plan is recorded as v1, and the owner approved it. Its five tasks are:
  1. Confirm the red. This is done, as described above.
  2. Add a checks-keys scenario in `checks-fleet.ts` via `checkingOver`, with Checks `desktop_test`, `hooks_test`, `ipc_test` and `test` in the walked order, one failed.
  3. Register it in `slices/manifest.ts`, beside `checking` and `checkingWithoutRunSheet` (imported at line 8).
  4. Point `opened()` in `checks-keys.test.tsx` (line 23) at the new name.
  5. Run `checks-keys.test.tsx` green, then `scenario.test.ts` for name clashes.
- The approval notice said to build it. My stretch ends at approval, so I did not start the fix. The next agent does tasks 2 to 5.

**What I decided and why**
- **Cause:** the test mounts the scenario `job-checks` (`checks-keys.test.tsx:23`). That scenario was deleted on 17 Oct in 694e9745f2 ("Bridge without Jobs"), along with `packages/surfaces/jobs/src/fake/job-checks-fixture.ts`. The test was written earlier, on 12 Oct (0cd257f76f), so it has been red since.
- **Retargeting alone can't work:** `hooks_test` and `ipc_test` appear in no mock scenario. The test needs four tiles in this order: `desktop_test`, `hooks_test`, `ipc_test`, `test`.
- **New scenario, not an edited `checks`:** I chose a separate scenario so the `checks` walks (for example `walks/checks-with-their-logs.ts`) and tests that use `checks` keep their data. I did not run those to see whether editing `checks` would break them.
- **Nothing a person sees in Bridge changes,** so the plan has `seen: false` and no walk.

**What is open**
- **Tile order:** `byRequested` (`packages/screens/src/manifest-checks.ts:178`) sorts newest-request first, ties by input position. Make the fixture produce exactly `desktop_test`, `hooks_test`, `ipc_test`, `test`, then check it in the browser run. The test presses ArrowDown and `j`, then `{ArrowUp}`, and expects `hooks_test`, `ipc_test`, `test`, then `hooks_test` again.
- **Declared names:** `checkEntriesOf` (`manifest-checks.ts:110`) only draws a run whose name is in `sheet.checks`. I did not open `sheet()` in `mock/manifest-fake.ts` to see how it declares them. Merge-line Checks (`landEntriesOf`) draw without the run sheet, so they may be the simpler route for three of the tiles. The checkout's own `test` Check needs the sheet.
- **Failed tile:** "one failed" in the plan is my own choice for covering the lit tile. The test never needs it, so it can be dropped.
- **Name clashes:** each scenario file exports `TITLE`, so a duplicate `TITLE` is expected and not a clash. Find real duplicate exports with `grep -hoE "^export (const|function|let|class) [A-Za-z0-9_]+" *.ts | awk '{print $3}' | sort | uniq -d` in the scenarios directory.
- **Tests to run:** only `checks-keys.test.tsx` and `scenario.test.ts`. The machine is shared, and the rig forbids cargo, `pnpm install` and other test files.