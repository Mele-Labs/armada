// Which Drones a Job has: those Fleet lists, each with its own rows, and the
// Job's own where the list does not hold it.

import { describe, expect, it } from "vitest";

import type { JobDrones, Turn } from "@armada/protocol";

import { droneViewsOf } from "./drone";
import { sampleDetail, sampleJob, samplePlan, sampleStep, sampleTask } from "./sample";

const DRONE = "01M3WJ6FGZ003DCX123T7W6YP1";

function onPlan(tasks = [sampleTask({ id: "T1" })], assigned: string | null = DRONE) {
  const plan = sampleStep({
    step_id: "plan",
    ordinal: 0,
    attempts: [{ attempt: 1, outcome: "running", started_at: "2026-10-01T20:23:51.575Z" }],
  });
  return sampleDetail({
    job: sampleJob({ current_step_id: "plan", ...(assigned === null ? {} : { assigned_drone: assigned }) }),
    steps: [plan, sampleStep()],
    work_plan: samplePlan(tasks),
  });
}

/** `crates/api/src/tests/shapes.rs`' `job_drones`, on this Job's running Drone. */
const LISTED: JobDrones = {
  job_id: "01M3WJ6FGZ003DCX123T7W6YP0",
  drones: [
    {
      drone_id: "01DRONEKILLED",
      step_id: "implement",
      state: "killed",
      since: "2026-09-11T08:00:00Z",
      ended_at: "2026-09-11T08:20:00Z",
      turns: 7,
      cost_micros: 146_473,
    },
    { drone_id: DRONE, step_id: "plan", state: "running", since: "2026-09-11T09:00:00Z", turns: 3, cost_micros: 41_002 },
  ],
};

function row(seq: number, drone_id?: string): Turn {
  return {
    ts: "2026-09-11T09:01:00Z",
    seq,
    by: "drone",
    ...(drone_id === undefined ? {} : { drone_id }),
    saw: { event: "said", text: `row ${seq}` },
  };
}

describe("the list Fleet serves", () => {
  it("is every Drone it names, in its order, with its names", () => {
    expect(droneViewsOf(LISTED)).toEqual([
      {
        id: "01DRONEKILLED",
        step: "implement",
        state: "killed",
        since: "2026-09-11T08:00:00Z",
        ended_at: "2026-09-11T08:20:00Z",
        turns: 7,
        cost_micros: 146_473,
      },
      { id: DRONE, step: "plan", state: "running", since: "2026-09-11T09:00:00Z", turns: 3, cost_micros: 41_002 },
    ]);
  });

  it("gives each Drone its own rows, and no Drone a row with no id", () => {
    const turns = [row(1, "01DRONEKILLED"), row(2, DRONE), row(3), row(4, DRONE)];

    const [killed, running] = droneViewsOf(LISTED, undefined, turns);

    expect(killed?.transcript?.map((one) => one.seq)).toEqual([1]);
    expect(running?.transcript?.map((one) => one.seq)).toEqual([2, 4]);
  });

  it("holds no transcript where the Job's turns are not in hand", () => {
    expect(droneViewsOf(LISTED).every((one) => one.transcript === undefined)).toBe(true);
  });
});

describe("the Job's own Drone", () => {
  it("is listed running on the step, timed from the step's live run, where the list is not read", () => {
    const whole = onPlan();

    expect(droneViewsOf(undefined, whole)).toEqual([
      { id: DRONE, step: "plan", state: "running", since: "2026-10-01T20:23:51.575Z" },
    ]);
  });

  it("is not listed twice where the list already names it", () => {
    const whole = onPlan();

    expect(droneViewsOf(LISTED, whole).map((one) => one.id)).toEqual(["01DRONEKILLED", DRONE]);
  });

  it("is absent where no process is on the Job, and where the Job is not given", () => {
    const idle = onPlan(undefined, null);

    expect(droneViewsOf(undefined, idle)).toEqual([]);
    expect(droneViewsOf(undefined)).toEqual([]);
  });
});
