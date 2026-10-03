// A feature Job whose tasks each ran on the model their tier, or a person,
// picked, **in the shape Fleet serves it since 23.6** (spike 022, slice 3):
// `PlanTask.tier` on three tasks and none on the fourth, which is Armada
// picking; `JobDetail.tiers` naming two tiers and leaving `medium` out; a
// person's pick on T4, the task that failed; and every Drone's
// `JobDrone.model`. The plan is `featureRunInGroups`', so the groups and the
// failure are slice 2's.

import type { JobDrones, PlanTask } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { featureRunInGroups } from "./job-groups-fixture";

const AT = (minute: number) => `2026-10-02T14:${String(minute).padStart(2, "0")}:00.000Z`;

/** Each task's tier as the planner gave it. T3's is left out: Armada picks. */
const TIERS: Record<string, string> = { T1: "difficult", T2: "easy", T4: "medium" };

/** Which Drone worked which task, and the model each was spawned as. */
const RAN: readonly { task: string; drone: string; model: string; from: number; to: number }[] = [
  { task: "T1", drone: "01M2TIERS0000DRONE00000T1", model: "opus", from: 1, to: 5 },
  { task: "T2", drone: "01M2TIERS0000DRONE00000T2", model: "haiku", from: 5, to: 9 },
  { task: "T3", drone: "01M2TIERS0000DRONE00000T3", model: "sonnet", from: 10, to: 15 },
  // The Job's own Drone, idle on the stopped step as Fleet keeps it: no exit yet.
  { task: "T4", drone: "01M3WJ6FGZ003DCX123T7W6YP1", model: "sonnet", from: 16, to: 0 },
];

/** The groups Job, its tasks tiered, its map set, T4's model picked by a person, and its Drones. */
export function featureWithTiers(): JobFixture {
  const base = featureRunInGroups();
  if (base.watched.state !== "read") return base;
  const whole = base.watched.detail;
  const plan = whole.work_plan!;
  const tasks: PlanTask[] = plan.tasks.map((task) => {
    const tier = TIERS[task.id];
    const tiered = tier === undefined ? task : { ...task, tier };
    // A person's pick after the failure, over the map: T4 runs on opus next.
    return task.id === "T4" ? { ...tiered, model: "opus" } : tiered;
  });
  const job = { ...base.job, title: "Run each task on the model its tier names" };
  const detail = {
    ...whole,
    job,
    tiers: { difficult: "opus", easy: "haiku" },
    work_plan: { ...plan, tasks },
  };
  const drones: JobDrones = {
    job_id: job.id,
    drones: RAN.map((one) => ({
      drone_id: one.drone,
      step_id: "implement",
      task: one.task,
      model: one.model,
      since: AT(one.from),
      ...(one.to === 0 ? { state: "running" as const } : { state: "done" as const, ended_at: AT(one.to) }),
    })),
  };
  return {
    ...base,
    name: "A plan whose tasks each ran on their tier's model",
    job,
    watched: { ...base.watched, detail },
    jobDrones: { state: "read", jobId: job.id, drones },
  };
}
