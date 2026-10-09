// `fleetPanelOf`'s rows, on their own — no window, no Fleet socket.
// Bridge/1114: #1088 shipped the panel with `pid · port` and dropped the
// second line the mock draws under it. Settled 2026-09-17: the two lines are
// now four rows, and a state draws only the rows it has a value for.

import { expect, test } from "vitest";
import type { Connection } from "@armada/protocol";
import type { HealthRead } from "@armada/screens/src/overview-reads";
import type { Statement } from "@armada/shell";
import { fleetPanelOf } from "./left-column";

const STATEMENT: Statement = { headline: "Fleet running", detail: "pid 1 · port 2", next: null };
const NO_HEALTH: HealthRead = { state: "none" };

const CONNECTED: Connection = {
  state: "connected",
  fleet: {
    protocolId: "0000000000000000",
    pid: 61372,
    port: 40000,
    startedAt: "Mon Sep 14 14:22:06 2026",
  },
  cursor: 0,
};

function pairs(panel: ReturnType<typeof fleetPanelOf>): [string, string][] | undefined {
  return panel.rows?.map((row) => [row.label, row.value]);
}

test("a connected Fleet reads four rows: pid, port, protocol and a live uptime", () => {
  // Two hours and fourteen minutes after `startedAt`.
  const now = Date.parse("Mon Sep 14 14:22:06 2026") + (2 * 60 + 14) * 60_000;
  const panel = fleetPanelOf(CONNECTED, STATEMENT, NO_HEALTH, now, null);
  expect(pairs(panel)).toEqual([
    ["pid", "61372"],
    ["port", "40000"],
    ["protocol", "00000000"],
    ["up", "2h 14m"],
  ]);
  expect(panel.detail).toBeUndefined();
});

test("the up row ticks: two reads a minute apart move it a minute", () => {
  const first = Date.parse("Mon Sep 14 14:22:06 2026") + 60_000;
  const later = first + 60_000;
  const before = pairs(fleetPanelOf(CONNECTED, STATEMENT, NO_HEALTH, first, null));
  const after = pairs(fleetPanelOf(CONNECTED, STATEMENT, NO_HEALTH, later, null));
  expect(before).not.toEqual(after);
});

test("a startedAt that will not parse drops the up row rather than drawing it blank", () => {
  const unparsable: Connection = {
    ...CONNECTED,
    fleet: { ...CONNECTED.fleet, startedAt: "not a date" },
  };
  const panel = fleetPanelOf(unparsable, STATEMENT, NO_HEALTH, Date.now(), null);
  expect(panel.rows?.map((row) => row.label)).toEqual(["pid", "port", "protocol"]);
});

test("a Fleet on another protocol has no rows, only what the statement says", () => {
  const mismatch: Connection = {
    state: "protocol_mismatch",
    fleet: CONNECTED.fleet,
    speaks: "3fa9c1d200000000",
    expected: "91bb07e400000000",
  };
  const panel = fleetPanelOf(mismatch, { ...STATEMENT, detail: "Fleet 3fa9c1d2 · Bridge 91bb07e4" }, NO_HEALTH, Date.now(), null);
  expect(panel.rows).toBeUndefined();
  expect(panel.detail).toBe("Fleet 3fa9c1d2 · Bridge 91bb07e4");
});

test("an unreachable Fleet reads pid and port, and how long it has been silent", () => {
  const now = 100_000;
  const unreachable: Connection = { state: "unreachable", fleet: CONNECTED.fleet, detail: "", sinceMs: now - 20_000 };
  const panel = fleetPanelOf(unreachable, STATEMENT, NO_HEALTH, now, now - 4_000);
  expect(pairs(panel)).toEqual([
    ["pid", "61372"],
    ["port", "40000"],
  ]);
  expect(panel.detail).toBe("alive, no answer for 20s · last read 4s ago");
});

test("a Fleet that is not running has no rows, only what the runtime file says", () => {
  const notRunning: Connection = { state: "not_running", absence: { why: "no_runtime_file", path: "~/x" } };
  const said: Statement = { headline: "Fleet is not running", detail: "no runtime file at ~/x", next: null };
  const panel = fleetPanelOf(notRunning, said, NO_HEALTH, Date.now(), null);
  expect(panel.rows).toBeUndefined();
  expect(panel.detail).toBe("no runtime file at ~/x");
});
