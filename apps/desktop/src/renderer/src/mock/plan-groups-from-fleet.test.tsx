// The Plan tab on a Job whose plan Fleet ran in groups (spike 022, slice 2):
// the groups are Fleet's, a failed task says so, and a Check pressed on a
// group's card opens that group's own run (#1652).

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

const groupCard = (ordinal: number) => page.getByRole("listitem", { name: `Group ${ordinal}`, exact: true });
const boundaryOf = (ordinal: number) => groupCard(ordinal).getByRole("region", { name: "Checks at this boundary" });
const taskRow = (id: string) => page.getByRole("listitem", { name: new RegExp(`^${id} `) });

async function onThePlanList() {
  await page.viewport(2000, 900);
  mount("real/groups-run-by-fleet");
  await page.getByRole("tab", { name: /^Plan/ }).click();
  await page.getByRole("tab", { name: "List" }).click();
  await expect.element(page.getByRole("list", { name: "Groups, in the order they run" })).toBeVisible();
}

test("the plan draws Fleet's three groups, not one per task, each holding its own tasks", async () => {
  await onThePlanList();
  for (const ordinal of [1, 2, 3]) await expect.element(groupCard(ordinal)).toBeVisible();
  expect(groupCard(4).query()).toBeNull();
  const first = page.getByRole("list", { name: "Group 1 tasks" }).element();
  expect([...first.children].map((one) => one.getAttribute("aria-label")?.split(" ")[0])).toEqual(["T1", "T2"]);
  await expect.element(groupCard(1)).toHaveTextContent("4c1b9d2");
});

test("the red group's task carries the failed mark, and says which group and run were still red", async () => {
  await onThePlanList();
  await expect.element(taskRow("T4").getByRole("img", { name: "Failed", exact: true })).toBeVisible();
  await expect.element(taskRow("T4")).toHaveTextContent("were still red on run 3, the last its retries allow");
  await expect.element(taskRow("T1").getByRole("img", { name: "Done", exact: true })).toBeVisible();
});

// #1652's own claim: group 3 also ran `test`, red on every run, and group 1's
// card opens group 1's run, which passed and held nothing back.
test("pressing test on group one's card opens group one's own run, though group three also ran test", async () => {
  await onThePlanList();
  await boundaryOf(1).getByRole("button", { name: /^Checks/ }).click();
  await boundaryOf(1).getByRole("button", { name: "test, passed" }).click();
  await expect.element(page.getByRole("tab", { name: /^Record/ })).toHaveAttribute("aria-selected", "true");
  await expect.element(page.getByRole("heading", { name: "test" })).toBeVisible();
  expect(page.getByText("Exited 1 — 2 of 1104 failed").query()).toBeNull();
  expect(page.getByText(/Blocked group/).query()).toBeNull();
});

test("a red run of test names the group it held back from the record, not by inference", async () => {
  await onThePlanList();
  await boundaryOf(3).getByRole("button", { name: "test, failed" }).click();
  await expect.element(page.getByRole("tab", { name: /^Record/ })).toHaveAttribute("aria-selected", "true");
  await expect.element(page.getByText("Exited 1 — 2 of 1104 failed")).toBeVisible();
  await expect.element(page.getByText("Blocked group 3 from passing.")).toBeVisible();
});
