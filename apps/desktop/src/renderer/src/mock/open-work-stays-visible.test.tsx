// Work still open is never filed under a finished step: a draft or open pull request and a follow-up not yet settled stand in the current step and in the Open group.
// The owner, 28 Oct 2026: two open pull requests sat under Propose, folded away as if they were done.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const step = (name: string) => page.getByRole("region", { name, exact: true });

async function open() {
  mount("open-work-stays-visible");
  await onScreen();
  await page.getByRole("region", { name: "Jobs", exact: true }).getByRole("button", { name: "Issues tab beside Reviews" }).click();
  await expect.element(step("Walk")).toBeVisible();
}

test("an open pull request stands in the current step, and not under a finished one", async () => {
  await open();
  await expect.element(step("Walk").getByText("armada/change-3012").first()).toBeInTheDocument();
  for (const finished of ["Propose", "Discuss", "Approve plan"]) await expect.element(step(finished).getByText("armada/change-3012")).not.toBeInTheDocument();
});

test("a merged pull request stays under the step that made it", async () => {
  await open();
  await step("Discuss").getByRole("button", { name: /Discuss/ }).click();
  await expect.element(step("Discuss").getByText("Search issues by query").first()).toBeInTheDocument();
});

test("the Open group lists the open work and the follow-up after the last step", async () => {
  await open();
  await expect.element(step("Open").getByText("armada/change-3012").first()).toBeInTheDocument();
  await expect.element(step("Open").getByText("Deferred").first()).toBeInTheDocument();
});
