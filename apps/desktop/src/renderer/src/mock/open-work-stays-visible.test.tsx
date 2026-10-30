// Work still open is never filed under a finished step: a draft or open pull request and a follow-up not yet settled stand in the Open group, and in the current step instead where that group is hidden.
// The owner, 28 Oct 2026: two open pull requests sat under Propose, folded away as if they were done.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();

const step = (name: string) => page.getByRole("region", { name, exact: true });

async function open() {
  mount("open-work-stays-visible");
  await onScreen();
  await page.getByRole("region", { name: "Jobs", exact: true }).getByRole("button", { name: "Issues tab beside Reviews" }).click();
  await expect.element(step("Walk")).toBeVisible();
}

test("an open pull request stands in the Open group, and not under a finished step or the current one", async () => {
  await open();
  await expect.element(step("Open").getByText("armada/change-3012").first()).toBeInTheDocument();
  for (const other of ["Propose", "Discuss", "Approve plan", "Walk"]) await expect.element(step(other).getByText("armada/change-3012")).not.toBeInTheDocument();
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

// An issue a Job card carries is drawn once, as the chip in that card. Open leaves it out until its group is done: before the plan is approved, and after, whether or not the group is handed yet.
test("the Open group leaves out the issue of a group that goes to a Job of its own, handed or not, and keeps the one no group names", async () => {
  mount("plan-groups-as-jobs");
  await onScreen();
  await page.getByRole("region", { name: "Jobs", exact: true }).getByRole("button", { name: "Fix four filed issues" }).click();
  const keys = ["armada-95v", "armada-tzg", "armada-7gs", "armada-cwj"];
  const leftOut = async () => {
    await expect.element(step("Open").getByText("armada-m8r").first()).toBeInTheDocument();
    for (const key of keys) await expect.element(step("Open").getByText(key)).not.toBeInTheDocument();
  };
  await expect.element(step("Plan").getByRole("button", { name: /^Issue armada-7gs/ }).first()).toBeInTheDocument();
  await leftOut();
  // Approved, the first two groups are handed to Jobs and the other two are still not: the plan is no longer being edited, so only the group's own mark holds its issue out.
  timePasses();
  await expect.element(step("Dispatch").getByRole("button", { name: "Job A start_job rule act reads the workflow it names" })).toBeInTheDocument();
  await leftOut();
});
