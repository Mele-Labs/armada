// Job 2 at its review gate with `deploy_qa`'s fix held for the owner, and the Board row carrying the
// alert for it (8 Oct 2026). The repair is already over, so nothing here moves on a timer: a walk
// that reaches this Job from another Job's lead finds the fix as it was left.
//
// Invented: Job 2 was recorded before Triggers, so nothing here is something Fleet served.

import type { JobAlert, JobTrigger } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { job2AtReviewWith } from "./job-2-at-review";
import { fixHeld, JOB_2_REPAIRING, REPAIRED } from "./job-2-repair";

/** What the Job's row says: the Trigger, where it fired and the step that delivers. */
export const FIX_ALERT: JobAlert = { kind: "fix_ready", trigger: REPAIRED, when: "pr_opened", step: "handoff" };

const WAITING: JobTrigger[] = JOB_2_REPAIRING.map((one) => (one.name === REPAIRED ? fixHeld(one) : one));

export function job2FixAlert(): JobFixture {
  const fixture = job2AtReviewWith(WAITING);
  if (fixture.watched.state !== "read") return { ...fixture, job: { ...fixture.job, alert: FIX_ALERT } };
  return {
    ...fixture,
    name: "Job 2, at its review gate, a fix held on an alert",
    job: { ...fixture.job, alert: FIX_ALERT },
    watched: { ...fixture.watched, detail: { ...fixture.watched.detail, job: { ...fixture.watched.detail.job, alert: FIX_ALERT } } },
  };
}
