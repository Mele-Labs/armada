// What the Drones destination reads: which Drones a filter holds, in which
// order, and one Drone's transcript as `DroneTurns` draws it.

import type { DroneTurn, JobDroneState, JobDronesFilter, Thought, TurnStep } from "@armada/components";
import type { ReactNode } from "react";
import type { JobDetail, Turn } from "@armada/protocol";

import { isWorking, type DroneView } from "./draft/drone";
import { clock, elapsedSince } from "./duration";
import { entriesOf, THINKING_TOKENS, type LogRow } from "./story";

export type DronesFilter = "all" | JobDroneState;
export type DronesOrder = "running" | "task";

/** The state, spelled. The same words the Plan board gives a task. */
export const DRONE_SAYS: Record<JobDroneState, string> = {
  running: "Running",
  done: "Done",
  failed: "Failed",
  killed: "Killed",
};

/** What a `running` Drone Fleet holds at rest says — not working (23.11). */
const AT_REST = "At rest";

/** A Drone's state in words: `At rest` for a running one between runs, else its state's. */
export function droneSays(drone: Pick<DroneView, "state" | "at_rest_since">): string {
  return drone.state === "running" && !isWorking(drone) ? AT_REST : DRONE_SAYS[drone.state];
}

/**
 * What a task's Drone says in Plan's peek, which reads the draft's per-task
 * Drones: nothing on the wire joins a task to a Drone's rows.
 */
export const TRANSCRIPT_UNSERVED = "Fleet does not serve one Drone's transcript yet.";
/** What one Drone's transcript says with none of its rows in the Job's. */
export const TRANSCRIPT_EMPTY = "This Drone has written nothing yet.";

/**
 * How long a Drone has run, on the header's own `Run time` terms: to now while
 * it runs, to when it stopped once it has. A stopped Drone with no end draws
 * nothing, as a Job does.
 */
export function ranForOf(drone: DroneView, now: number): string | undefined {
  return drone.ended_at !== undefined
    ? elapsedSince(drone.since, drone.ended_at)
    : drone.state === "running"
      ? elapsedSince(drone.since, now)
      : undefined;
}

/**
 * The Drone a task has now: its latest, where one task has had two — the first
 * ended by hand and the second finishing it. Undefined for a task none has run.
 */
export function droneOnTask(drones: readonly DroneView[], taskId: string): DroneView | undefined {
  return drones
    .filter((drone) => drone.task === taskId)
    .sort((a, b) => (a.since ?? "").localeCompare(b.since ?? ""))
    .at(-1);
}

export const ORDER_SAYS: Record<DronesOrder, string> = {
  running: "Running first",
  task: "Task order",
};

/** All first, then each state a Drone here is in. A state no Drone is in is not offered. */
export function dronesFiltersOf(drones: readonly DroneView[]): JobDronesFilter[] {
  const states = (["running", "done", "failed", "killed"] as const).filter((state) =>
    drones.some((drone) => drone.state === state),
  );
  return [
    { id: "all", label: "All", count: drones.length },
    ...states.map((state) => ({
      id: state,
      label: DRONE_SAYS[state],
      count: drones.filter((drone) => drone.state === state).length,
    })),
  ];
}

/**
 * The Drones a filter holds, in the order chosen. Task order is the plan's,
 * and two Drones on one task in the order they started. Running first keeps
 * that order under the running ones.
 */
export function dronesUnder(
  drones: readonly DroneView[],
  filter: DronesFilter,
  order: DronesOrder,
): DroneView[] {
  const byTask = drones
    .filter((drone) => filter === "all" || drone.state === filter)
    .sort(
      (a, b) =>
        (a.task ?? "").localeCompare(b.task ?? "", undefined, { numeric: true }) ||
        (a.since ?? "").localeCompare(b.since ?? ""),
    );
  if (order === "task") return byTask;
  // Working first: a Drone at rest is held, and not at work.
  return [...byTask.filter(isWorking), ...byTask.filter((drone) => !isWorking(drone))];
}

