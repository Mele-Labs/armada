// What the Drones destination reads: which Drones a filter holds, in which
// order, and one Drone's transcript as `DroneTurns` draws it.

import type { DroneTurn, JobDroneState, JobDronesFilter, TurnStep } from "@armada/components";
import type { ReactNode } from "react";
import type { JobDetail, Turn } from "@armada/protocol";

import type { DroneView } from "./draft/drone";
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
 */
export function droneTurnsOf(
  rows: readonly Turn[],
  detail: JobDetail | null,
  brief: (lines: LogRow["payload"]) => ReactNode,
): DroneTurn[] {
  const answers = new Map<string, boolean>();
  for (const row of rows) {
    if (row.saw.event === "answered") answers.set(row.saw.call, row.saw.failed);
  }
  const turns: DroneTurn[] = [];
  for (const row of rows) {
    const saw = row.saw;
    const base = {
      id: String(row.seq),
      at: clock(row.ts),
      kind: saw.event,
      who: row.by,
      ...(row.step === undefined ? {} : { step: stepOf(detail, row.step) }),
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
        // The speaker column names who, so the refusal says itself here.
        turns.push({ ...base, subject: saw.tool, said: `Refused: ${saw.because}` });
        break;
      case "ended":
        turns.push({
          ...base,
          subject: `${saw.turns} turns · $${(saw.cost_micros / 1_000_000).toFixed(2)}`,
        });
        break;
      case "unrecognised":
        turns.push({ ...base, subject: saw.kind, quiet: true });
        break;
      default:
        turns.push(base);
    }
  }
  return turns;
}
