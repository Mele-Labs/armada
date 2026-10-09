// A Drone's sketch beside the Now panel, in place of the canvas while its ask is open: a Plan
// decision with a sketch of its own and one without, a Judge question with a sketch, and a Drone
// question with none. Mock data only: the sketches are mermaid source, and ride on the draft by Job id.
// A Plan decision carries the current state, and each of its options how the result looks if taken. The walks `job-sketch-*` play them, one per scenario.

import type { NowView } from "@armada/jobs/draft/now";
import { featureRunning } from "@armada/jobs/fake";

import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { asRow, holding } from "../holding";
import type { Scenario } from "../moment";
import { BACKOFF, SHAPE_LATER, SHAPE_NOW, SHAPE_SPLIT, SHAPE_WRAP, TESTS_FAKE, TESTS_NOW, TESTS_PIN } from "../scenes";

const ROWS = [
  { at: 81, slug: "sketch-plan", title: "Split the writer from the clock" },
  { at: 82, slug: "sketch-judge", title: "Cap the retry backoff" },
  { at: 83, slug: "sketch-none", title: "Pin the store clock" },
] as const;

function uncleared(fixture: JobFixture): JobFixture {
  const { reclaimed_at: _cleared, ...job } = fixture.job;
  if (fixture.watched.state !== "read") return { ...fixture, job };
  const { reclaimed_at: _also, ...detailJob } = fixture.watched.detail.job;
  return { ...fixture, job, watched: { ...fixture.watched, detail: { ...fixture.watched.detail, job: detailJob } } };
}

const fixtures = ROWS.map((row) => uncleared(asRow(featureRunning(), row.at, row.slug, row.title)));
const [plan, judge, none] = fixtures.map((one) => one.job.id) as [string, string, string];

const DRONE_ID = "01M3WJ6FGZ003DCX123T7W6YP1";

const sketch = (scene: unknown) => ({ scene });

const now: Record<string, NowView> = {
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
              { id: "split", label: "Split it out", sketch: sketch(SHAPE_SPLIT) },
              { id: "wrap", label: "Wrap it in place", sketch: sketch(SHAPE_WRAP) },
              { id: "defer", label: "Leave it for a later Job", sketch: sketch(SHAPE_LATER) },
            ],
            sketch: sketch(SHAPE_NOW),
          },
          {
            id: "tests",
            question: "Pin the clock in the existing fixtures, or add a fake?",
            options: [
              { id: "pin", label: "Pin it in the fixtures", sketch: sketch(TESTS_PIN) },
              { id: "fake", label: "Add a fake clock", sketch: sketch(TESTS_FAKE) },
            ],
            sketch: sketch(TESTS_NOW),
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
  [judge]: {
    asks: [{ key: "j", kind: "judge", name: "Judge on Implement", text: "Is the retry cap in scope?", sketch: sketch(BACKOFF) }],
  },
  [none]: {
    asks: [{ key: "d", kind: "drone", name: "Implement Drone", text: "Which clock does the fixture pin?", target: DRONE_ID }],
  },
};

function opening(name: string, says: string, job: string): Scenario {
  return { ...holding(`job-sketch-${name}`, `A Drone's sketch on Overview: ${says}`, fixtures, { opens: job }), draft: { now } };
}

export const s205JobSketchPlan = opening("plan", "a Plan decision, each with its own sketch", plan);
export const s205JobSketchJudge = opening("judge", "a Judge question with a sketch", judge);
export const s205JobSketchNone = opening("none", "a Drone question with no sketch", none);