/**
 * What a Drone is called. A task's is named for its task; the one on the Job
 * now is the Job's own, as Plan names it; any other worked its step, and is
 * named for that — a list off the record holds Drones long gone, and calling
 * each the Job's would name three things one.
 */
export function droneLabelOf(drone: DroneView, whole: JobDetail | null): string {
  if (drone.task !== undefined) return `Drone on ${drone.task}`;
  if (drone.id === whole?.job.assigned_drone) return "This Job's Drone";
  return `Drone on ${stepOf(whole, drone.step).label}`;
}

/** A step's name, the way the rail names it. */
export function stepOf(detail: JobDetail | null, stepId: string): TurnStep {
  const label = detail?.steps.find((step) => step.step_id === stepId)?.label;
  return label === undefined
    ? { id: stepId, label: stepId, labelIsAnIdentifier: true }
    : { id: stepId, label };
}

/**
 * One Drone's rows as `DroneTurns` draws them: a call and its answer on one
 * row, the answer that said only that it answered folded into its call. What
 * Armada told it is drawn by `brief`, from the lines Overview's brief reads.
 * A thinking row carries what it added toward its run's total.
 */
export function droneTurnsOf(rows: readonly Turn[], brief: (lines: LogRow["payload"]) => ReactNode): DroneTurn[] {
  const answers = new Map<string, boolean>();
  for (const row of rows) {
    if (row.saw.event === "answered") answers.set(row.saw.call, row.saw.failed);
  }
  const turns: DroneTurn[] = [];
  // The estimate the last row left. Only a thinking row carries one forward:
  // any other row ends the model call its estimates were counting.
  let estimated = 0;
  for (const row of rows) {
    const saw = row.saw;
    const before = estimated;
    estimated = saw.event === "thinking" ? saw.estimated_tokens : 0;
    const base = {
      id: String(row.seq),
      at: clock(row.ts),
      kind: saw.event,
      who: row.by,
    };
    switch (saw.event) {
      case "answered":
        continue;
      case "started":
        turns.push({ ...base, subject: `${saw.session} · ${saw.model} · ${saw.mcp_servers} mcp servers` });
        break;
      case "called": {
        const failed = answers.get(saw.call);
        turns.push({
          ...base,
          subject: saw.tool,
          ...(saw.detail === "" ? {} : { detail: saw.detail }),
          truncated: saw.truncated,
          ...(failed === true ? { answer: "Failed." } : {}),
        });
        break;
      }
      case "said":
        turns.push({ ...base, said: saw.text });
        break;
      case "instructed":
        turns.push({ ...base, said: brief(entriesOf([row], undefined)[0]?.payload ?? []) });
        break;
      case "refused":
        // The card names who, so the refusal says itself here.
        turns.push({ ...base, subject: saw.tool, said: `Refused: ${saw.because}` });
        break;
      case "ended":
        turns.push({
          ...base,
          subject: `${saw.turns} turns · $${(saw.cost_micros / 1_000_000).toFixed(2)}`,
        });
        break;
      case "thinking":
        turns.push({ ...base, thought: thoughtOf(saw.estimated_tokens, before), quiet: true });
        break;
      case "unrecognised":
        // A thinking line that carried no figure arrives under its old kind:
        // still thinking, and adding nothing to the run's total.
        turns.push({
          ...base,
          ...(saw.kind === THINKING_TOKENS ? { thought: { of: "thinking" as const } } : { subject: saw.kind }),
          quiet: true,
        });
        break;
      default:
        turns.push(base);
    }
  }
  return turns;
}

/**
 * A thinking row, with what it added.
 *
 * **A row's count is what it added, not the running estimate.** The harness
 * sends a cumulative figure within one model call, so summed raw a run's total
 * would count each call's early tokens once per row; the deltas sum to it.
 * `before` is the estimate the previous row left, zero at a call's start — and
 * a figure lower than it is a new call begun without a row between.
 */
function thoughtOf(estimated: number, before: number): Thought {
  return { of: "thinking", tokens: estimated - (estimated < before ? 0 : before) };
}
