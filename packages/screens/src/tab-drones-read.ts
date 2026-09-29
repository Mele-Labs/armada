// What the Drones destination reads: which Drones a filter holds, in which
// order, and one Drone's transcript as `DroneTurns` draws it.

import type { DroneTurn, JobDroneState, JobDronesFilter, Thought, TurnStep } from "@armada/components";
import type { ReactNode } from "react";
import type { JobDetail, Turn } from "@armada/protocol";

import type { DroneThought, DroneView } from "./draft/drone";
import { clock } from "./duration";
import { entriesOf, type LogRow } from "./story";

export type DronesFilter = "all" | JobDroneState;
export type DronesOrder = "running" | "task";

/** The state, spelled. The same words the Plan board gives a task. */
export const DRONE_SAYS: Record<JobDroneState, string> = {
  running: "Running",
  done: "Done",
  failed: "Failed",
  killed: "Killed",
};

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
        a.task.localeCompare(b.task, undefined, { numeric: true }) ||
        (a.since ?? "").localeCompare(b.since ?? ""),
    );
  if (order === "task") return byTask;
  return [
    ...byTask.filter((drone) => drone.state === "running"),
    ...byTask.filter((drone) => drone.state !== "running"),
  ];
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
 * A thinking row carries what `thoughts` holds for it, toward its run's total.
 */
export function droneTurnsOf(
  rows: readonly Turn[],
  brief: (lines: LogRow["payload"]) => ReactNode,
  thoughts?: DroneView["thoughts"],
): DroneTurn[] {
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
    const payload = thoughts?.get(row.seq);
    const before = estimated;
    estimated =
      saw.event === "unrecognised" && saw.kind === THINKING && payload?.of === "tokens" ? payload.estimated : 0;
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
      case "unrecognised": {
        const thought = thoughtOf(saw.kind, payload, before);
        turns.push({ ...base, ...(thought === undefined ? { subject: saw.kind } : { thought }), quiet: true });
        break;
      }
      default:
        turns.push(base);
    }
  }
  return turns;
}

/** The harness saying the model is thinking. Carries `estimated_tokens` on its own line. */
const THINKING = "system/thinking_tokens";

/**
 * A thinking row, with what the draft carries for it.
 *
 * **A row's count is what it added, not the running estimate.** The harness
 * sends a cumulative figure within one model call, so summed raw a run's total
 * would count each call's early tokens once per row; the deltas sum to it.
 * `before` is the estimate the previous row left, zero at a call's start — and
 * a figure lower than it is a new call begun without a row between.
 */
function thoughtOf(kind: string, payload: DroneThought | undefined, before: number): Thought | undefined {
  if (kind === THINKING) {
    if (payload?.of !== "tokens") return { of: "thinking" };
    return { of: "thinking", tokens: payload.estimated - (payload.estimated < before ? 0 : before) };
  }
  return undefined;
}
