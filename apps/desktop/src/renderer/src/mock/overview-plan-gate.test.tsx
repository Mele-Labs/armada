// Overview's review gate on a plan — through `App`.
//
// **The owner's, 30 Sep 2026** (`.claude/decisions/2026-09-30-overview-shows-
// the-plan-at-its-gate.md`): a plan waiting on him drew the work review, and
// asked him to approve without showing what he was approving. The gate now
// draws Plan's own review where the waiting step claimed a plan, and the work
// review everywhere else. What decides it is the claim's `evidence_type`.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { ARC_JOB_ID } from "@armada/screens/src/fixtures/build/arc";
import type { Evidence } from "@armada/protocol";

import type { Scenario } from "./scenario";
import { scenarioNamed } from "./scenario";
import { entered, mount, unmountAfterEach } from "./testing";

unmountAfterEach();

const PLAN_REVIEW = "arc/plan-review";

/** The plan-review moment, with the plan step's claim read as `evidence`. */
function claiming(evidence: Evidence): Scenario {
  const scenario = scenarioNamed(PLAN_REVIEW);
  if (scenario === undefined) throw new Error(`no scenario named ${PLAN_REVIEW}`);
  const reads = scenario.reads[ARC_JOB_ID];
  if (reads === undefined) throw new Error("the plan-review moment holds no reads for its Job");
  return {
    ...scenario,
    reads: { ...scenario.reads, [ARC_JOB_ID]: { ...reads, recorded: { ...reads.recorded, evidence } } },
  };
}

/** Overview's gate, as the plan it draws. */
async function thePlan() {
  const plan = page.getByRole("region", { name: "The plan" });
  await expect.element(plan).toBeVisible();
  return plan;
}

test("a plan waiting on a person is drawn on Overview, its groups and its tasks, in place of the work review", async () => {
  mount(PLAN_REVIEW);
  const plan = await thePlan();
  await plan.getByRole("tab", { name: "List" }).click();
  await expect.element(plan.getByRole("listitem", { name: "Group 1" })).toBeVisible();
  await expect.element(plan.getByRole("listitem", { name: /^T1 / })).toBeVisible();
  expect(page.getByRole("button", { name: "Approve the work" }).query()).toBeNull();
});

test("Approve the plan on Overview approves this Job", async () => {
  const app = mount(PLAN_REVIEW);
  const approveReview = vi.spyOn(app.api, "approveReview");
  const plan = await thePlan();
  await plan.getByRole("button", { name: "Approve the plan" }).click();
  await expect.poll(() => approveReview.mock.calls.length).toBe(1);
  expect(approveReview).toHaveBeenCalledWith(ARC_JOB_ID);
});

test("Propose a change on a task opened from Overview sends the planning Drone an instruction naming the task", async () => {
  const app = mount(PLAN_REVIEW);
  const redirect = vi.spyOn(app.api, "redirectDrone");
  const plan = await thePlan();
  await plan.getByRole("tab", { name: "List" }).click();
  await plan.getByRole("listitem", { name: /^T5 / }).getByRole("button").click();
  const task = page.getByRole("dialog", { name: "Draw what is running, in four lists" });
  await entered(task);
  await task.getByRole("button", { name: "Propose a change" }).click();
  await userEvent.type(task.getByRole("textbox", { name: "Propose a change" }), "Keep running-rows.tsx in T5");
  await task.getByRole("button", { name: "Send to the Drone" }).click();
  await expect.poll(() => redirect.mock.calls.length).toBe(1);
  const [jobId, instruction] = redirect.mock.calls[0]!;
  expect(jobId).toBe(ARC_JOB_ID);
  expect(instruction).toContain("on T5: Keep running-rows.tsx in T5");
});

const NOT_A_PLAN: Record<string, Evidence> = {
  "a document": {
    state: "read",
    jobId: ARC_JOB_ID,
    steps: [
      {
        step_id: "plan",
        evidence_type: "document",
        claimed: "The plan, written down.",
        shown_by: ".armada/deliverables/3-show-what-s-running/plan.1.md",
      },
    ],
  },
  nothing: { state: "read", jobId: ARC_JOB_ID, steps: [] },
};

test.each(Object.keys(NOT_A_PLAN))("a gate whose step claimed %s still draws the work review", async (claim) => {
  mount(claiming(NOT_A_PLAN[claim]!));
  await expect.element(page.getByRole("button", { name: "Approve the work" })).toBeVisible();
  expect(page.getByRole("region", { name: "The plan" }).query()).toBeNull();
  expect(page.getByRole("button", { name: "Approve the plan" }).query()).toBeNull();
});
