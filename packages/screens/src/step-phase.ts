// The phase track of a running step: its parts in order — its Drones, its
// gate's Checks, its Judge — each done, now or next, and a person where one
// holds it. Read off what the wire already carries: the Job's open question
// and command, the gate's live Checks (`StepDetail.checking`, which the step
// panel draws a running mark per Check from), the Judge's call out
// (`StepDetail.judging`), and the Drones on the step, less any at rest
// (`at_rest_since`, 23.11).
//
// The owner's annotation of 3 Oct 2026, `ouqa` ("I had no idea it was running
// checks"), and his walk of the first answer, a mark on the state pill: "too
// easy to miss". He chose this track.

import type { StepActivity, StepPhase, StepPhasePart } from "@armada/components";
import type { JobDetail as JobWhole, StepDetail } from "@armada/protocol";

import { isWorking, type DroneView } from "./draft/drone";
import { checksOf, isRunning, isWaiting } from "./gates";

export type TrackPart = StepPhasePart;

/** Each part's name, drawn beside its glyph. */
const NAME: Record<StepPhase, string> = { drone: "Drones", checks: "Checks", judge: "Judge", waiting: "You" };

/**
 * The track of `step`, or `undefined` on a step neither running nor waiting on
 * a person.
 *
 * **Only the parts the step declares**: Drones always, Checks where it declares
 * any, a Judge where it declares one. A person's part is drawn only while one
 * holds the step — after the Drones for a Drone's question or command, at the
 * end for a person's gate or a Judge's question.
 *
 * **A Check in flight is the part now, over a Drone that reads running.** A
 * Drone resting at the gate is held and not at work, which `at_rest_since`
 * says; a Fleet before 23.11 says neither, and the gate's own live set is the
 * fact either way. The Judge comes after the Checks, as the gate asks it.
 */
export function trackOf(
  whole: JobWhole,
  step: StepDetail,
  activity: StepActivity,
  drones: readonly DroneView[],
): TrackPart[] | undefined {
  if (activity !== "running" && activity !== "awaiting_human") return undefined;
  const parts: StepPhase[] = [
    "drone",
    ...((step.checks ?? []).length > 0 ? (["checks"] as const) : []),
    ...((step.judge_checks ?? []).length > 0 ? (["judge"] as const) : []),
  ];
  const here = step.step_id === whole.job.current_step_id;
  const onStep = drones.filter((one) => one.step === step.step_id);
  const working = onStep.filter(isWorking);
  const checks = checksOf(step);

  // Where a person holds it, the part is theirs, placed after what it interrupts.
  const gateHolds = activity === "awaiting_human" || whole.judge_question?.step_id === step.step_id;
  const droneAsks = here && (whole.asking !== undefined || whole.command_waiting !== undefined);
  if (gateHolds || droneAsks) {
    const at = gateHolds ? parts.length : 1;
    const order: StepPhase[] = [...parts.slice(0, at), "waiting", ...parts.slice(at)];
    return order.map((phase, index) => part(phase, index < at ? "done" : index === at ? "now" : "next", "Waiting on you"));
  }

  const nowIs: { phase: StepPhase; label: string } | undefined = checks.some(isRunning)
    ? { phase: "checks", label: "Checks running" }
    : checks.some(isWaiting)
      ? { phase: "checks", label: "Checks waiting" }
      : step.judging !== undefined
        ? { phase: "judge", label: "Judge reading" }
        : working.length > 0
          ? { phase: "drone", label: dronesSaid(working) }
          : undefined;
  const at = nowIs === undefined ? -1 : parts.indexOf(nowIs.phase);
  // Nothing live: the Drones are done where one is resting, and the rest is to come.
  const doneTo = at !== -1 ? at : onStep.some((one) => one.at_rest_since !== undefined) ? 1 : 0;
  return parts.map((phase, index) =>
    index === at ? part(phase, "now", nowIs!.label) : part(phase, index < doneTo ? "done" : "next"),
  );
}

/** One part, named on hover by what it is doing now, or by its name and where it stands. */
function part(phase: StepPhase, state: TrackPart["state"], nowSaid?: string): TrackPart {
  const name = NAME[phase];
  const label = state === "now" ? (nowSaid ?? name) : state === "done" ? `${name} done` : `${name} next`;
  return { phase, name, label, state };
}

/** The Drones at work: the task each is on, or how many where none names one. */
function dronesSaid(working: readonly DroneView[]): string {
  const tasks = working.flatMap((one) => (one.task === undefined ? [] : [one.task]));
  const many = working.length > 1;
  if (tasks.length === 0) return many ? `${working.length} Drones working` : "Drone working";
  return `${many ? "Drones" : "Drone"} on ${tasks.join(", ")}`;
}
