// Every Drone a Job has used, one per task and the one on a step that works no
// task, with its whole transcript. Draft,
// for `crates/ipc/src/drones.rs`.
//
// Source of truth today: `DroneList` and `DroneDetail`, which serve **live
// Drones only** — an exited Drone is gone from both — and `DroneMoved` in the
// Job's history, which names every Drone the Job has had and nothing about
// what it did. A `Turn` carries no Drone id, so one Drone's rows cannot be
// pulled out of the Job's stream. What is added is the Drone's state after it
// stopped, its turns and cost, and its transcript keyed by its id.

import type { JobDetail, Turn } from "@armada/protocol";

import type { GroupView } from "./group";

/**
 * Where a Drone is. `failed` is a task's own agent that stopped without
 * finishing; `killed` is one a person ended. Two different events wanting two
 * different responses, the split `drone_killed` exists for.
 */
export type DroneState = "running" | "done" | "failed" | "killed";

export type DroneView = {
  /** The Drone's id, as `DroneSummary.drone_id`. */
  id: string;
  /**
   * The task it was put on, by id. **Absent on a Drone working the step
   * itself** — a plan step's, which writes the tasks rather than working one.
   */
  task?: string;
  /** The step it ran under. */
  step: string;
  state: DroneState;
  /** When it was spawned — `DroneMoved`'s `drone_spawned`. Absent where unknown. */
  since?: string;
  /** When it stopped. Absent while it runs. */
  ended_at?: string;
  /** Turns taken so far. */
  turns?: number;
  /**
   * What it cost, in millionths of a dollar. **Absent while it runs**, for
   * `TaskView.cost_micros`'s reason: cost reaches Armada on the session's
   * terminating line.
   */
  cost_micros?: number;
  /**
   * Every row this Drone wrote, in order. **Absent is a transcript Fleet does
   * not serve**, never an empty one: today's `observe_job` does not say which
   * Drone wrote a row.
   */
  transcript?: readonly Turn[];
  /**
   * What a thinking row in `transcript` carries beyond its kind, by the row's
   * `seq`. **A sidecar rather than a row type wrapping `Turn`**, so the
   * transcript stays the wire's own rows and this is the one field deleted
   * when Fleet serves what it holds. Absent, a thinking row reads in words and
   * nothing more.
   */
  thoughts?: ReadonlyMap<number, DroneThought>;
};

/**
 * One thinking row's payload, which the wire drops today.
 *
 * **Fleet owes the count** (the owner, 29 Sep 2026: "words and a token
 * count"): a `system/thinking_tokens` line's `estimated_tokens` is read by
 * `crates/adapters/src/watching.rs` for the proposer and dropped by the
 * transcript decoder, which emits only the kind.
 */
export type DroneThought =
  /** The harness's estimate, **cumulative within one model call**, as it is sent. */
  { of: "tokens"; estimated: number };

/**
 * Today's wire: the Drone on each task that names one, with no transcript.
 * A task's `failed` is its Drone's; nothing today can say a person ended one.
 *
 * **And the Job's own Drone, given the Job, where no running Drone already
 * stands on its step.** A Drone on a planning step works no task, so no task
 * names it, and Job 2's Drones tab was empty while one worked (owner, 1 Oct
 * 2026). `assigned_drone` is presence — a process is on the Job — so it reads
 * running, on the step the Job is on. **Past Drones are not here**: Fleet
 * serves live ones only.
 */
export function droneViewsOf(groups: readonly GroupView[], whole?: JobDetail): DroneView[] {
  const onTasks = taskDronesOf(groups);
  const id = whole?.job.assigned_drone;
  const step = whole?.job.current_step_id;
  if (id === undefined || step === undefined) return onTasks;
  if (onTasks.some((one) => one.id === id || (one.step === step && one.state === "running"))) return onTasks;
  const view: DroneView = { id, step, state: "running" };
  // **The step's live run began when its Drone was spawned for it**, so that
  // run's start is the Drone's — `GET /drones`' `since` differs by the spawn's
  // milliseconds. A run that has ended says nothing about this Drone.
  const run = whole?.steps.find((one) => one.step_id === step)?.attempts.at(-1);
  if (run !== undefined && run.ended_at === undefined) view.since = run.started_at;
  return [...onTasks, view];
}

function taskDronesOf(groups: readonly GroupView[]): DroneView[] {
  return groups.flatMap((group) => group.tasks).flatMap((task): DroneView[] => {
    if (task.drone_id === undefined) return [];
    const state: DroneState | undefined =
      task.state === "working"
        ? "running"
        : task.state === "done"
          ? "done"
          : task.state === "failed"
            ? "failed"
            : undefined;
    if (state === undefined) return [];
    const view: DroneView = { id: task.drone_id, task: task.id, step: task.coord.step, state };
    if (task.turns !== undefined) view.turns = task.turns;
    if (task.cost_micros !== undefined) view.cost_micros = task.cost_micros;
    return [view];
  });
}
