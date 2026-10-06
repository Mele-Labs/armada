import { expect, test } from "vitest";

import type { CheckoutRunRecord, CheckoutRunSheet } from "@armada/protocol";

import { askerOf, checkDetailsOf, checkEntriesOf, checkRowOf } from "./manifest-checks";

const entry = (name: string) => ({ name, run: name, narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true });

function ran(name: string, at: string, over: Partial<CheckoutRunRecord> = {}): CheckoutRunRecord {
  return {
    id: `crun_${name}`, name, command: name, required: [], started_at: at, ended_at: at, duration_ms: 2000,
    exit_code: 0, expect_exit_code: 0, ended: "exited 0", stopped: false, changed: [], undoable: false, log: "x", ...over,
  };
}

const sheet: CheckoutRunSheet = {
  setup: [entry("bootstrap")],
  checks: [entry("build"), entry("test"), entry("format")],
  commands: [entry("fmt")],
  running: { id: "crun_test", name: "test", command: "test", started_at: "2026-10-06T14:00:00Z" },
  verify: {
    id: "v",
    started_at: "2026-10-06T13:59:00Z",
    steps: [
      { group: "checks", name: "test", run: "test", state: "running", run_id: "crun_test" },
      { group: "checks", name: "format", run: "format", state: "waiting" },
    ],
  },
};

test("a Check is listed out, then waiting, then ended newest first, and a Command or Setup entry is not", () => {
  const runs = [ran("fmt", "2026-10-06T14:05:00Z"), ran("build", "2026-10-06T13:00:00Z"), ran("bootstrap", "2026-10-06T12:00:00Z"), ran("build", "2026-10-06T13:30:00Z", { id: "crun_build2", exit_code: 1 })];
  const rows = checkEntriesOf(sheet, runs).map((one) => checkRowOf(one));
  expect(rows.map((one) => [one.name, one.status])).toEqual([
    ["test", "running"],
    ["format", "waiting"],
    ["build", "failed"],
    ["build", "passed"],
  ]);
});

test("a Check nobody asked for has no row", () => {
  expect(checkEntriesOf({ ...sheet, running: undefined, verify: undefined }, [])).toEqual([]);
});

test("a stopped run is stopped, and a run with no exit code says what ended it", () => {
  const [one] = checkEntriesOf({ ...sheet, running: undefined, verify: undefined }, [ran("build", "2026-10-06T13:00:00Z", { exit_code: undefined, stopped: true, ended: "stopped" })]);
  expect(checkRowOf(one!).status).toBe("stopped");
  expect(checkDetailsOf(one!).map((fact) => fact.label)).toContain("Ended by");
});

const label = (id: string) => `job-${id}`;

test("who asked is a fact per kind, a link only where a route exists", () => {
  expect(askerOf({ kind: "gate", job_id: "1", step: "implement" }, label).parts).toEqual(["Gate", { link: "job", label: "job-1", jobId: "1" }, "implement"]);
  expect(askerOf({ kind: "drone_task", job_id: "1", step: "implement", task_id: "T3", drone_id: "d9" }, label).parts).toEqual([
    "Drone d9",
    { link: "job", label: "job-1", jobId: "1" },
    "implement",
    "T3",
  ]);
  expect(askerOf({ kind: "drone_step", job_id: "1", step: "fix", drone_id: "d9" }, label).parts).toEqual(["Drone d9", { link: "job", label: "job-1", jobId: "1" }, "fix"]);
  expect(askerOf({ kind: "merge_line", branch: "b" }, label).parts).toEqual([{ link: "merge-line", label: "Merge line" }, "b"]);
  expect(askerOf({ kind: "outside" }, label).parts).toEqual(["Started outside a Job"]);
});

test("a merge line Check is a row with the line's requester and the names its log is asked by", () => {
  const line = { root: "/r", line: [{ branch: "b", place: 1, state: "gating", checks: [{ name: "test", state: "timed_out", requester: { kind: "merge_line", branch: "b" } }] }], off: [], landed: [], sent_back: [] };
  const [one] = checkEntriesOf(undefined, [], [line]);
  expect(one?.land).toEqual({ root: "/r", branch: "b", check: "test" });
  expect(checkRowOf(one!)).toMatchObject({ status: "failed", says: "timed out", by: "Merge line · b" });
});
