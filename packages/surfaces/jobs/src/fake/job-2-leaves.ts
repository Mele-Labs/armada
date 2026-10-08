// Job 2 at its review gate with a leaf of every kind off the step it fired at (8 Oct 2026): `fmt`
// passed, `gate` failed, `deploy_qa` failed and its repair Drone has a fix held for the owner, the
// Skill Trigger `tidy` has a Drone on it, and the step added after `implement` has one too. The
// branches are the same rows Fleet serves for a repair and a side run, so each stands where it
// does there; none of it is something Fleet served.

import type { JobTrigger } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { job2AtReviewWith } from "./job-2-at-review";
import { fixHeld, REPAIRED } from "./job-2-repair";
import { at, JOB_2_TRIGGERS } from "./job-2-triggers";
import { RUNNING_STEP, RUNNING_TRIGGER } from "./job-2-side-runs";

/** `deploy_qa` has failed, and its repair Drone has found a fix. */
const repaired = JOB_2_TRIGGERS.map((one): JobTrigger =>
  one.name === REPAIRED
    ? fixHeld({ ...one, state: "repairing", exit_code: 1, repair: { attempt: 1, branch: "armada/repair-deploy_qa-1" } })
    : // `smoke` blocks the Job and has failed: Rerun and Skip.
      one.name === "smoke"
      ? { ...one, state: "held", blocks: true, exit_code: 1, ended_at: at(30) }
      : one,
);

export function job2Leaves(): JobFixture {
  const fixture = job2AtReviewWith([...repaired, RUNNING_TRIGGER]);
  const named = { ...fixture, name: "Job 2, at its review gate, a leaf of every kind off its step" };
  if (named.watched.state !== "read") return named;
  return { ...named, watched: { ...named.watched, detail: { ...named.watched.detail, additions: [RUNNING_STEP] } } };
}
