// The Judge did not answer on a step (owner, Job 3, 5 Oct 2026): Overview said
// "This Job the gate could not decide", offered only asking again, and drew the
// step's Checks as not run beside a panel that said they passed.

import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import { featureUndecided } from "./job-detail-undecided";
import { onJob } from "./scenario";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

afterEach(async () => {
  await page.viewport(1440, 900);
});

async function opened(): Promise<void> {
  await page.viewport(1600, 1000);
  const fixture = featureUndecided();
  mount(onJob(fixture));
  await expect.element(page.getByRole("button", { name: fixture.job.handle })).toBeVisible();
}

test("Overview says the Judge did not answer, in plain words, with both ways on", async () => {
  await opened();
  await expect.element(page.getByRole("heading", { name: "The Judge did not answer" })).toBeVisible();
  await expect.element(page.getByText("The work was not judged. Ask again, or accept the step yourself.")).toBeVisible();
  expect(document.body.textContent).not.toMatch(/\bgate\b/i);
  const lead = document.querySelector(".armada-lead");
  for (const name of ["Accept this step", "Ask the Judge again", "Redirect drone", "Read what stopped it"]) {
    const act = page.getByRole("button", { name, exact: true });
    await expect.element(act).toBeVisible();
    expect(lead?.contains(await act.element())).toBe(true);
  }
});

test("the step's panel offers the same two ways, and its Checks passed", async () => {
  await opened();
  await page.getByRole("tab", { name: /^Workflow/ }).last().click();
  await page.getByRole("button", { name: /^Plan the change, / }).last().click();
  await expect.element(page.getByRole("button", { name: "Accept this step", exact: true }).last()).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Ask the Judge again", exact: true }).last()).toBeVisible();
  await expect.element(page.getByText("plan_recorded").last()).toBeVisible();
});

test("accepting the step needs no reason, and the Job carries on", async () => {
  await opened();
  await page.getByRole("button", { name: "Accept this step", exact: true }).last().click();
  const confirm = page.getByRole("dialog").getByRole("button", { name: "Accept this step", exact: true });
  await expect.element(confirm).toBeEnabled();
  await confirm.click();
  await expect.element(page.getByRole("heading", { name: "The Judge did not answer" })).not.toBeInTheDocument();
  expect(page.getByRole("button", { name: "Ask the Judge again", exact: true }).query()).toBeNull();
});

test("asking again with the Judge still silent moves nothing", async () => {
  await opened();
  await page.getByRole("button", { name: "Ask the Judge again", exact: true }).last().click();
  await expect.element(page.getByRole("heading", { name: "The Judge did not answer" })).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Accept this step", exact: true }).last()).toBeVisible();
});
