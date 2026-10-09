// Job 2 at its review gate with a Script added to this Job only, on a destructive Command, waiting on the
// owner (8 Oct 2026): `wipe_qa` was added after the pull request opened and blocks, so Fleet keeps the gate
// for him and the row carries the bell. The leaf off the step it fired at has Run and Skip, as a saved
// Trigger's has (`job-2-asks.ts`). Shaped as Fleet serves it (`packages/protocol/src/added-steps.ts`).
//
// Invented: Job 2 was recorded before Triggers, so nothing here is something Fleet served.

import type { AddedStep, JobAlert } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { alerted } from "./job-2-hold";
import { job2AtReviewWith } from "./job-2-at-review";
import { at, JOB_2_TRIGGERS } from "./job-2-triggers";

/** The Command the Script runs. */
export const ADDED_ASKING = "wipe_qa";

/** What the Job's row says: the step's Command, where it fired and the step that delivers. */
export const ADDED_ASKS_ALERT: JobAlert = { kind: "asks", trigger: ADDED_ASKING, when: "pr_opened", step: "handoff" };

/** The Script, added while the Job ran, with `block` on. */
const ADDED: AddedStep = {
  id: "a1",
  runs: { kind: "script", command: ADDED_ASKING },
  when: "pr_opened",
  step: "handoff",
  block: true,
  repair: false,
  placed: "running",
  added_at: at(15),
  state: "awaiting_owner",
  started_at: at(21),
};

/** The same Job under an id of its own, so the walk that asks about an added step is its own. */
export const addedAsksId = (): string => `${job2AtReviewWith([]).job.id.slice(0, -1)}B`;

export function job2AddedAsking(): JobFixture {
  const base = job2AtReviewWith(JOB_2_TRIGGERS.filter((one) => one.name === "fmt"));
  if (base.watched.state !== "read") return base;
  const was = alerted({ ...base, watched: { ...base.watched, detail: { ...base.watched.detail, additions: [ADDED] } } }, ADDED_ASKS_ALERT);
  const fixture = JSON.parse(JSON.stringify(was).replaceAll(was.job.id, addedAsksId())) as JobFixture;
  return { ...fixture, name: "Job 2, at its review gate, an added step on a destructive Command waiting on the owner" };
}
