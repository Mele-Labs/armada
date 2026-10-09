// What is asking, on the left where the canvas was, for an ask that drew no sketch: the asker's live
// output, what it changed so far with the file it asks about marked, and for a Judge the work product
// beside the checks it reads. One Job per asker (a Drone, a Judge, a Plan Drone). Mock data only:
// Fleet serves none of it, so it rides on the draft by Job id. The walks `job-asker-*` play them.

import type { NowAskerDraft, NowView } from "@armada/jobs/draft/now";
import { featureRunning } from "@armada/jobs/fake";

import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { asRow, holding } from "../holding";
import type { Scenario } from "../moment";
import { SHAPE_SPLIT, SHAPE_WRAP } from "../scenes";

const ROWS = [
  { at: 84, slug: "asker-drone", title: "Pin the store clock" },
  { at: 85, slug: "asker-judge", title: "Cap the retry backoff" },
  { at: 86, slug: "asker-plan", title: "Split the writer from the clock" },
] as const;

function uncleared(fixture: JobFixture): JobFixture {
  const { reclaimed_at: _cleared, ...job } = fixture.job;
  if (fixture.watched.state !== "read") return { ...fixture, job };
  const { reclaimed_at: _also, ...detailJob } = fixture.watched.detail.job;
  return { ...fixture, job, watched: { ...fixture.watched, detail: { ...fixture.watched.detail, job: detailJob } } };
}

const fixtures = ROWS.map((row) => uncleared(asRow(featureRunning(), row.at, row.slug, row.title)));
const [drone, judge, plan] = fixtures.map((one) => one.job.id) as [string, string, string];

const DRONE_ID = "01M3WJ6FGZ003DCX123T7W6YP1";

const DRONE_ASKER: NowAskerDraft = {
  name: "Implement Drone",
  of: "drone",
  step: "Implement",
  state: "waiting",
  actions: ["Read crates/store/src/clock.rs", "Read crates/store/tests/fixtures.rs", "Edit crates/store/tests/fixtures.rs"],
  tail: ["test writer::flushes_on_close ... ok", "test writer::stamps_each_batch ... FAILED", "  expected 02:00, got 11:43", "stopped: two clocks reach the fixture"],
  changed: [
    { path: "crates/store/src/clock.rs", change: "added", diff: ["+pub trait Clock {", "+    fn now(&self) -> Hour;", "+}"] },
    {
      path: "crates/store/tests/fixtures.rs",
      change: "changed",
      asking: true,
      diff: ["     let store = Store::open(dir);", "-    store.set_clock(wall());", "+    store.set_clock(pinned(Hour::new(2)));", "     store"],
    },
    { path: "crates/store/src/writer.rs", change: "changed", diff: ["-    let hour = wall_clock();", "+    let hour = self.clock.now();"] },
  ],
};

const JUDGE_ASKER: NowAskerDraft = {
  name: "Judge on Implement",
  of: "judge",
  step: "Implement",
  state: "running",
  actions: ["Read the diff against the criteria", "Read the typecheck output", "Read the lint output"],
  tail: ["Criterion 1: met", "Criterion 2: met", "Criterion 3: reading the retry cap"],
  product: {
    title: "Cap the retry backoff",
    lines: ["crates/store/src/backoff.rs   added", "crates/store/src/writer.rs    changed", "+    let next = (prev * 2).min(CAP);"],
  },
  checks: [
    { name: "typecheck", state: "passed" },
    { name: "lint", state: "failed", tail: ["backoff.rs:41 clippy::needless_return"] },
    { name: "store", state: "running", tail: ["test backoff::caps_at_the_cap ... ok", "test backoff::doubles_each_miss ..."] },
  ],
  changed: [
    { path: "crates/store/src/backoff.rs", change: "added", asking: true, diff: ["+pub const CAP: Duration = Duration::from_secs(30);"] },
    { path: "crates/store/src/writer.rs", change: "changed" },
  ],
};

const PLAN_ASKER: NowAskerDraft = {
  name: "Plan Drone",
  of: "drone",
  step: "Plan",
  state: "waiting",
  actions: ["Read docs/scope.md", "Read crates/store/src/writer.rs", "Write the plan"],
  tail: ["plan: split the clock out of the writer", "plan: pin the clock in the fixtures", "stopped: three decisions are yours"],
  changed: [
    { path: "crates/store/src/writer.rs", change: "changed", asking: true, diff: ["-    let hour = wall_clock();", "+    let hour = clock.now();"] },
    { path: "crates/store/src/clock.rs", change: "added" },
  ],
};

const now: Record<string, NowView> = {
  [drone]: { asks: [{ key: "d", kind: "drone", name: "Implement Drone", text: "Which clock does the fixture pin?", target: DRONE_ID, asker: DRONE_ASKER }] },
  [judge]: { asks: [{ key: "j", kind: "judge", name: "Judge on Implement", text: "Is the retry cap in scope?", asker: JUDGE_ASKER }] },
  [plan]: {
    asks: [
      {
        key: "p",
        kind: "plan",
        asker: PLAN_ASKER,
        decisions: [
          {
            id: "shape",
            question: "Split the clock out of the writer, or wrap it in place?",
            options: [
              { id: "split", label: "Split it out", sketch: { scene: SHAPE_SPLIT } },
              { id: "wrap", label: "Wrap it in place", sketch: { scene: SHAPE_WRAP } },
            ],
          },
          {
            id: "scope",
            question: "Take the retry cap in this Job?",
            options: [{ id: "in", label: "Take it here" }, { id: "out", label: "Keep it out" }],
          },
        ],
      },
    ],
  },
};

function opening(name: string, says: string, job: string): Scenario {
  return { ...holding(`job-asker-${name}`, `What is asking, on Overview: ${says}`, fixtures, { opens: job }), draft: { now } };
}

export const s206JobAskerDrone = opening("drone", "a Drone question with no sketch", drone);
export const s206JobAskerJudge = opening("judge", "a Judge question with no sketch", judge);
export const s206JobAskerPlan = opening("plan", "a Plan question, its options carrying sketches", plan);
