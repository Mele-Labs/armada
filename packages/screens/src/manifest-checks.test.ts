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

test("Checks are listed by when each was requested, and a Command or Setup entry is not", () => {
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

test("every kind but outside opens its requester", () => {
  expect(askerOf({ kind: "gate", job_id: "1", step: "implement" }, label)).toEqual({
    label: "Gate · job-1 · implement",
    opens: { to: "job", jobId: "1", step: "implement" },
  });
  expect(askerOf({ kind: "drone_task", job_id: "1", step: "implement", task_id: "T3", drone_id: "d9" }, label)).toEqual({
    label: "Drone d9 · job-1 · implement · T3",
    opens: { to: "job", jobId: "1", task: "T3" },
  });
  expect(askerOf({ kind: "drone_task", job_id: "1", step: "implement", drone_id: "d9" }, label).opens).toEqual({ to: "job", jobId: "1", drone: "d9", step: "implement" });
  expect(askerOf({ kind: "drone_step", job_id: "1", step: "fix", drone_id: "d9" }, label).opens).toEqual({ to: "job", jobId: "1", drone: "d9", step: "fix" });
  expect(askerOf({ kind: "merge_line", branch: "b" }, label)).toEqual({ label: "Merge line · b", opens: { to: "merge-line", branch: "b" } });
  expect(askerOf({ kind: "outside" }, label)).toEqual({ label: "Started outside a Job" });
});

test("mixed sources sort newest request first, with their requester and handle", () => {
  const base = { job_id: "1", job_handle: "1-a-job", job_title: "A job", step: "fix", attempt: 1 };
  const reported = [
    { ...base, source: "gate", name: "test", requester: { kind: "gate", job_id: "1", step: "fix", handle: "1-a-job" }, state: "timed_out", ended_at: "2026-10-06T13:00:00Z" },
    { ...base, source: "asked_run", name: "build", requester: { kind: "drone_step", job_id: "1", step: "fix", drone_id: "d", handle: "1-a-job" }, state: "lost", started_at: "2026-10-06T14:01:00Z", logs: [{ check: "build", kept: "k.log" }], asked_run_id: 4 },
  ];
  const line = { root: "/r", line: [{ branch: "b", place: 1, state: "gating", checks: [{ name: "lint", state: "running", started_at: "2026-10-06T14:00:30Z", requester: { kind: "merge_line", branch: "b" } }, { name: "slow", state: "waiting", requester: { kind: "merge_line", branch: "b" } }] }], off: [], landed: [], sent_back: [] };
  const entries = checkEntriesOf(sheet, [ran("build", "2026-10-06T13:30:00Z")], [line], reported);
  const rows = entries.map((one) => checkRowOf(one, label));
  // One list, newest request first across every source; the line's waiting Check takes its turn's start.
  expect(rows.map((one) => [one.name, one.by, one.says])).toEqual([
    ["build", "Drone d · 1-a-job · fix", "lost"],
    ["lint", "Merge line · b", "running"],
    ["slow", "Merge line · b", "waiting"],
    ["test", "Started outside a Job", "running"],
    ["format", "Started outside a Job", "waiting"],
    ["build", "Started outside a Job", "passed"],
    ["test", "Gate · 1-a-job · fix", "outran its budget"],
  ]);
  expect(entries[0]?.logs).toEqual([{ check: "build", kept: "k.log" }]);
  expect(entries[0]?.job).toEqual({ id: "1", handle: "1-a-job" });
});

test("a merge line Check is a row with the line's requester and the names its log is asked by", () => {
  const line = { root: "/r", line: [{ branch: "b", place: 1, state: "gating", checks: [{ name: "test", state: "timed_out", requester: { kind: "merge_line", branch: "b" } }] }], off: [], landed: [], sent_back: [] };
  const [one] = checkEntriesOf(undefined, [], [line]);
  expect(one?.land).toEqual({ root: "/r", branch: "b", check: "test" });
  expect(checkRowOf(one!)).toMatchObject({ status: "failed", says: "timed out", by: "Merge line · b" });
});
