// The Checks filter row's rules: what each filter holds, and a Job's narrowing.

import { expect, test } from "vitest";
import type { ManifestCheckRow, MergeLines } from "@armada/protocol";

import { CHECK_FILTERS, checkEntriesOf, entryOfReported, heldBy, ofJob, type CheckEntry } from "./manifest-checks";

function row(source: string, state: string, job = "J1"): ManifestCheckRow {
  return {
    source,
    requester: { kind: "gate", job_id: job },
    job_id: job,
    job_handle: job,
    job_title: job,
    step: "s",
    attempt: 1,
    name: state,
    state,
  };
}

const one = (source: string, state: string): CheckEntry => entryOfReported(row(source, state));

test("each filter holds the states it names, and Completed holds passed, failed, skipped and stopped", () => {
  const states: [string, string][] = [
    ["asked_run", "running"],
    ["gate", "passed"],
    ["gate", "failed"],
    ["gate", "timed_out"],
    ["gate", "skipped"],
    ["gate", "never_ran"],
    ["asked_run", "stopped"],
  ];
  const held = (filter: (typeof CHECK_FILTERS)[number]) =>
    states.filter(([source, state]) => heldBy(filter, one(source, state))).map(([, state]) => state);
  expect(held("all")).toHaveLength(7);
  expect(held("active")).toEqual(["running"]);
  expect(held("waiting")).toEqual([]);
  expect(held("passed")).toEqual(["passed"]);
  expect(held("failed")).toEqual(["failed", "timed_out"]);
  expect(held("skipped")).toEqual(["skipped", "never_ran"]);
  expect(held("completed")).toEqual(["passed", "failed", "timed_out", "skipped", "never_ran", "stopped"]);
});

test("a gate row or a Drone's run Fleet reports waiting is Waiting, and a gate row running is Active", () => {
  const held = (filter: (typeof CHECK_FILTERS)[number], source: string, state: string) => heldBy(filter, one(source, state));
  for (const source of ["gate", "asked_run"]) {
    expect(held("waiting", source, "waiting")).toBe(true);
    expect(held("completed", source, "waiting")).toBe(false);
  }
  expect(held("active", "gate", "running")).toBe(true);
});

test("a Check asked for and not started is Waiting alone", () => {
  const waiting: CheckEntry = { id: "w", name: "w", status: "waiting", command: "", requester: { kind: "outside" } };
  expect(CHECK_FILTERS.filter((filter) => heldBy(filter, waiting))).toEqual(["all", "waiting"]);
});

test("a Job's Checks are the reported rows with its id and any entry whose requester names it", () => {
  const mine = one("gate", "passed");
  const other = entryOfReported(row("gate", "passed", "J2"));
  const outside: CheckEntry = { id: "o", name: "o", status: "passed", command: "", requester: { kind: "outside" } };
  const line: CheckEntry = { id: "l", name: "l", status: "waiting", command: "", requester: { kind: "merge_line", job_id: "J1" } };
  expect(ofJob([mine, other, outside, line], "J1")).toEqual([mine, line]);
});

test("a Job whose branch is queued behind another has its Checks in Waiting, as Fleet lists them", () => {
  const requester = { kind: "merge_line", branch: "fleet/mine", job_id: "J1", handle: "7-mine" };
  const lines: MergeLines = {
    lines: [
      {
        root: "/repo",
        line: [
          { branch: "fleet/ahead", place: 1, state: "gating", checks: [{ name: "ipc_test", state: "running", started_at: "2026-10-07T10:00:00Z", requester: { kind: "merge_line", branch: "fleet/ahead", job_id: "J0" } }] },
          { branch: "fleet/mine", place: 2, state: "waiting", checks: [{ name: "ipc_test", state: "waiting", requester }, { name: "fleet_test", state: "waiting", requester }] },
        ],
        off: [],
        landed: [],
        sent_back: [],
      },
    ],
  };
  const mine = ofJob(checkEntriesOf(undefined, [], lines.lines), "J1");
  expect(mine.filter((entry) => heldBy("waiting", entry)).map((entry) => entry.name)).toEqual(["ipc_test", "fleet_test"]);
  expect(mine.filter((entry) => heldBy("active", entry))).toEqual([]);
});
