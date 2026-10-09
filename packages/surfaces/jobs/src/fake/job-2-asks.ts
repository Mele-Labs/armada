// Job 2 at its review gate with a Trigger on a destructive Command waiting on the owner (8 Oct 2026):
// `wipe_qa` blocks, so Fleet keeps the gate for him and the row carries the bell. The leaf off the step
// it fired at has Run and Skip. Every row is shaped as Fleet serves it (`packages/protocol/src/triggers.ts`,
// `trigger-holds.ts`).
//
// Invented: Job 2 was recorded before Triggers, so nothing here is something Fleet served.

import type { JobAlert, JobTrigger } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { alerted } from "./job-2-hold";
import { job2AtReviewWith } from "./job-2-at-review";
import { JOB_2_TRIGGERS } from "./job-2-triggers";

/** The Trigger on the destructive Command. */
export const ASKING = "wipe_qa";

/** What the Job's row says: the Trigger, where it fired and the step that delivers. */
export const ASKS_ALERT: JobAlert = { kind: "asks", trigger: ASKING, when: "pr_opened", step: "handoff" };

/** `fmt` passed, and `wipe_qa` waits on him with `block` on. */
const ROWS: JobTrigger[] = JOB_2_TRIGGERS.filter((one) => one.name === "fmt" || one.name === ASKING).map((one) =>
  one.name === ASKING ? { ...one, blocks: true } : one,
);

/** The same Job under an id of its own, so the walk that asks is its own. */
export const asksId = (): string => `${job2AtReviewWith(ROWS).job.id.slice(0, -1)}A`;

export function job2Asking(): JobFixture {
  const was = alerted(job2AtReviewWith(ROWS), ASKS_ALERT);
  const fixture = JSON.parse(JSON.stringify(was).replaceAll(was.job.id, asksId())) as JobFixture;
  return { ...fixture, name: "Job 2, at its review gate, a destructive Command waiting on the owner" };
}
