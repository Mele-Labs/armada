// A Job's Checks tab, and the filter row it shares with the Checks page, through `App` on a Fleet
// whose Job has Checks in every state asked from its gate, a Drone and the merge line. The owner,
// 7 Oct 2026: a tab of the Checks page narrowed to the Job, newest first, with filters above.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const FILTERS = ["All", "Active", "Waiting", "Completed", "Passed", "Failed", "Skipped"];

/** The Checks the list draws, top to bottom. */
const listed = () =>
  page
    .getByRole("region", { name: "Checks", exact: true })
    .getByRole("button")
    .elements()
    .map((one) => one.textContent);

async function openChecksTab(): Promise<void> {
  mount("job-checks");
  await onScreen();
  await page.getByRole("tab", { name: "Checks", exact: true }).click();
  await expect.element(page.getByRole("button", { name: "hooks_test", exact: true })).toBeVisible();
}

test("the strip draws Checks after Record, with no figure", async () => {
  mount("job-checks");
  await onScreen();
  const strip = page.getByRole("tablist", { name: "Job detail" });
  await expect.element(strip.getByRole("tab", { name: "Checks", exact: true })).toBeVisible();
  const names = strip.getByRole("tab").elements().map((one) => one.textContent ?? "");
  // Workflow and Settings carry their own figures; Checks, being a list, has none.
  expect(names.map((one) => one.replace(/\d+$/, ""))).toEqual(["Overview", "Workflow", "Plan", "Record", "Checks", "Drones", "Pulse", "Settings"]);
  expect(names[4]).toBe("Checks");
});

test("the tab lists this Job's Checks alone, newest first", async () => {
  await openChecksTab();
  await expect
    .poll(listed)
    .toEqual(["desktop_test", "hooks_test", "ipc_test", "fleet_test", "format", "components_test", "docs_test", "clippy"]);
});

test("each filter narrows the Job's Checks, and Completed holds passed, failed and skipped", async () => {
  await openChecksTab();
  const filters = page.getByRole("tab", { name: /^(All|Active|Waiting|Completed|Passed|Failed|Skipped)$/ });
  expect(filters.elements().map((one) => one.textContent)).toEqual(FILTERS);
  await expect.element(page.getByRole("tab", { name: "All", exact: true })).toHaveAttribute("aria-selected", "true");

  const under: Record<string, string[]> = {
    Active: ["hooks_test"],
    Waiting: ["desktop_test", "fleet_test"],
    Completed: ["ipc_test", "format", "components_test", "docs_test", "clippy"],
    Passed: ["ipc_test", "components_test"],
    Failed: ["format", "clippy"],
    Skipped: ["docs_test"],
  };
  for (const [filter, names] of Object.entries(under)) {
    await page.getByRole("tab", { name: filter, exact: true }).click();
    await expect.poll(listed, { message: filter }).toEqual(names);
  }
  await page.getByRole("tab", { name: "All", exact: true }).click();
  await expect.poll(() => listed().length).toBe(8);
});

test("a row opens its facts and log, and its Drone opens on this Job's Drones tab", async () => {
  await openChecksTab();
  await page.getByRole("button", { name: "format", exact: true }).click();
  const panel = page.getByRole("dialog", { name: "Check" });
  await expect.element(panel.getByText(/Diff in crates/)).toBeVisible();
  await panel.getByRole("button", { name: /Drone/ }).click();
  await expect.element(page.getByRole("tab", { name: "Drones", exact: true })).toHaveAttribute("aria-selected", "true");
});

test("the Checks page has the same filter row", async () => {
  mount("job-checks");
  await onScreen();
  await page.getByRole("navigation", { name: "Work" }).getByRole("button", { name: "Checks", exact: true }).click();
  await expect.element(page.getByRole("button", { name: "docs_test", exact: true })).toBeVisible();
  const filters = page.getByRole("tab", { name: /^(All|Active|Waiting|Completed|Passed|Failed|Skipped)$/ });
  expect(filters.elements().map((one) => one.textContent)).toEqual(FILTERS);

  await page.getByRole("tab", { name: "Skipped", exact: true }).click();
  await expect.poll(listed).toEqual(["docs_test"]);
  await page.getByRole("tab", { name: "Completed", exact: true }).click();
  // The checkout's own ended runs count here too, beside every Job's.
  await expect.poll(listed).toContain("typecheck");
  expect(listed()).toContain("docs_test");
  expect(listed()).not.toContain("hooks_test");
  expect(listed()).not.toContain("test");
});
