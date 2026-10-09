// Job 2 at its review gate with a blocking Trigger holding it (7 Oct 2026): `deploy_qa` has `block` on and
// failed after the pull request opened, so Fleet keeps the gate for the owner. The Job does not escalate
// for a hold at `pr_opened`; the row carries `alert`. Every row is shaped as Fleet serves it
// (`packages/protocol/src/triggers.ts`, `trigger-holds.ts`), and each move below is the row Fleet writes next.
//
// Invented: Job 2 was recorded before Triggers, so nothing here is something Fleet served.

import type { JobAlert, JobTrigger } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { job2AtReviewWith } from "./job-2-at-review";
import { at, JOB_2_TRIGGERS } from "./job-2-triggers";

/** The blocking Trigger that holds the Job. */
export const HELD = "deploy_qa";

/** What the Job's row says: the Trigger, where it fired and the step that delivers. */
export const HOLD_ALERT: JobAlert = { kind: "held", trigger: HELD, when: "pr_opened", step: "handoff" };

/** `deploy_qa` blocks and failed: the Job waits on the owner. */
const HELD_ROW: JobTrigger = {
  name: HELD,
  when: "pr_opened",
  step: "handoff",
  level: "machine",
  state: "held",
  exit_code: 1,
  started_at: at(20),
  ended_at: at(31),
  log_at: at(31),
  blocks: true,
};

/** Job 2's Triggers, with `deploy_qa` holding the Job. `wipe_qa` is left out: a Trigger asking him would ring the Job's bell on its own. */
export const JOB_2_HELD: JobTrigger[] = JOB_2_TRIGGERS.filter((one) => one.state !== "awaiting_owner").map((one) => (one.name === HELD ? HELD_ROW : one));

/** The fixture with the row's alert, and the detail's, put on. */
export function alerted(fixture: JobFixture, alert: JobAlert | undefined): JobFixture {
  const { alert: _was, ...job } = fixture.job;
  const row = alert === undefined ? job : { ...job, alert };
  if (fixture.watched.state !== "read") return { ...fixture, job: row };
  const { alert: _alert, ...inner } = fixture.watched.detail.job;
  return {
    ...fixture,
    job: row,
    watched: { ...fixture.watched, detail: { ...fixture.watched.detail, job: alert === undefined ? inner : { ...inner, alert } } },
  };
}

/** The Job at its review gate with `deploy_qa` holding it. */
export function job2Held(): JobFixture {
  return { ...alerted(job2AtReviewWith(JOB_2_HELD), HOLD_ALERT), name: "Job 2, at its review gate, held by a Trigger" };
}

/** The same Job under an id of its own, so the walk that skips the Trigger is its own. */
export const holdSkippedId = (): string => `${job2Held().job.id.slice(0, -1)}S`;

export function job2HeldAndSkipped(): JobFixture {
  const was = job2Held();
  const fixture = JSON.parse(JSON.stringify(was).replaceAll(was.job.id, holdSkippedId())) as JobFixture;
  return { ...fixture, name: "Job 2, at its review gate, held by a Trigger the owner skips" };
}

/** The Command passes again: the firing is `passed` and the Job holds nothing. */
export const reran = (one: JobTrigger, second: number): JobTrigger => ({ ...one, state: "passed", exit_code: 0, ended_at: at(second) });

/** The owner let it go: skipped, and the reason is his. */
export const skippedByOwner = (one: JobTrigger): JobTrigger => ({
  ...one,
  state: "skipped",
  skipped: { reason: "by_owner", name: one.name, said: `\`${one.name}\` was skipped by the owner` },
});

/** The fixture with the hold let go: its alert gone from the row and the detail. */
export const released = (fixture: JobFixture): JobFixture => alerted(fixture, undefined);
