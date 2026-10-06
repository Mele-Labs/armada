import { expect, test } from "vitest";

import type { CheckoutRunRecord, CheckoutRunSheet } from "@armada/protocol";

import { checkDetailsOf, checkEntriesOf, checkRowOf } from "./manifest-checks";

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
  const rows = checkEntriesOf(sheet, runs).map(checkRowOf);
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
