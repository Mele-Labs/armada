// A Job's run, read by a surface that is not the Job page: the flight deck mounts `JobRun` for the Job it has
// open. The claims are that it draws the steps of the Job's workflow and edits none of them, and that it opens at
// the readable scale on the step at work while Fit still reaches the whole of a long run.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { JobDetail, WorkflowSummary } from "@armada/protocol";
import { mount, rerender, unmount } from "@armada/screens/src/mounted";

import { refactorAtApproval } from "./fake";
import { JobRun } from "./job-run";

afterEach(unmount);

/** The Job at its gate, with its workflow's work step repeated until the run is far taller than the frame. */
function longRun(steps: number): { whole: JobDetail; workflows: WorkflowSummary[] } {
  const fixture = refactorAtApproval();
  if (fixture.watched.state !== "read") throw new Error("the fixture reads");
  const { detail } = fixture.watched;
  const work = detail.steps[1]!;
  const repeated = Array.from({ length: steps }, (_, at) => ({ ...work, step_id: `work-${at}`, label: `Work ${at}`, ordinal: at }));
  const whole: JobDetail = { ...detail, steps: repeated, job: { ...detail.job, current_step_id: "work-0" } };
  const workflows = fixture.workflows.map((one) =>
    one.id === whole.job.workflow_id ? { ...one, steps: repeated.map((step) => ({ step_id: step.step_id, label: step.label, checks: [], judge_checks: [], advance_gate: "auto_if_judge_passes", delivers: false, phase: "work" as const })) } : one,
  );
  return { whole, workflows };
}

const FRAME = { width: 900, height: 600 };

function framed(canvas: React.ReactElement): React.ReactElement {
  return <div style={{ ...FRAME, position: "relative" }}>{canvas}</div>;
}

/** The scale React Flow has the canvas at, off the element rather than out of a hook. */
const scaleOf = (): number => {
  const transform = document.querySelector<HTMLElement>(".react-flow__viewport")?.style.transform ?? "";
  const found = /scale\(([\d.]+)\)/.exec(transform);
  return found === null ? Number.NaN : Number(found[1]);
};

/** The scale the canvas opens a long run at: the legibility floor, on the step at work. */
const READABLE = 0.7;

test("JobRun draws the steps of the Job's workflow, and none of them can be moved or edited", async () => {
  const { whole, workflows } = longRun(3);
  mount(framed(<JobRun whole={whole} workflows={workflows} />));

  for (const label of ["Work 0", "Work 1", "Work 2"]) await expect.element(page.getByText(label, { exact: true }).first()).toBeVisible();
  expect(document.querySelectorAll(".react-flow__node.draggable")).toHaveLength(0);
  // A read canvas has no field to type into on any card.
  expect(document.querySelectorAll(".react-flow__node input, .react-flow__node textarea")).toHaveLength(0);
});

test("JobRun opens at the readable scale, and Fit reaches the whole of a long run", async () => {
  const { whole, workflows } = longRun(14);
  const run = (job: JobDetail) => framed(<JobRun whole={job} workflows={workflows} />);
  mount(run(whole));

  await expect.poll(scaleOf).toBeCloseTo(READABLE, 2);
  await userEvent.click(page.getByRole("button", { name: "Fit", exact: true }).element());
  // React Flow queues a fit and takes it when the next render hands it nodes to measure. A Job's detail is read
  // again every few seconds, which is what does it in the deck; the test hands it one.
  rerender(run({ ...whole }));
  await expect.poll(scaleOf).toBeLessThan(READABLE - 0.1);
});
