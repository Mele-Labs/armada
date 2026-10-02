// One Judge refusal, one answer, on every surface (owner, 1 Oct 2026, `#1748`
// row 13).
//
// **On his Job 2 three surfaces told three stories.** Overview asked the
// Judge's question with its three answers; Plan said the plan was waiting and
// offered Approve the plan, which skipped the question; the Workflow step panel
// offered nothing but Hold to stop. Plan and the step panel now draw
// Overview's own block, with the same handler.

import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import { featureAtAPlanRefusal } from "./job-detail-refusal";
import { onJob } from "./scenario";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

beforeEach(() => window.localStorage.removeItem("armada.bridge.workflow-view"));
afterEach(async () => {
  await page.viewport(1440, 900);
});

const ANSWERS = ["Agree with the refusal", "Disagree, just this step", "Always disagree"];

const refusal = () => page.getByRole("region", { name: "A judge refusal you are being asked about" }).last();

async function opened() {
  await page.viewport(1600, 1000);
  const fixture = featureAtAPlanRefusal();
  const app = mount(onJob(fixture));
  await expect.element(page.getByRole("button", { name: fixture.job.handle })).toBeVisible();
  return { api: app.api, jobId: fixture.job.id };
}

async function answersAndNoApproval(): Promise<void> {
  await expect.element(refusal()).toBeVisible();
  await expect.element(refusal().getByText("Does the plan address what was asked, and nothing beyond it?")).toBeVisible();
  for (const name of ANSWERS) await expect.element(refusal().getByRole("button", { name })).toBeVisible();
  expect(page.getByRole("button", { name: "Approve the plan" }).query()).toBeNull();
}

test("Plan draws Overview's refusal and its three answers, and no Approve the plan", async () => {
  await opened();
  await page.getByRole("tab", { name: /^Plan/ }).last().click();
  await answersAndNoApproval();
});

test("Disagree, just this step on Plan answers the Judge as Overview's does", async () => {
  const { api, jobId } = await opened();
  const answerJudge = vi.spyOn(api, "answerJudge");
  await page.getByRole("tab", { name: /^Plan/ }).last().click();
  await refusal().getByRole("button", { name: "Disagree, just this step" }).click();
  expect(answerJudge).toHaveBeenCalledWith(jobId, "2026-10-01T20:26:21.196Z", "disagree_once", undefined);
});

test("Overview's own block sends the same answer", async () => {
  const { api, jobId } = await opened();
  const answerJudge = vi.spyOn(api, "answerJudge");
  await refusal().getByRole("button", { name: "Disagree, just this step" }).click();
  expect(answerJudge).toHaveBeenCalledWith(jobId, "2026-10-01T20:26:21.196Z", "disagree_once", undefined);
});

test("the Workflow step panel draws the refusal and its three answers", async () => {
  await opened();
  await page.getByRole("tab", { name: /^Workflow/ }).last().click();
  await page.getByRole("button", { name: /^Plan the change, / }).last().click();
  await expect.element(page.getByRole("region", { name: "Question for you" }).last()).toBeVisible();
  await answersAndNoApproval();
});

// `#1748` row 14: inside Overview's lead the block drew a second yellow rule
// beside the card's own accent, two parallel lines the owner called terrible.
test("the refusal draws no rule of its own beside the lead card's accent", async () => {
  await opened();
  await expect.element(refusal()).toBeVisible();
  expect(Number.parseFloat(getComputedStyle(refusal().element()).borderInlineStartWidth)).toBe(0);
});
