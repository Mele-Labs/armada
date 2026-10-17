// Triggers' members: the mock Fleet's four routes, answered from `triggers-fleet.ts`. A held Trigger,
// a repair and the steps added to one Job belong to a Job's detail, which Bridge no longer draws, so
// the routes that serve them answer as a Job the mock does not hold.

import type { AddedStepsApi } from "../../../../shared/api/added-steps";
import type { TriggersApi, TriggersState } from "../../../../shared/api/triggers";
import { TRIGGERS_NOTHING_YET } from "../../../../shared/api/triggers";
import type { Fleet, Slice } from "../fake-context";
import { triggersServed } from "../triggers-fleet";

const none = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

function served(fleet: Fleet): TriggersApi & AddedStepsApi {
  const triggers = triggersServed();
  return {
    ...triggers,
    // Read off the Jobs the Fleet holds, as Fleet derives it: a Job with an alert is listed, `blocked` where it is
    // stopped mid-flight.
    readAlerts: async () => {
      const rows = fleet.state().jobs.filter((job) => job.alert !== undefined);
      const alert = (job: (typeof rows)[number]) => ({ job_id: job.id, handle: job.handle, status: job.status });
      return {
        ok: true,
        blocked: rows.filter((job) => job.status === "escalated").map(alert),
        waiting: rows.filter((job) => job.status !== "escalated").map(alert),
      };
    },
    chooseTriggerFix: async () => none.outcome,
    rerunTrigger: async () => none.outcome,
    skipTrigger: async () => none.outcome,
    addJobStep: async () => none,
    editJobStep: async () => none,
    removeJobStep: async () => none,
    readRepairDiff: async () => none,
  };
}

export const triggers: Slice<TriggersApi & AddedStepsApi, TriggersState> = {
  name: "triggers",
  state: TRIGGERS_NOTHING_YET,
  api: (_scenario, fleet) => served(fleet),
};
