// Every Drone the arc's Job has used, each with the transcript it wrote.
//
// **One Drone per task, and one task that had two.** T2's first Drone was
// ended by hand eight minutes in and the task was run again by a second, so
// the list holds nine Drones for eight tasks — which is why it is a list of
// Drones and not a list of tasks.

import type { Turn } from "@armada/protocol";
import { droneViewsOf, type DroneView, type GroupView, type TaskView } from "../../draft";
import { answered, called, droneEnded, instructed, said } from "./base";

/** When each task's Drone was spawned and, once it stopped, when that was. */
const TIMES: Record<string, { since: string; ended?: string }> = {
  T1: { since: "2026-09-22T09:12:00Z", ended: "2026-09-22T09:41:00Z" },
  T2: { since: "2026-09-22T09:21:00Z", ended: "2026-09-22T09:38:00Z" },
  T3: { since: "2026-09-22T09:58:00Z", ended: "2026-09-22T10:06:00Z" },
  T4: { since: "2026-09-22T09:58:00Z", ended: "2026-09-22T10:08:00Z" },
  T5: { since: "2026-09-22T10:15:00Z", ended: "2026-09-22T10:34:00Z" },
  T6: { since: "2026-09-22T10:15:00Z", ended: "2026-09-22T10:31:00Z" },
  T7: { since: "2026-09-22T10:58:00Z" },
  T8: { since: "2026-09-22T11:10:00Z" },
};

/** T2's first Drone, which a person ended before it finished. */
const T2_FIRST = "01M2D5HKQP001DRONE000T2A";

/** An instant `seconds` after `from`, in the wire's spelling. */
function after(from: string, seconds: number): string {
  return new Date(Date.parse(from) + seconds * 1000).toISOString().replace(".000Z", "Z");
}

/**
 * What a Drone on `task` wrote: the brief, a read of every file in its scope,
 * the edit, the tests, and — where it finished — its closing line.
 */
function transcriptOf(task: TaskView, drone: Pick<DroneView, "state" | "since" | "turns" | "cost_micros">): Turn[] {
  const step = task.coord.step;
  const since = drone.since ?? TIMES[task.id]!.since;
  let at = 0;
  const next = (gap = 20) => after(since, (at += gap));
  const rows: Turn[] = [
    instructed(step, since, 2, task.expects ?? task.title, "Implement"),
    said(step, next(), `Starting on ${task.id}: ${task.title}. Reading the files it touches first.`),
  ];
  task.scope.forEach((path, n) => {
    const call = `${task.id}-read-${n}`;
    rows.push(called(step, next(), call, "Read", path));
    rows.push(answered(step, next(4), call));
  });
  if (drone.state === "killed") {
    rows.push(said(step, next(), "The panel should read the Job's resources route, so I'll add a poll beside it."));
    rows.push(called(step, next(), `${task.id}-edit-0`, "Edit", "crates/ipc/operations.toml"));
    return rows;
  }
  const target = task.scope[0] ?? "";
  rows.push(said(step, next(), `The change belongs in ${target}. Writing it now.`));
  rows.push(called(step, next(), `${task.id}-edit`, "Edit", target));
  rows.push(answered(step, next(6), `${task.id}-edit`));
  rows.push(called(step, next(), `${task.id}-test`, "Bash", "pnpm -C packages/screens exec vitest run"));
  if (drone.state === "running") return rows;
  const failed = drone.state === "failed";
  rows.push(answered(step, next(40), `${task.id}-test`, failed));
  if (!failed) rows.push(said(step, next(), task.shown ?? "Done. The tests pass."));
  rows.push(droneEnded(step, next(), drone.turns ?? 0, drone.cost_micros ?? 0));
  return rows;
}

/**
 * Every Drone the plan's tasks name, with times and a transcript, and T2's
 * ended first Drone where T2 has run.
 */
export function arcDrones(groups: readonly GroupView[]): DroneView[] {
  const tasks = new Map(groups.flatMap((group) => group.tasks).map((task) => [task.id, task]));
  const drones: DroneView[] = droneViewsOf(groups).map((drone) => {
    const task = tasks.get(drone.task)!;
    const time = TIMES[drone.task]!;
    const view: DroneView = { ...drone, since: time.since };
    if (drone.state !== "running" && time.ended !== undefined) view.ended_at = time.ended;
    view.transcript = transcriptOf(task, view);
    return view;
  });
  const t2 = tasks.get("T2");
  if (t2?.drone_id === undefined) return drones;
  const first: DroneView = {
    id: T2_FIRST,
    task: "T2",
    step: t2.coord.step,
    state: "killed",
    since: "2026-09-22T09:12:00Z",
    ended_at: "2026-09-22T09:20:00Z",
    turns: 7,
  };
  first.transcript = transcriptOf(t2, first);
  return [...drones, first];
}

/**
 * The Drone running T6 again after group three failed its Checks — the
 * process Pulse counts in `groupFailed`.
 */
export function t6Retry(groups: readonly GroupView[]): DroneView {
  const t6 = groups.flatMap((group) => group.tasks).find((task) => task.id === "T6")!;
  const retry: DroneView = {
    id: "01M2D5HKQP001DRONE000T6B",
    task: "T6",
    step: t6.coord.step,
    state: "running",
    since: "2026-09-22T10:44:20Z",
    turns: 3,
  };
  retry.transcript = transcriptOf(t6, retry);
  return retry;
}
