// A Job on the bug workflow, at Fix with Tests and Deliver to come, for the walk `jump-to-step`: each step to come offers Jump here beside Skip, and the draft
// takes a jump as Fleet will (`../jump-step.ts`), skipping the steps between. Over the same Job as `skip-a-step`.

import type { Session } from "@armada/screens/src/draft/sessions";

import { AUDIT_FILES, BUSY, ROTATION, approved, auditLedger } from "../audit-job";
import { jumpStep } from "../jump-step";
import type { Scenario } from "../moment";
import { skipStep } from "../skip-step";
import { s261OneSessionView as base } from "./one-session-view";

const FLOW = ["Find the cause", "Fix", "Tests", "Deliver"];

/** The Job's workflow after `stand` moves it: `at` and `skipped` replaced, the rest kept. */
const moved = (one: Session, stand: (at: number | undefined, skipped: readonly number[], steps: number) => { at: number | undefined; skipped: number[] } | undefined): Session => {
  const workflow = one.work?.workflow;
  if (one.work === undefined || workflow === undefined) return one;
  const stands = stand(workflow.at, workflow.skipped ?? [], workflow.steps.length);
  if (stands === undefined) return one;
  const { at: _at, skipped: _skipped, ...rest } = workflow;
  return { ...one, work: { ...one.work, workflow: { ...rest, ...(stands.at === undefined ? {} : { at: stands.at }), ...(stands.skipped.length === 0 ? {} : { skipped: stands.skipped }) } } };
};

const bugFix = (one: Session): Session => {
  const workflow = one.work?.workflow;
  if (one.work === undefined || workflow === undefined) return one;
  return { ...one, work: { ...one.work, workflow: { ...workflow, name: "Bug fix" }, proposal: { workflow: "Bug fix" } } };
};

export const s337JumpToStep: Scenario = {
  ...base,
  name: "jump-to-step",
  says: "A Job on the bug workflow at Fix, whose steps to come can be jumped to, skipping the steps between",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(BUSY, (one) => bugFix(auditLedger(one, { steps: FLOW, at: 1 }, [approved(ROTATION, "2026-10-26T13:58:00.000Z"), approved(AUDIT_FILES, "2026-10-26T14:06:00.000Z")])));
      if (store.work === undefined) return store;
      const work = {
        ...store.work,
        jump: (id: string, step: number) => store.edit(id, (one) => moved(one, (at, skipped, steps) => jumpStep({ at, skipped }, steps, step))),
        skip: (id: string, step: number, skip: boolean) => store.edit(id, (one) => moved(one, (at, skipped, steps) => skipStep({ at, skipped }, steps, step, skip))),
      };
      return { ...store, work };
    },
  },
};
