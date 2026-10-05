// A Drone held inside a call, waiting on a person, drawn on the Drone itself.
//
// **The wire says which step, not which Drone.** `CommandInFlight.step_id` is
// all it carries, so a Drone owns the ask only where it is the one Drone at
// work on that step. A step running several at once (a Drone per task) cannot
// say whose it is, and no card claims it: Overview's box still does, and the
// step's own panel draws it for the step.

import type { ReactNode } from "react";

import type { JobDetail as JobWhole } from "@armada/protocol";

import { isWorking, type DroneView } from "./draft/drone";
import { answeringOf, commandOf, type ExplainOne } from "./step";
import type { ActingAct } from "./pending";
import type { CommandAnswer } from "@armada/protocol";

/** What the mark says of a Drone waiting on a person: the Board's own word. */
export const NEEDS_YOU = "Needs you";

/** The command a Job's Drone is held on, drawn once and placed by the surface. */
export type HeldCommand = {
  /** The step Fleet says the ask is on. */
  stepId: string;
  /** The one Drone at work on that step. Absent where the wire cannot say which. */
  droneId?: string;
  /** Overview's own box, with the command named over the answers. */
  node: ReactNode;
};

/** The one Drone at work on the held step, or none where there are several. */
export function holderOf(whole: JobWhole | null, drones: readonly DroneView[]): DroneView | undefined {
  const waiting = whole?.command_waiting;
  if (waiting === undefined) return undefined;
  const at = drones.filter((one) => one.step === waiting.step_id && isWorking(one));
  return at.length === 1 ? at[0] : undefined;
}

export function heldCommandOf(
  whole: JobWhole | null,
  drones: readonly DroneView[],
  jobId: string,
  stale: boolean,
  acting: boolean,
  onAnswerCommand: (jobId: string, call: string, answer: CommandAnswer, note?: string, rule?: string) => void,
  actingAct: ActingAct | undefined,
  explain: ExplainOne | undefined,
): HeldCommand | undefined {
  const waiting = whole?.command_waiting;
  if (waiting === undefined) return undefined;
  const answering = answeringOf(jobId, stale, acting, onAnswerCommand, actingAct);
  const holder = holderOf(whole, drones);
  return {
    stepId: waiting.step_id,
    ...(holder === undefined ? {} : { droneId: holder.id }),
    node: commandOf(whole, answering, explain, true),
  };
}
