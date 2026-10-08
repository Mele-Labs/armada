// The Now panel beside a Job's Overview canvas, one Job per state: a Drone at work, Checks landing,
// a Plan decision open, a Judge reading, an issue, and each reason a Job runs nothing (a resource,
// other Jobs, a transition, a one-off step), then a Job with no reason at all, which is a defect. Mock data only: Fleet
// serves no plan interview, so the asks, issues and live rows ride on the draft, by Job id.
// The walks `job-now-*` play them, one per scenario.

import type { NowView } from "@armada/jobs/draft/now";
import { featureRunning } from "@armada/jobs/fake";

import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { asRow, holding } from "../holding";
import type { Scenario } from "../moment";

const ROWS = [
  { at: 61, slug: "now-none", title: "Tidy the store fixtures" },
  { at: 62, slug: "now-drone", title: "Pin the store clock" },
  { at: 63, slug: "now-checks", title: "Cache the manifest read" },
  { at: 64, slug: "now-plan", title: "Split the writer from the clock" },
  { at: 65, slug: "now-judge", title: "Cap the retry backoff" },
  { at: 66, slug: "now-issue", title: "Shorten the reconnect wait" },
  { at: 67, slug: "now-resource", title: "Index the record files" },
  { at: 68, slug: "now-jobs", title: "Move the store reads" },
  { at: 69, slug: "now-transition", title: "Rename the gate verbs" },
  { at: 70, slug: "now-oneoff", title: "Rebuild the fixtures" },
] as const;

/** The recording's Job was cleared after it ran; these are on the Board's Running section instead. */
function uncleared(fixture: JobFixture): JobFixture {
  const { reclaimed_at: _cleared, ...job } = fixture.job;
  if (fixture.watched.state !== "read") return { ...fixture, job };
  const { reclaimed_at: _also, ...detailJob } = fixture.watched.detail.job;
  return { ...fixture, job, watched: { ...fixture.watched, detail: { ...fixture.watched.detail, job: detailJob } } };
}

const fixtures = ROWS.map((row) => uncleared(asRow(featureRunning(), row.at, row.slug, row.title)));
const [none, drone, checks, plan, judge, issue, resource, jobs, transition, oneoff] = fixtures.map((one) => one.job.id) as [
  string, string, string, string, string, string, string, string, string, string,
];

const IMPLEMENT = { id: "implement", name: "Implement" } as const;
const PLAN_STEP = { id: "plan", name: "Plan" } as const;
const HANDOFF = { id: "handoff", name: "Handoff" } as const;

/** The Job's own Drone, which the Drones tab lists. A Drone is named after its step. */
const DRONE_ID = "01M3WJ6FGZ003DCX123T7W6YP1";

const DRONE = {
  key: "d",
  of: "drone",
  name: "Implement Drone",
  line: "Edit crates/store/src/clock.rs",
  step: IMPLEMENT,
  tail: ["Read crates/store/src/clock.rs", "Read crates/store/src/writer.rs", "Edit crates/store/src/clock.rs"],
  state: "running",
  target: DRONE_ID,
} as const;

const CHECK_ACTS = [
  { key: "retry", glyph: "retry", said: "Retry now" },
  { key: "skip", glyph: "skip", said: "Skip check" },
] as const;

