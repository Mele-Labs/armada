// A failed Trigger's repair on the mock Fleet: the rows `JobDetail.triggers` carries, moved the way
// Fleet moves them (`fleet::repairing`, `fleet::placing_a_fix`) and `choose_trigger_fix` refused where
// Fleet refuses it. **Looking starts it**, as a repair starts under whoever is watching the Job: when
// the open Job holds a Trigger in `repairing`, its plan runs. A Drone that finds a fix holds it for the
// owner; one that finds none tries twice and the Trigger fails for good. **A Skill Trigger and a Skill
// or Drone step run the same way** (`fleet::side_run`): `running` with a Drone on it, then the fix
// held, then placed with nothing to run again.

import type { AddedStep, JobTrigger, Outcome, TriggerFixChoice } from "@armada/protocol";
import {
  chosenOntoTheBranch,
  fixHeld,
  noFix,
  placed,
  repairFailsId,
  REPAIRED,
  secondTry,
  job2Repairing,
  sideFixHeld,
  sidePlaced,
  sideStepFixHeld,
  sideStepPlaced,
} from "@armada/jobs/fake";

import type { BridgeApi } from "../../../shared/api";
import type { Fleet } from "./fake-context";

/** Whether the Drone finds a fix, by the Job it is on. */
const PLANS = (): Readonly<Record<string, "finds_a_fix" | "finds_none">> => ({
  [job2Repairing().job.id]: "finds_a_fix",
  [repairFailsId()]: "finds_none",
});

const refusal = (code: string, message: string): Outcome => ({
  ok: false,
  why: "refused",
  error: { code, message, run_id: "mock", fields: {}, chain: [] },
});

/** The seconds a moved row is stamped with, after the log line the fixture wrote at 20. */
const SECOND = { none: 52, placed: 44 };

export function repairServed(fleet: Fleet): Pick<BridgeApi, "chooseTriggerFix"> {
  const begun = new Set<string>();
  const move = (jobId: string, moved: (one: JobTrigger) => JobTrigger, name: string = REPAIRED) =>
    fleet.stepping(jobId, (whole) => ({
      ...whole,
      triggers: (whole.triggers ?? []).map((one) => (one.name === name ? moved(one) : one)),
    }));
  const later = (ms: number, then: () => void) => void window.setTimeout(then, ms);
  const moveStep = (jobId: string, id: string, moved: (one: AddedStep) => AddedStep) =>
    fleet.stepping(jobId, (whole) => ({ ...whole, additions: (whole.additions ?? []).map((one) => (one.id === id ? moved(one) : one)) }));

  fleet.listen((state) => {
    const watched = state.watched;
    if (watched.state !== "read") return;
    const jobId = watched.jobId;
    // A Drone on a Skill or a step added to the Job: its answer is a fix in a few seconds.
    for (const one of watched.detail.additions ?? []) {
      const key = `${jobId}|${one.id}`;
      if (one.state === "running" && one.repair_record !== undefined && !begun.has(key)) {
        begun.add(key);
        later(5000, () => moveStep(jobId, one.id, sideStepFixHeld));
      }
    }
    for (const one of watched.detail.triggers ?? []) {
      const key = `${jobId}|${one.name}`;
      if (one.state === "running" && one.repair !== undefined && !begun.has(key)) {
        begun.add(key);
        later(3500, () => fleet.stepping(jobId, (whole) => ({ ...whole, triggers: (whole.triggers ?? []).map((row) => (row.name === one.name ? sideFixHeld(row) : row)) })));
      }
    }
    const plan = PLANS()[jobId];
    const repairing = (watched.detail.triggers ?? []).some((one) => one.name === REPAIRED && one.state === "repairing");
    if (plan === undefined || !repairing || begun.has(jobId)) return;
    begun.add(jobId);
    if (plan === "finds_a_fix") {
      later(3500, () => move(jobId, fixHeld));
    } else {
      later(2000, () => move(jobId, secondTry));
      later(4000, () => move(jobId, (one) => noFix(one, SECOND.none)));
    }
  });

  return {
    chooseTriggerFix: async (jobId, body): Promise<Outcome> => {
      const watched = fleet.state().watched;
      const choice: TriggerFixChoice = body.choice;
      // A step added to the Job: no Command to run again, so the fix merged or opened is the end of it.
      if (body.addition !== undefined) {
        const step = watched.state === "read" && watched.jobId === jobId ? (watched.detail.additions ?? []).find((one) => one.id === body.addition) : undefined;
        if (step === undefined || step.state !== "fix_ready" || step.repair_record?.choice !== undefined) {
          return refusal("fleet.no_fix_waiting", `no fix for \`${body.addition}\` is waiting on a choice`);
        }
        moveStep(jobId, step.id, (one) => sideStepPlaced(one, choice, SECOND.placed));
        return { ok: true };
      }
      const held =
        watched.state === "read" && watched.jobId === jobId
          ? (watched.detail.triggers ?? []).find((one) => one.name === body.trigger && one.state === "fix_ready" && one.repair?.choice === undefined)
          : undefined;
      if (held === undefined) {
        return refusal("fleet.no_fix_waiting", `no fix for \`${body.trigger}\` is waiting on a choice`);
      }
      // A Skill's fix has no Command to run again either.
      if (held.drone === true) {
        move(jobId, (one) => sidePlaced(one, choice, SECOND.placed), held.name);
        return { ok: true };
      }
      // `new_pr` passes at once, the Command having passed on the repair branch; `this_branch` merges
      // and runs the Command again on the Job's.
      if (choice === "new_pr") {
        move(jobId, (one) => placed(one, choice, SECOND.placed));
      } else {
        move(jobId, chosenOntoTheBranch);
        later(2500, () => move(jobId, (one) => placed(one, choice, SECOND.placed)));
      }
      return { ok: true };
    },
  };
}
