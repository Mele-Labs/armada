// A Job on the feature workflow, at Walk with Refine and Deliver to come, for the walk `skip-a-step`: each step but the last offers Skip, from its icon and its
// band's right-press menu, and the draft takes a skip and an undo as Fleet will (`../skip-step.ts`). Over the same Job as `field-expand-plan`.

import type { Session } from "@armada/screens/src/draft/sessions";

import { AUDIT_FILES, BUSY, ROTATION, approved, auditLedger } from "../audit-job";
import type { Scenario } from "../moment";
import { skipStep } from "../skip-step";
import { s261OneSessionView as base } from "./one-session-view";

const FLOW = ["Propose", "Discuss", "Approve plan", "Walk", "Refine", "Deliver"];

const skipped = (one: Session, step: number, skip: boolean): Session => {
  const workflow = one.work?.workflow;
  if (one.work === undefined || workflow === undefined) return one;
  const stands = skipStep({ at: workflow.at, skipped: workflow.skipped ?? [] }, workflow.steps.length, step, skip);
  if (stands === undefined) return one;
  const { at: _at, skipped: _skipped, ...rest } = workflow;
  return { ...one, work: { ...one.work, workflow: { ...rest, ...(stands.at === undefined ? {} : { at: stands.at }), ...(stands.skipped.length === 0 ? {} : { skipped: stands.skipped }) } } };
};

export const s336SkipAStep: Scenario = {
  ...base,
  name: "skip-a-step",
  says: "A Job on the feature workflow at Walk, whose steps to come can be skipped and the skip undone",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(BUSY, (one) => auditLedger(one, { steps: FLOW, at: 3 }, [approved(ROTATION, "2026-10-26T13:58:00.000Z"), approved(AUDIT_FILES, "2026-10-26T14:06:00.000Z")]));
      return store.work === undefined ? store : { ...store, work: { ...store.work, skip: (id, step, skip) => store.edit(id, (one) => skipped(one, step, skip)) } };
    },
  },
};
