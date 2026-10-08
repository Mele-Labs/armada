// The Now panel beside a Job's Overview canvas, one Job per state: nothing going on, a Drone at
// work, Checks landing, a Plan decision open, a Judge reading, and an issue. Mock data only: Fleet
// serves no plan interview, so the asks, issues and live rows ride on the draft, by Job id.
// The walk `job-now-panel` plays it.

import type { NowView } from "@armada/jobs/draft/now";
import { featureRunning } from "@armada/jobs/fake";

import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { asRow, holding } from "../holding";
import type { Scenario } from "../moment";

const ROWS = [
  { at: 61, slug: "now-idle", title: "Tidy the store fixtures" },
  { at: 62, slug: "now-drone", title: "Pin the store clock" },
  { at: 63, slug: "now-checks", title: "Cache the manifest read" },
  { at: 64, slug: "now-plan", title: "Split the writer from the clock" },
  { at: 65, slug: "now-judge", title: "Cap the retry backoff" },
  { at: 66, slug: "now-issue", title: "Shorten the reconnect wait" },
] as const;

/** The recording's Job was cleared after it ran; these are on the Board's Running section instead. */
function uncleared(fixture: JobFixture): JobFixture {
  const { reclaimed_at: _cleared, ...job } = fixture.job;
  if (fixture.watched.state !== "read") return { ...fixture, job };
  const { reclaimed_at: _also, ...detailJob } = fixture.watched.detail.job;
  return { ...fixture, job, watched: { ...fixture.watched, detail: { ...fixture.watched.detail, job: detailJob } } };
}

const fixtures = ROWS.map((row) => uncleared(asRow(featureRunning(), row.at, row.slug, row.title)));
const [idle, drone, checks, plan, judge, issue] = fixtures.map((one) => one.job.id) as [string, string, string, string, string, string];

const IMPLEMENT = { id: "implement", name: "Implement" } as const;

const DRONE = {
  key: "d",
  of: "drone",
  name: "Drone on Implement",
  line: "Edit crates/store/src/clock.rs",
  step: IMPLEMENT,
  tail: ["Read crates/store/src/clock.rs", "Read crates/store/src/writer.rs", "Edit crates/store/src/clock.rs"],
  state: "running",
} as const;

const now: Record<string, NowView> = {
  [idle]: {},
  [drone]: { running: [DRONE] },
  [checks]: {
    running: [
      DRONE,
      { key: "c1", of: "check", name: "typecheck", step: IMPLEMENT, state: "passed" },
      { key: "c2", of: "check", name: "lint", step: IMPLEMENT, tail: ["clock.rs:41 clippy::needless_return", "clock.rs:58 clippy::let_and_return"], state: "failed" },
      { key: "c3", of: "check", name: "store", step: IMPLEMENT, tail: ["running 212 tests", "test clock::pins_the_hour ... ok", "test writer::flushes_on_close ..."], state: "running" },
      { key: "c4", of: "check", name: "desktop_test", step: IMPLEMENT, state: "running" },
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
      { key: "d", kind: "drone", name: "Drone on Implement", text: "Which clock does the fixture pin?" },
    ],
    running: [DRONE],
  },
  [judge]: {
    running: [
      { key: "j", of: "judge", name: "Judge on Implement", step: IMPLEMENT, tail: ["Reading the diff against the criteria", "Criterion 2: met"], state: "running" },
      { key: "c", of: "check", name: "typecheck", state: "passed" },
    ],
  },
  [issue]: {
    issues: [
      { key: "i1", of: "check", name: "store", text: "store: 2 failed", said: "Check failed" },
      { key: "i2", of: "drone", name: "Drone on Implement", text: "No output for 14m", said: "Drone stuck" },
    ],
    running: [{ key: "c", of: "check", name: "lint", state: "running" }],
  },
};

const base = holding("job-now-panel", "The Now panel on Overview: idle, a Drone, Checks landing, a Plan question, a Judge, an issue", fixtures);

export const s204JobNowPanel: Scenario = { ...base, draft: { now } };
