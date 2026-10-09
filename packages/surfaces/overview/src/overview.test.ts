// Overview's readings, case by case: the machine's three ignore a pick. Overview 27 (#1091) retired the tile band these once drew for; `left-column.ts` reads them
// into the left column's Fleet panel now.

import { describe, expect, it } from "vitest";

import type { Connection, FleetHealth } from "@armada/protocol";
import { connectedTo, PROTOCOL_ID } from "@armada/protocol";
import { doctorReading, dronesReading, fleetReading } from "./overview";

const NOW = Date.parse("2026-09-13T12:00:00Z");
const FLEET = { protocolId: PROTOCOL_ID, pid: 4242, port: 7878, startedAt: "2026-09-13T11:00:00Z" };
const CONNECTED: Connection = connectedTo(FLEET, 1);

describe("the Fleet tile", () => {
  it("reads the connection's own statement, in the status bar's three hues", () => {
    expect(fleetReading(CONNECTED, NOW, null)).toMatchObject({ value: "Fleet running", tone: "completed-success" });
    const unreachable: Connection = { state: "unreachable", fleet: FLEET, detail: "", sinceMs: NOW - 20_000 };
    expect(fleetReading(unreachable, NOW, null)).toMatchObject({ value: "Fleet unreachable", tone: "awaiting-review" });
    const down: Connection = { state: "not_running", absence: { why: "no_runtime_file", path: "fleet.json" } };
    expect(fleetReading(down, NOW, null)).toMatchObject({ value: "Fleet is not running", tone: "completed-failed" });
    expect(fleetReading({ state: "reading" }, NOW, null).tone).toBeUndefined();
  });
});

describe("the Doctor tile", () => {
  const health = (...outcomes: [string, string][]): FleetHealth => ({
    probes: outcomes.map(([module, outcome]) => ({ module, outcome, detail: "read" })),
    not_probed: [{ owner: "adapters", because: "Doctor is not built" }],
    helm_action_authority: "acting",
  });

  it("says the worst of Doctor's words and names every module that did not pass", () => {
    const reading = doctorReading({ state: "read", health: health(["Fleet", "pass"], ["SQLite", "fail"], ["Manifest", "warn"]) });
    expect(reading).toMatchObject({ value: "fail", tone: "completed-failed", detail: "SQLite: fail · Manifest: warn" });
  });

  it("names what was probed when all of it passes", () => {
    const reading = doctorReading({ state: "read", health: health(["Fleet", "pass"], ["SQLite", "pass"]) });
    expect(reading).toMatchObject({ value: "pass", tone: "completed-success", detail: "Fleet, SQLite" });
  });

  it("draws a word it does not know as itself, with no hue", () => {
    const reading = doctorReading({ state: "read", health: health(["Kit", "skipped"]) });
    expect(reading).toMatchObject({ value: "skipped", detail: "Kit: skipped" });
    expect(reading.tone).toBeUndefined();
  });

  it("stands in until it is read", () => {
    expect(doctorReading({ state: "reading" }).value).toBeUndefined();
  });
});

describe("the Drones tile", () => {
  it("names the hold only while something is queued", () => {
    const full = { bound: 2, occupied: 2, held_by: "memory" };
    expect(dronesReading(CONNECTED, full, 1)).toMatchObject({ value: "2 of 2", detail: "waiting on memory" });
    expect(dronesReading(CONNECTED, full, 0)).toMatchObject({ detail: "None free" });
    expect(dronesReading(CONNECTED, { bound: 4, occupied: 1 }, 0)).toMatchObject({ detail: "3 free" });
  });

  it("stands in while connected and unread, and says not read otherwise", () => {
    expect(dronesReading(CONNECTED, null, 0).value).toBeUndefined();
    expect(dronesReading({ state: "reading" }, null, 0).value).toBe("Not read");
  });
});
