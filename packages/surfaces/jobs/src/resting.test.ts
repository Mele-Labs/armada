// A Drone at rest is not a Drone at work (protocol 23.11, `at_rest_since`).
// The owner on Job 3, 3 Oct 2026: "Drone on T4 is still running but I think
// it's actually done" — it was resting at the gate while its Checks ran.

import { describe, expect, it } from "vitest";

import type { PulseView } from "./draft/pulse";
import { droneViewsOf, type DroneView } from "./draft/drone";
import { pulseReadingOf } from "./resources";
import { droneSays, dronesUnder } from "./tab-drones-read";

const RESTED = "2026-09-22T10:40:00Z";
const drone = (over: Partial<DroneView>): DroneView => ({ id: "d1", step: "implement", state: "running", ...over });

function view(drone: string): PulseView {
  return {
    job: "01J",
    read_at: "2026-09-22T10:30:00.000Z",
    held: "running",
    processes: [
      { pid: 1, command: "node", cpu_percent: 0, memory_bytes: 1, running_for: "04:12", recorded: true, drone, owner: "b" },
    ],
    worktrees: [{ path: "/repo/.armada/worktrees/1", branch: "b", bytes: 1 }],
    logs: [],
  };
}

describe("a Drone at rest", () => {
  it("carries at_rest_since off the wire", () => {
    const [one] = droneViewsOf({
      job_id: "01J",
      drones: [{ drone_id: "d1", step_id: "implement", state: "running", since: RESTED, at_rest_since: RESTED }],
    });
    expect(one?.at_rest_since).toBe(RESTED);
  });

  it("is said to be at rest, not running", () => {
    expect(droneSays(drone({ at_rest_since: RESTED }))).toBe("At rest");
    expect(droneSays(drone({}))).toBe("Running");
  });

  it("is not sorted with the Drones at work", () => {
    const order = dronesUnder(
      [drone({ id: "a", task: "T1", at_rest_since: RESTED }), drone({ id: "b", task: "T2" })],
      "all",
      "running",
    );
    expect(order.map((one) => one.id)).toEqual(["b", "a"]);
  });

  it("is not counted as a Drone working on Pulse", () => {
    const [resting] = pulseReadingOf(view("d1"), null, null, undefined, new Map(), new Set(["d1"])).worktrees;
    expect(resting?.state).toBe("no drone working");
    const [working] = pulseReadingOf(view("d1"), null, null, undefined, new Map(), new Set()).worktrees;
    expect(working?.state).toBe("1 drone working");
  });
});
