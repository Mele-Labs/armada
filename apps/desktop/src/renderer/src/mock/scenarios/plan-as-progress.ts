// The plan as the Job's progress: Now is one row a chunk of groups, each ended by a gate Fleet sees for itself. The plan gate is closed; three groups with no gate between them
// are one live chunk, named for all three, waiting on his walk, with two subagents at work, each on the group its brief named; Checks and the merge are to come. Time passes
// three times: the walk is approved and Prune old files is live; Checks fail at the pushed commit; Checks pass with one test that is red on main too.
// Over the Job of `job-walk-on-the-spine`.

import type { GateClose } from "@armada/protocol";
import type { Session, WorkDrone, WorkGroup } from "@armada/screens/src/draft/sessions";

import { BUSY, auditLedger } from "../audit-job";
import type { Scenario } from "../moment";
import { s261OneSessionView as base } from "./one-session-view";

const closedAt = (at: string, fact: string, more: Partial<GateClose> = {}): GateClose => ({
  at,
  fact,
  ...more,
});

const groups = (stage: number): WorkGroup[] => [
  {
    id: "g0",
    agent: "claude",
    title: "Name the cause and the target",
    paths: [],
    tasks: [
      {
        id: "Z1",
        text: "Find where the audit log is written",
        state: "done",
      },
      {
        id: "Z2",
        text: "Agree the rotation with you",
        state: "done",
      },
    ],
    gate: "plan_approved",
    gateClosed: closedAt("2026-10-26T13:52:00.000Z", "You approved the plan, version 2"),
  },
  {
    id: "f1",
    agent: "rust-engineer",
    title: "Rotate at midnight",
    paths: [
      "crates/store",
    ],
    tasks: [
      {
        id: "A1",
        text: "Rotate the log file at midnight",
        state: "done",
        seen: true,
      },
      {
        id: "A2",
        text: "Keep seven files",
        state: "done",
        seen: true,
      },
    ],
  },
  {
    id: "f2",
    agent: "protocol-engineer",
    title: "Report the rotation",
    paths: [
      "crates/ipc",
      "packages/protocol",
    ],
    tasks: [
      {
        id: "B1",
        text: "Add rotated_at to the audit reply",
        state: "done",
        seen: true,
      },
      {
        id: "B2",
        text: "Mirror it in the TypeScript types",
        state: stage === 0 ? "working" : "done",
        seen: true,
      },
    ],
  },
  {
    id: "f3",
    agent: "bridge-engineer",
    title: "Show the rotation",
    paths: [
      "apps/desktop",
    ],
    tasks: [
      {
        id: "C1",
        text: "List rotated files in System",
        state: stage === 0 ? "working" : "done",
        seen: true,
      },
      {
        id: "C2",
        text: "Open a rotated file from the list",
        state: stage === 0 ? "open" : "done",
      },
    ],
    gate: "walk_approved",
    ...(stage < 2
      ? {}
      : {
          gateClosed: closedAt(
            "2026-10-26T14:06:00.000Z",
            "You approved the walk Audit files in System",
          ),
        }),
  },
  {
    id: "f4",
    agent: "rust-engineer",
    title: "Prune old files",
    paths: [
      "crates/fleet",
    ],
    tasks: [
      {
        id: "D1",
        text: "Delete files past the keep count",
        state: stage < 2 ? "open" : stage === 2 ? "working" : "done",
      },
      {
        id: "D2",
        text: "Test the prune",
        state: stage < 3 ? "open" : stage === 3 ? "working" : "done",
      },
    ],
    gate: "checks_green",
    ...(stage === 3
      ? {
          gateChecks: {
            commit: "9c41e2a",
            state: "red" as const,
            failed: [
              "prune_keeps_seven",
            ],
            already_red: [
              "tab-host.test.tsx",
            ],
          },
        }
      : stage === 4
        ? {
            gateClosed: closedAt("2026-10-26T14:21:00.000Z", "Checks passed at 1b500c4", {
              commit: "1b500c4",
              already_red: [
                "tab-host.test.tsx",
              ],
            }),
          }
        : {}),
  },
  {
    id: "f5",
    agent: "claude",
    title: "Deliver",
    paths: [],
    tasks: [
      {
        id: "E1",
        text: "Open the pull request",
        state: "open",
      },
    ],
    gate: "merged",
  },
];

/** The subagents the Field sees at work at each stage, over the files they edit. */
const field = (stage: number): WorkDrone[] =>
  stage === 0
    ? [
        {
          id: "protocol",
          agent: "protocol-engineer",
          task: "Mirror rotated_at in the TypeScript types",
          path: "packages/protocol",
          doing: "write",
          group: "f2",
          file: "packages/protocol/src/audit.ts",
        },
        {
          id: "bridge",
          agent: "bridge-engineer",
          task: "List rotated files in System",
          path: "apps/desktop",
          doing: "write",
          group: "f3",
          file: "apps/desktop/src/renderer/src/system/AuditFiles.tsx",
        },
      ]
    : stage === 2 || stage === 3
      ? [
          {
            id: "prune",
            agent: "rust-engineer",
            task: "Delete audit files past the keep count",
            path: "crates/fleet",
            doing: "write",
            group: "f4",
            file: "crates/fleet/src/prune.rs",
          },
          {
            id: "prune-test",
            agent: "test-writer",
            task: "Test that seven files are kept",
            path: "crates/fleet",
            doing: "write",
            group: "f4",
            file: "crates/fleet/src/tests/prune.rs",
          },
        ]
      : [];

const staged =
  (stage: number) =>
  (one: Session): Session =>
    one.work?.plan === undefined
      ? one
      : {
          ...one,
          work: {
            ...one.work,
            plan: {
              ...one.work.plan,
              groups: groups(stage),
              // Its subagents done and its turn ended, the Job is held on the walk's gate until he answers.
              ...(stage === 1
                ? {
                    held: {
                      group: "f3",
                      since: "2026-10-26T14:02:00.000Z",
                    },
                  }
                : {}),
            },
            field: field(stage),
          },
        };

export const s348PlanAsProgress: Scenario = {
  ...base,
  name: "plan-as-progress",
  says: "A Job whose plan is its workflow: Now one row a chunk, each ended by its gate, what is being done under the live one",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(BUSY, (one) =>
        staged(0)(
          auditLedger(
            one,
            {
              steps: [
                "Feature",
              ],
              at: 0,
            },
            [],
          ),
        ),
      );
      let stage = 0;
      return {
        ...store,
        // Time passes: the Job is held on the walk, the walk is approved, then Checks fail, then they pass.
        later: () => {
          if (stage >= 4) return;
          stage += 1;
          store.edit(BUSY, staged(stage));
        },
      };
    },
  },
};
