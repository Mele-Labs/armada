// Which Drones a Job has, from today's wire: those its tasks name, and its own.

import { describe, expect, it } from "vitest";

import { droneViewsOf } from "./drone";
import { taskGroupsOf } from "./group";
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

describe("the Job's own Drone", () => {
  it("is listed running on the step, timed from the step's live run, where no task names it", () => {
    const whole = onPlan();

    expect(droneViewsOf(taskGroupsOf(whole), whole)).toEqual([
      { id: DRONE, step: "plan", state: "running", since: "2026-10-01T20:23:51.575Z" },
    ]);
  });

  it("is not listed twice where a task already names it", () => {
    const whole = onPlan([sampleTask({ id: "T1", state: "working" })]);

    expect(droneViewsOf(taskGroupsOf(whole), whole).map((one) => [one.id, one.task])).toEqual([[DRONE, "T1"]]);
  });

  it("is absent where no process is on the Job, and where the Job is not given", () => {
    const idle = onPlan(undefined, null);

    expect(droneViewsOf(taskGroupsOf(idle), idle)).toEqual([]);
    expect(droneViewsOf(taskGroupsOf(onPlan()))).toEqual([]);
  });
});
