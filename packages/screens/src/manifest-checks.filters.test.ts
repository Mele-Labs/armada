// The Checks filter row's rules: what each filter holds.

import { expect, test } from "vitest";
import type { CheckListStatus } from "@armada/components";
import type { MergeLines } from "@armada/protocol";

import { CHECK_FILTERS, checkEntriesOf, heldBy, type CheckEntry } from "./manifest-checks";

const entryAt = (status: CheckListStatus): CheckEntry => ({ id: status, name: status, status, command: "", requester: { kind: "outside" } });

test("each filter holds the statuses it names, and Completed holds passed, failed and stopped", () => {
  const statuses: CheckListStatus[] = ["running", "waiting", "passed", "failed", "stopped"];
  const held = (filter: (typeof CHECK_FILTERS)[number]) => statuses.filter((status) => heldBy(filter, entryAt(status)));
  expect(held("all")).toHaveLength(5);
  expect(held("active")).toEqual(["running"]);
  expect(held("waiting")).toEqual(["waiting"]);
  expect(held("passed")).toEqual(["passed"]);
  expect(held("failed")).toEqual(["failed"]);
  expect(held("completed")).toEqual(["passed", "failed", "stopped"]);
});

test("a Check asked for and not started is Waiting alone", () => {
  const waiting: CheckEntry = { id: "w", name: "w", status: "waiting", command: "", requester: { kind: "outside" } };
  expect(CHECK_FILTERS.filter((filter) => heldBy(filter, waiting))).toEqual(["all", "waiting"]);
});

test("a branch queued behind another has its Checks in Waiting, as Fleet lists them", () => {
  const requester = { kind: "merge_line", branch: "fleet/mine" };
  const lines: MergeLines = {
    lines: [
      {
        root: "/repo",
        line: [
          { branch: "fleet/ahead", place: 1, state: "gating", checks: [{ name: "ipc_test", state: "running", started_at: "2026-10-07T10:00:00Z", requester: { kind: "merge_line", branch: "fleet/ahead" } }] },
          { branch: "fleet/mine", place: 2, state: "waiting", checks: [{ name: "ipc_test", state: "waiting", requester }, { name: "fleet_test", state: "waiting", requester }] },
        ],
        off: [],
        landed: [],
        sent_back: [],
      },
    ],
  };
  const mine = checkEntriesOf(undefined, [], lines.lines).filter((entry) => entry.requester.branch === "fleet/mine");
  expect(mine.filter((entry) => heldBy("waiting", entry)).map((entry) => entry.name)).toEqual(["ipc_test", "fleet_test"]);
  expect(mine.filter((entry) => heldBy("active", entry))).toEqual([]);
});
