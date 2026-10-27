// A plan held behind a walk is not put to him anywhere: the spine draws it as the thread would, with no Approve until Fleet asks for it.
// The owner, 27 Oct 2026: a held plan showed Approve on the spine while the thread had no card for it.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const job = () => page.getByRole("region", { name: /^Job: / });
const approve = () => page.getByRole("button", { name: "Approve", exact: true });

async function open(scenario: string, title: string) {
  mount(scenario);
  await onScreen();
  await page.getByRole("region", { name: "Jobs", exact: true }).getByRole("button", { name: title }).click();
  await expect.element(job()).toBeVisible();
}

test("a plan held behind a walk shows on the spine with no Approve", async () => {
  await open("plan-waits-for-walk", "Hold the plan");
  await expect.element(page.getByText("Show the bay's warm time").first()).toBeVisible();
  await expect.element(approve()).not.toBeInTheDocument();
});

test("a plan Fleet asks for keeps Approve on the spine", async () => {
  await open("plan-revision-diff", "Warm the next bay");
  await expect.element(approve().first()).toBeVisible();
});
