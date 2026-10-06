// A Judge refusal he agreed with: the step stopped and the Job escalated
// (owner, Job 3, 2 Oct 2026). Overview still asked the question he had just
// answered, with `Answer it`, and the Workflow step panel said nothing about
// why the step stopped.

import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import { featureAfterAgreeing } from "@armada/jobs/fake";
import { onJob } from "./scenario";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

afterEach(async () => {
  await page.viewport(1440, 900);
});

const CRITERION = "addresses_the_request";

async function opened(): Promise<void> {
  await page.viewport(1600, 1000);
  const fixture = featureAfterAgreeing();
  mount(onJob(fixture));
  await expect.element(page.getByRole("button", { name: fixture.job.handle })).toBeVisible();
}

test("Overview's lead asks nothing, and offers what Fleet's recourse does", async () => {
  await opened();
  await expect.element(page.getByRole("heading", { name: "Stopped on a Judge refusal" })).toBeVisible();
  expect(page.getByRole("button", { name: "Answer it" }).query()).toBeNull();
  expect(page.getByText(/until you answer/).query()).toBeNull();
  const lead = document.querySelector(".armada-lead");
  for (const name of ["Overrule the verdict", "Restart step"]) {
    const act = page.getByRole("button", { name });
    await expect.element(act).toBeVisible();
    expect(lead?.contains(await act.element())).toBe(true);
  }
});

test("the Workflow step panel says why the step stopped, and offers the same acts", async () => {
  await opened();
  await page.getByRole("tab", { name: /^Workflow/ }).last().click();
  await page.getByRole("button", { name: /^Plan the change, / }).last().click();
  const why = page.getByRole("region", { name: "Why it stopped" }).last();
  await expect.element(why).toBeVisible();
  await expect.element(why.getByText(CRITERION)).toBeVisible();
  await expect.element(why.getByText(/T4 adds "Document the rule/)).toBeVisible();
  await expect.element(why.getByRole("button", { name: "Overrule the verdict" })).toBeVisible();
  await expect.element(why.getByRole("button", { name: "Restart step" })).toBeVisible();
});
