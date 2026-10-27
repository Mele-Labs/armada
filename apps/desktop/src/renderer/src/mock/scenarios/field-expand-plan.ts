// A Job mid-build with its plan in progress, for the walk `fieldExpandPlan`: five groups, one done, three under way, one not begun, and
// drones over the files they write. Over the `one-session-view` scenario, with its busy Session given the plan and the Field. Its Completed holds what
// the Job has made and a walk approved, and a second walk is shown and waiting in Now (`../audit-job.ts`).

import type { Scenario } from "../moment";
import { AUDIT_FILES, AUDIT_TAKES, BUSY, ROTATION, approved, auditLedger, waitingOn, walkActs } from "../audit-job";
import { s261OneSessionView as base } from "./one-session-view";

export const s274FieldExpandPlan: Scenario = {
  ...base,
  name: "field-expand-plan",
  says: "A Job mid-build: its plan in progress along the Field, what it has made in Completed, and a walk waiting in Now",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(BUSY, (one) => auditLedger(one, { steps: ["Draft", "Present", "Build", "Verify", "Pull request"], at: 2 }, [approved(ROTATION, "2026-10-26T13:58:00.000Z"), AUDIT_FILES], waitingOn(AUDIT_FILES)));
      return walkActs(store, BUSY, AUDIT_FILES, AUDIT_TAKES);
    },
  },
};
