// A walk is an output of the Job, as a pull request is, and this plays its whole life on the Job's story: pending in Next under the workflow's Present step; shown, it
// moves to Now as a call on him, a lamp that pulses, its name and a tick for each take; pressing it, or Enter on it, opens the walk window as Bridge opens one; and on
// his Approve it moves to Completed under Present as an approved walk. The walk's `later` step shows the walk. Over the same Job as `field-expand-plan`, at Build.

import type { Session } from "@armada/screens/src/draft/sessions";

import { AUDIT_TAKES, BUSY, auditLedger, shown, waitingOn, walkActs, walkPage } from "../audit-job";
import type { Scenario } from "../moment";
import { s261OneSessionView as base } from "./one-session-view";

const FLOW = ["Draft", "Build", "Present", "Verify", "Pull request"];
const WALK = shown("b-4", "14:01:00", "Audit files in System", walkPage("job-walk-on-the-spine", "audit-files"), 3);

/** The workflow at a step. */
const at = (step: number) => (one: Session): Session => (one.work?.workflow === undefined ? one : { ...one, work: { ...one.work, workflow: { ...one.work.workflow, at: step } } });

export const s332JobWalkOnTheSpine: Scenario = {
  ...base,
  name: "job-walk-on-the-spine",
  says: "A Job at Build whose workflow still has Present ahead: its walk is pending in Next, then shown in Now, then approved in Completed",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(BUSY, (one) => auditLedger(one, { steps: FLOW, at: 1 }, []));
      let shownYet = false;
      return {
        ...walkActs(store, BUSY, WALK, AUDIT_TAKES, at(3)),
        // Time passes: Build is done and the Job shows the walk.
        later: () => {
          if (shownYet) return;
          shownYet = true;
          store.edit(BUSY, (one) => ({ ...at(2)(one), rows: [...one.rows, WALK], waitingFor: [...(one.waitingFor ?? []), ...waitingOn(WALK)] }));
        },
      };
    },
  },
};
