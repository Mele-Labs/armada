// A Trigger that holds the Job, on the mock Fleet: `rerun_trigger` and `skip_trigger` answered as
// `fleet::trigger_hold` answers them. A rerun runs the Command again, and in the mock it passes, so the
// hold is let go; a skip records the firing skipped by the owner. Both refuse where nothing holds the Job.
// **A Trigger on a destructive Command that waits on him is answered by the same two**: Run is the
// rerun, and it ends passed in the same breath, so no timer has to fire while another Job is open.

import type { HoldAct, JobDetail, JobTrigger, Outcome } from "@armada/protocol";
import { reran, skippedByOwner } from "@armada/jobs/fake";

import type { BridgeApi } from "../../../shared/api";
import type { Fleet } from "./fake-context";

const refusal = (code: string, message: string): Outcome => ({
  ok: false,
  why: "refused",
  error: { code, message, run_id: "mock", fields: {}, chain: [] },
});

/** The seconds a moved row is stamped with, after the log line the fixture wrote at 31. */
const SECOND = 44;

/** Whether a firing is the one `by` names. */
const named = (one: JobTrigger, by: HoldAct): boolean => by.trigger !== undefined && one.name === by.trigger;

/** The Job with its alert gone once no Trigger holds it. */
const settled = (whole: JobDetail, triggers: JobTrigger[]): JobDetail => {
  // A bell that asks stays while a Trigger still asks; the fixtures that predate asking carry such a row beside a hold.
  const asking = whole.job.alert?.kind === "asks" && triggers.some((one) => one.state === "awaiting_owner");
  if (asking || triggers.some((one) => one.state === "held" || one.state === "rerunning")) return { ...whole, triggers };
  const { alert: _alert, ...job } = whole.job;
  return { ...whole, job, triggers };
};

export function holdServed(fleet: Fleet): Pick<BridgeApi, "rerunTrigger" | "skipTrigger"> {
  const later = (ms: number, then: () => void) => void window.setTimeout(then, ms);
  const move = (jobId: string, by: HoldAct, moved: (one: JobTrigger) => JobTrigger) =>
    fleet.stepping(jobId, (whole) => settled(whole, (whole.triggers ?? []).map((one) => (named(one, by) ? moved(one) : one))));

  /** The held firing `by` names on the open Job, or the refusal Fleet gives where there is none. */
  const holding = (jobId: string, by: HoldAct): JobTrigger | Outcome => {
    if ((by.trigger === undefined) === (by.addition === undefined)) {
      return refusal("fleet.no_hold_named", "name the Trigger or the added step, one of the two");
    }
    const watched = fleet.state().watched;
    const one =
      watched.state === "read" && watched.jobId === jobId
        ? (watched.detail.triggers ?? []).find(
            (row) => named(row, by) && (row.state === "held" || row.state === "awaiting_owner" || (row.blocks === true && ["repairing", "rerunning", "fix_ready"].includes(row.state))),
          )
        : undefined;
    return one === undefined ? refusal("fleet.no_hold", "nothing by that name holds this Job") : one;
  };

  return {
    rerunTrigger: async (jobId, by): Promise<Outcome> => {
      const one = holding(jobId, by);
      if ("ok" in one) return one;
      if (one.state === "repairing" || one.state === "rerunning") {
        return refusal("fleet.hold_repairing", `\`${one.name}\` is being repaired, which lets the hold go by itself`);
      }
      if (one.state === "fix_ready") {
        return refusal("fleet.hold_has_a_fix", `a fix for \`${one.name}\` waits on a choice, which settles it`);
      }
      // Run on a destructive Command: it runs once and ends passed, with nothing to wait for.
      if (one.state === "awaiting_owner") {
        move(jobId, by, (row) => reran(row, SECOND));
        return { ok: true };
      }
      move(jobId, by, (row) => ({ ...row, state: "rerunning" }));
      later(2000, () => move(jobId, by, (row) => reran(row, SECOND)));
      return { ok: true };
    },
    skipTrigger: async (jobId, by): Promise<Outcome> => {
      const one = holding(jobId, by);
      if ("ok" in one) return one;
      if (one.state === "repairing" || one.state === "rerunning") {
        return refusal("fleet.hold_repairing", `\`${one.name}\` is being repaired, which lets the hold go by itself`);
      }
      move(jobId, by, skippedByOwner);
      return { ok: true };
    },
  };
}
