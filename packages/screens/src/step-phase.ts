// Which part of a running step it is in, read off what the wire already
// carries: the Job's open question and command, the gate's live Checks
// (`StepDetail.checking`, which the step panel draws a running mark per Check
// from), the Judge's call out (`StepDetail.judging`), and the Drones on the
// step. The owner's annotation of 3 Oct 2026, `ouqa`.

import type { StepActivity, StepPhase } from "@armada/components";
import type { JobDetail as JobWhole, StepDetail } from "@armada/protocol";

import type { DroneView } from "./draft/drone";
import { checksOf, isRunning, isWaiting } from "./gates";

/** The phase, and the words its mark is named by. */
export type StepPhaseReading = { phase: StepPhase; label: string };

/**
 * The phase of `step`, or `undefined` where it is not running or nothing says.
 *
 * **In this order, most pressing first.** A person's turn leads, because it is
 * what holds the rest. **A Check in flight wins over a Drone that reads
 * running**: Fleet reports a Drone that has handed in as still running while
 * its gate works (being fixed in Fleet), and the gate's own live set is the
 * fact. The Judge comes after the Checks, as the gate asks it.
 */
export function phaseOf(
  whole: JobWhole,
  step: StepDetail,
  activity: StepActivity,
  drones: readonly DroneView[],
): StepPhaseReading | undefined {
  if (activity !== "running") return undefined;
  const here = step.step_id === whole.job.current_step_id;
  if (
    (here && (whole.asking !== undefined || whole.command_waiting !== undefined)) ||
    whole.judge_question?.step_id === step.step_id
  ) {
    return { phase: "waiting", label: "Waiting on you" };
  }
  const checks = checksOf(step);
  if (checks.some(isRunning)) return { phase: "checks", label: "Checks running" };
  if (checks.some(isWaiting)) return { phase: "checks", label: "Checks waiting" };
  if (step.judging !== undefined) return { phase: "judge", label: "Judge reading" };
  const working = drones.filter((one) => one.step === step.step_id && one.state === "running");
  if (working.length === 0) return undefined;
  const tasks = working.flatMap((one) => (one.task === undefined ? [] : [one.task]));
  const many = working.length > 1;
  return {
    phase: "drone",
    label:
      tasks.length === 0
        ? many
          ? `${working.length} Drones working`
          : "Drone working"
        : `${many ? "Drones" : "Drone"} on ${tasks.join(", ")}`,
  };
}
