// Group three's boundary with its Checks under way: two done, `screens_test` writing its log, the
// rest waiting. What a Check's log panel opens on from a plan group's strip, and what group two's
// passed Checks kept to read once they had ended.
//
// **Not an arc moment, so not in `ARC_MOMENTS`.** The mock's `check-logs` scenario draws it,
// beside a merge line, for the walk that opens a log from each.

import type { CheckOutputRead, StepDetail } from "@armada/protocol";

import type { ArcMoment } from "./arc-base";
import { BRIDGE_CHECKS, checkNames } from "./arc-base";
import { allPassed, executing, executingConcurrent, implementStep } from "./arc-executing";
import { withGroup } from "./arc-plan";

/** Where the boundary's Checks write, under this Job's records. */
const CHECKS_AT = ".armada/checks/3-show-what-s-running";

/** The live log of each Check group three's gate has started, by name. */
export const LIVE_LOGS = {
  test: `${CHECKS_AT}/implement.1.live.test.log`,
  typecheck: `${CHECKS_AT}/implement.1.live.typecheck.log`,
  screens_test: `${CHECKS_AT}/implement.1.live.screens_test.log`,
} as const;

/** What group two's `screens_test` kept once it had passed. */
const KEPT_SCREENS_TEST = `${CHECKS_AT}/implement.1.screens_test.log`;

const PASSED_LINES = [
  " RUN  v3.2.4 /Users/user/armada/packages/screens",
  "",
  " ✓ src/board.test.ts (22 tests) 31ms",
  " ✓ src/plan-board.test.ts (31 tests) 58ms",
  " ✓ src/merge-line.test.ts (12 tests) 9ms",
  "",
  " Test Files  213 passed (213)",
  "      Tests  1383 passed (1383)",
];

const PASSED_OUTPUT: CheckOutputRead = {
  ok: true,
  output: {
    attempt: 1,
    name: "screens_test",
    path: KEPT_SCREENS_TEST,
    lines: PASSED_LINES,
    from_line: 1,
    total_lines: PASSED_LINES.length,
    bytes: PASSED_LINES.join("\n").length,
    whole: true,
  },
};

const STARTED = "2026-09-22T10:41:00Z";

/** The step, with group two's runs kept and group three's gate under way. */
function checkingStep(): StepDetail {
  const runs = allPassed(checkNames(BRIDGE_CHECKS)).map((run) =>
    run.name === "screens_test" ? { ...run, output_path: KEPT_SCREENS_TEST } : run,
  );
  const ran = (name: string) => ({ attempt: 1, name, outcome: "passed" });
  return {
    ...implementStep(runs, "2026-09-22T10:44:00Z"),
    checking: {
      attempt: 1,
      checks: [
        { name: "test", started_at: STARTED, took_ms: 61_000, ran: ran("test"), output_path: LIVE_LOGS.test },
        {
          name: "typecheck",
          started_at: STARTED,
          took_ms: 24_000,
          ran: ran("typecheck"),
          output_path: LIVE_LOGS.typecheck,
        },
        { name: "bridge_build" },
        { name: "storybook" },
        { name: "desktop_test" },
        { name: "screens_test", started_at: "2026-09-22T10:42:05Z", output_path: LIVE_LOGS.screens_test },
        { name: "components_test" },
      ],
    },
  };
}

export function groupChecking(): ArcMoment {
  const concurrent = executingConcurrent();
  const groups = withGroup(concurrent.draft.groups!, "g3", { state: "checking" });
  return {
    name: "groupChecking",
    says: "Implement — group three's Checks are running, and screens_test is writing its log",
    fixtures: [
      {
        ...executing({
          says: "running — a group's boundary is running its Checks",
          groups,
          step: checkingStep(),
          processes: [],
        }),
        checkOutputs: { "implement.1.screens_test.log": PASSED_OUTPUT },
      },
    ],
    ...(concurrent.opens === undefined ? {} : { opens: concurrent.opens }),
    draft: { ...concurrent.draft, groups },
  };
}
