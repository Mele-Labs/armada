// Doctor's grid from what Bridge holds: the doc's order, Armada API from the connection, and the
// probes nobody runs kept apart from the three words.

import { describe, expect, it } from "vitest";
import type { Connection, FleetHealth } from "@armada/protocol";

import { gridOf, readingOf, worstOf } from "./grid";

const FLEET = { pid: 4242, port: 7878, protocolId: "p", startedAt: "", runId: "r" } as unknown as Extract<Connection, { state: "connected" }>["fleet"];
const CONNECTED: Connection = { state: "connected", fleet: FLEET, cursor: 0 };

const HEALTH: FleetHealth = {
  probes: [
    { module: "System stats", outcome: "warn", detail: "memory 91% in use" },
    { module: "Fleet", outcome: "pass", detail: "answering" },
    { module: "Manifest", outcome: "pass", detail: "/a/armada.yml" },
    { module: "Manifest", outcome: "fail", detail: "/b/armada.yml would not parse" },
    { module: "SQLite", outcome: "pass", detail: "opens" },
  ],
  not_probed: [
    { owner: "adapters", because: "none is built" },
    { owner: "config", because: "neither is built" },
  ],
  helm_action_authority: "acting",
};

describe("gridOf", () => {
  it("draws the rows in the doc's order, a Manifest per repository, and Armada API from the connection", () => {
    const grid = gridOf({ state: "read", health: HEALTH }, CONNECTED, "Running");
    expect(grid.rows.map((row) => `${row.module} ${row.result}`)).toEqual([
      "Fleet pass",
      "Armada API pass",
      "Manifest pass",
      "Manifest fail",
      "SQLite pass",
      "System stats warn",
    ]);
    expect(grid.rows[1]?.detail).toBe("connected on port 7878");
    expect(worstOf(grid.rows)).toBe("fail");
  });

  it("keeps the probes nobody runs out of the results, as Fleet groups them, and Bridge's own as a row", () => {
    const grid = gridOf({ state: "read", health: { ...HEALTH, not_probed: [...HEALTH.not_probed, { owner: "Bridge", because: "a client's own state" }] } }, CONNECTED, "Running");
    expect(grid.gaps).toEqual([
      { owner: "adapters", because: "none is built" },
      { owner: "config", because: "neither is built" },
    ]);
    expect(grid.rows.some((row) => row.module === "Kit")).toBe(false);
  });

  it("reads while the probes are in flight, and fails Fleet and Armada API from outside when Fleet is down", () => {
    expect(gridOf({ state: "reading" }, CONNECTED, "Running").rows.every((row) => row.result === "reading" || row.module === "Armada API")).toBe(true);
    const down = gridOf({ state: "failed", outcome: { ok: false, why: "not_connected" } }, { state: "not_running", absence: "no_file" } as unknown as Connection, "Not running");
    expect(down.rows.map((row) => `${row.module} ${row.result}`)).toEqual(["Fleet fail", "Armada API fail"]);
    expect(down.unread).toBeDefined();
    expect(down.gaps[0]?.owner).toBe("fleet");
  });

  it("keeps a module this build does not know, rather than hiding it", () => {
    const grid = gridOf({ state: "read", health: { ...HEALTH, probes: [...HEALTH.probes, { module: "Network", outcome: "fail", detail: "offline" }] } }, CONNECTED, "Running");
    // Not dropped, and not drawn among the doc's own rows either: after them.
    expect(grid.rows.at(-1)).toMatchObject({ module: "Network", result: "fail" });
  });
});

describe("readingOf", () => {
  it("names the values in a probe's line, and keeps the rest as sentences", () => {
    expect(readingOf("cpu 64% in use, memory 91% in use, 18874368 KiB free on the volume")).toEqual({
      lines: [],
      facts: [
        { label: "CPU", value: "64% used" },
        { label: "Memory", value: "91% used" },
        { label: "Disk", value: "18.0 GB free" },
      ],
    });
    expect(readingOf("/Users/user/ledger/armada.yml would not re-read: checks.test names no command")).toEqual({
      lines: ["Would not re-read: checks.test names no command"],
      facts: [{ label: "File", value: "ledger/armada.yml", mono: true, full: "/Users/user/ledger/armada.yml" }],
    });
  });

  it("keeps a line no rule knows word for word, so nothing a probe says is lost", () => {
    expect(readingOf("opens; 42 Jobs read back")).toEqual({ lines: ["Opens", "42 Jobs read back"], facts: [] });
    expect(readingOf("")).toEqual({ lines: [], facts: [] });
  });
});
