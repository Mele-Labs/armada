// A failed Trigger's repair on the mock Fleet: the rows `JobDetail.triggers` carries, moved the way
// Fleet moves them (`fleet::repairing`, `fleet::placing_a_fix`) and `choose_trigger_fix` refused where
// Fleet refuses it. **Looking starts it**, as a repair starts under whoever is watching the Job: when
// the open Job holds a Trigger in `repairing`, its plan runs. A Drone that finds a fix holds it for the
// owner; one that finds none tries twice and the Trigger fails for good.

import type { JobTrigger, Outcome, TriggerFixChoice } from "@armada/protocol";
import { chosenOntoTheBranch, fixHeld, noFix, placed, repairFailsId, REPAIRED, secondTry, job2Repairing } from "@armada/jobs/fake";

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
  const move = (jobId: string, moved: (one: JobTrigger) => JobTrigger) =>
    fleet.stepping(jobId, (whole) => ({
      ...whole,
      triggers: (whole.triggers ?? []).map((one) => (one.name === REPAIRED ? moved(one) : one)),
    }));
  const later = (ms: number, then: () => void) => void window.setTimeout(then, ms);

  fleet.listen((state) => {
    const watched = state.watched;
    if (watched.state !== "read") return;
    const jobId = watched.jobId;
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
      const held =
        watched.state === "read" && watched.jobId === jobId
          ? (watched.detail.triggers ?? []).find((one) => one.name === body.trigger && one.state === "fix_ready" && one.repair?.choice === undefined)
          : undefined;
      if (held === undefined) {
        return refusal("fleet.no_fix_waiting", `no fix for \`${body.trigger}\` is waiting on a choice`);
      }
      const choice: TriggerFixChoice = body.choice;
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
