// Every Drone a Job has had, each with its own rows. Read off
// `list_job_drones` (`crates/ipc/src/drones.rs`) and the Job's observed turns,
// which Fleet stamps with the Drone whose transcript each is in (protocol 21.4).
//
// `task` is on the wire since 23.1, on a step that works its tasks a Drone each
// (spike 022, slice 1b); a Drone that worked its whole step names none.

import type { DroneState, JobDetail, JobDrones, Turn } from "@armada/protocol";

import type { GroupView } from "./group";

export type { DroneState };

export type DroneView = {
  /** The Drone's id — `drone_id` on the wire. */
  id: string;
  /**
   * The task it was put on, by id — `JobDrone.task`. **Absent on a Drone that
   * worked its whole step.**
   */
  task?: string;
  /** The step it ran under. */
  step: string;
  state: DroneState;
  /** The model it was spawned as — `JobDrone.model`, since 23.6. Absent where Fleet kept none. */
  model?: string;
  /** When it was spawned. Absent only on the Job's own Drone before the list has it. */
  since?: string;
  /** When it stopped. Absent while it runs. */
  ended_at?: string;
  /**
   * When its last run ended, on a `running` Drone Fleet still holds — at the
   * gate while its Checks run, or for a person. **Absent while it works.**
   * `JobDrone.at_rest_since`, since 23.11.
   */
  at_rest_since?: string;
  /** Turns taken so far. **Absent where none has been counted**, never nought. */
  turns?: number;
  /**
   * What it cost, in millionths of a dollar. A running Drone carries one as of
   * its last finished invocation. **Absent is no price named**, never nought.
   */
  cost_micros?: number;
  /**
   * Every row this Drone's transcript holds, in order — the Job's observed
   * turns whose `drone_id` is this one's. **Absent is the Job's turns not in
   * hand**, never an empty transcript.
   */
  transcript?: readonly Turn[];
};

/**
 * Every Drone Fleet lists for the Job, in the order they were spawned, each
 * with its rows out of `turns`. A row with no `drone_id` is no Drone's — a
 * Helm thread's, or one from a Fleet before 21.4.
 *
 * **And the Job's own Drone, given the Job, where the list does not hold it
 * and no running Drone already stands on its step.** `assigned_drone` is
 * presence, so it reads running on the step the Job is on — what the tab drew
 * while the list was unread, or was a step behind the event that spawned it
 * (owner, 1 Oct 2026: Job 2's tab was empty while its plan step worked).
 */
/**
 * A Drone at work: `running`, and not at rest. **`running` alone names both**
 * — a Drone resting at the gate while its Checks run is still held — which is
 * what the owner read as a Drone still working on 3 Oct 2026.
 */
export function isWorking(drone: Pick<DroneView, "state" | "at_rest_since">): boolean {
  return drone.state === "running" && drone.at_rest_since === undefined;
}

export function droneViewsOf(listed: JobDrones | undefined, whole?: JobDetail, turns?: readonly Turn[]): DroneView[] {
  const rowsOf = (id: string) => (turns === undefined ? {} : { transcript: turns.filter((row) => row.drone_id === id) });
  const views = (listed?.drones ?? []).map((one): DroneView => ({
    id: one.drone_id,
    ...(one.task === undefined ? {} : { task: one.task }),
    step: one.step_id,
    state: one.state,
    since: one.since,
    ...(one.model === undefined ? {} : { model: one.model }),
    ...(one.ended_at === undefined ? {} : { ended_at: one.ended_at }),
    ...(one.at_rest_since === undefined ? {} : { at_rest_since: one.at_rest_since }),
    ...(one.turns === undefined ? {} : { turns: one.turns }),
    ...(one.cost_micros === undefined ? {} : { cost_micros: one.cost_micros }),
    ...rowsOf(one.drone_id),
  }));
  const id = whole?.job.assigned_drone;
  const step = whole?.job.current_step_id;
  if (id === undefined || step === undefined) return views;
  if (views.some((one) => one.id === id || (one.step === step && one.state === "running"))) return views;
  const view: DroneView = { id, step, state: "running", ...rowsOf(id) };
  // **The step's live run began when its Drone was spawned for it**, so that
  // run's start is the Drone's — `GET /drones`' `since` differs by the spawn's
  // milliseconds. A run that has ended says nothing about this Drone.
  const run = whole?.steps.find((one) => one.step_id === step)?.attempts.at(-1);
  if (run !== undefined && run.ended_at === undefined) view.since = run.started_at;
  return [...views, view];
}

/**
 * The Drone on each task that names one, with no transcript — the draft's
 * per-task Drones, and Plan's peek at a task's own. A task's `failed` is its
 * Drone's; nothing here can say a person ended one.
 */
export function taskDronesOf(groups: readonly GroupView[]): DroneView[] {
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