const now: Record<string, NowView> = {
  [none]: {},
  [drone]: { running: [DRONE] },
  [checks]: {
    running: [
      DRONE,
      { key: "c1", of: "check", name: "typecheck", step: IMPLEMENT, state: "passed", target: "typecheck" },
      { key: "c2", of: "check", name: "lint", step: IMPLEMENT, tail: ["clock.rs:41 clippy::needless_return", "clock.rs:58 clippy::let_and_return"], state: "failed", acts: CHECK_ACTS, target: "lint" },
      { key: "c3", of: "check", name: "store", step: IMPLEMENT, tail: ["running 212 tests", "test clock::pins_the_hour ... ok", "test writer::flushes_on_close ..."], state: "running", acts: CHECK_ACTS, target: "store" },
      { key: "c4", of: "check", name: "desktop_test", step: IMPLEMENT, state: "running", acts: CHECK_ACTS, target: "desktop_test" },
    ],
  },
  [plan]: {
    asks: [
      {
        key: "p",
        kind: "plan",
        decisions: [
          {
            id: "shape",
            question: "Split the clock out of the writer, or wrap it in place?",
            options: [
              { id: "split", label: "Split it out" },
              { id: "wrap", label: "Wrap it in place" },
              { id: "defer", label: "Leave it for a later Job" },
            ],
          },
          {
            id: "tests",
            question: "Pin the clock in the existing fixtures, or add a fake?",
            options: [
              { id: "pin", label: "Pin it in the fixtures" },
              { id: "fake", label: "Add a fake clock" },
            ],
          },
          {
            id: "scope",
            question: "Take the retry cap in this Job?",
            options: [
              { id: "in", label: "Take it here" },
              { id: "out", label: "Keep it out" },
            ],
          },
        ],
      },
      { key: "j", kind: "judge", name: "Judge on Implement", text: "Is the retry cap in scope?" },
      { key: "d", kind: "drone", name: "Implement Drone", text: "Which clock does the fixture pin?", target: DRONE_ID },
    ],
    running: [DRONE],
  },
  [judge]: {
    running: [
      { key: "j", of: "judge", name: "Judge on Implement", step: IMPLEMENT, tail: ["Reading the diff against the criteria", "Criterion 2: met"], state: "running" },
      { key: "c", of: "check", name: "typecheck", step: IMPLEMENT, state: "passed", target: "typecheck" },
    ],
  },
  [issue]: {
    issues: [
      { key: "i1", of: "check", name: "store", text: "store: 2 failed", said: "Check failed", step: IMPLEMENT, acts: CHECK_ACTS, target: "store" },
      {
        key: "i2",
        of: "drone",
        name: "Implement Drone",
        text: "No output for 14m",
        said: "Drone stuck",
        step: IMPLEMENT,
        acts: [{ key: "redirect", glyph: "redirect", said: "Redirect Drone" }, { key: "restart", glyph: "retry_step", said: "Retry step" }],
        target: DRONE_ID,
      },
    ],
    running: [{ key: "c", of: "check", name: "lint", step: IMPLEMENT, state: "running", acts: CHECK_ACTS, target: "lint" }],
  },
  [resource]: { waiting: [{ key: "r", kind: "resource", text: "Worktree slot", step: IMPLEMENT }] },
  [jobs]: {
    waiting: [
      { key: "j1", kind: "job", text: "Pin the store clock", target: drone },
      { key: "j2", kind: "job", text: "Cache the manifest read", target: checks },
    ],
  },
  [transition]: { waiting: [{ key: "t", kind: "transition", text: "Plan to Implement", step: PLAN_STEP }] },
  [oneoff]: { waiting: [{ key: "o", kind: "step", text: "Handoff, a one-off step", step: HANDOFF }] },
};

const FLEET_SAYS = "The Now panel on Overview";

/** One scenario per Job, opened on it, so a walk starts on its state and not on the Board. */
function opening(name: string, says: string, job: string): Scenario {
  return { ...holding(`job-now-${name}`, `${FLEET_SAYS}: ${says}`, fixtures, { opens: job }), draft: { now } };
}

export const s204JobNowIdle = opening("idle", "a Job with nothing to say, which reads as a defect", none);
export const s204JobNowDrone = opening("drone", "one Drone at work, its output open", drone);
export const s204JobNowChecks = opening("checks", "Checks landing, with quick acts", checks);
export const s204JobNowPlan = opening("plan", "a Plan decision asked one at a time", plan);
export const s204JobNowJudge = opening("judge", "a Judge reading", judge);
export const s204JobNowIssue = opening("issue", "a failed Check and a stuck Drone, with their fixes", issue);
export const s204JobNowResource = opening("resource", "waiting on a resource", resource);
export const s204JobNowJobs = opening("jobs", "waiting on other Jobs", jobs);
export const s204JobNowTransition = opening("transition", "between two steps", transition);
export const s204JobNowOneOff = opening("one-off", "running a one-off step", oneoff);
