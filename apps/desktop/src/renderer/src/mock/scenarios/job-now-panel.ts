// The Now panel beside a Job's Overview canvas, one Job per state: nothing going on, a Drone at
// work, Checks landing, a Plan decision open, a Judge reading, and an issue. Mock data only: Fleet
// serves no plan interview, so the asks, issues and live rows ride on the draft, by Job id.
// The walk `job-now-panel` plays it.

import type { NowView } from "@armada/jobs/draft/now";
import { featureRunning } from "@armada/jobs/fake";

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

const fixtures = ROWS.map((row) => asRow(featureRunning(), row.at, row.slug, row.title));
const [, drone, checks, plan, judge, issue] = fixtures.map((one) => one.job.id) as [string, string, string, string, string, string];

const DRONE = { key: "d", of: "drone", name: "Drone on Implement", line: "Edit crates/store/src/clock.rs", state: "running" } as const;

const now: Record<string, NowView> = {
  [drone]: { running: [DRONE] },
  [checks]: {
    running: [
      DRONE,
      { key: "c1", of: "check", name: "typecheck", state: "passed" },
      { key: "c2", of: "check", name: "lint", state: "failed" },
      { key: "c3", of: "check", name: "store", state: "running" },
      { key: "c4", of: "check", name: "desktop_test", state: "running" },
    ],
  },
  [plan]: {
    asks: [
      {
        key: "p",
        kind: "plan",
        question: "Split the clock out of the writer, or wrap it in place?",
        options: [
          { id: "split", label: "Split it out" },
          { id: "wrap", label: "Wrap it in place" },
          { id: "defer", label: "Leave it for a later Job" },
        ],
      },
      { key: "j", kind: "judge", name: "Judge on Review the change", text: "Is the retry cap in scope?" },
      { key: "d", kind: "drone", name: "Drone on Implement", text: "Which clock does the fixture pin?" },
    ],
    running: [DRONE],
  },
  [judge]: {
    running: [
      { key: "j", of: "judge", name: "Judge on Review the change", state: "running" },
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
