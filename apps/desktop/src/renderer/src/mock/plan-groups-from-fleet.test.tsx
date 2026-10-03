// The Plan tab on a Job whose plan Fleet ran in groups (spike 022, slice 2):
// the groups are Fleet's, a failed task says so, and a Check pressed on a
// group's card opens that group's own run (#1652).

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { featureJudgeQuestioned } from "./job-groups-fixture";
import { onJob } from "./scenario";
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
// card opens group 1's run, which passed and held nothing back. The press opens
// the Check's log, which goes on to the Record (owner, 2 Oct 2026).
const toTheRecord = () => page.getByRole("dialog", { name: "Check log" }).getByRole("button", { name: "Open in the Record" });

test("pressing test on group one's card opens group one's own run, though group three also ran test", async () => {
  await onThePlanList();
  await boundaryOf(1).getByRole("button", { name: /^Checks/ }).click();
  await boundaryOf(1).getByRole("button", { name: "test, passed" }).click();
  await toTheRecord().click();
  await expect.element(page.getByRole("tab", { name: /^Record/ })).toHaveAttribute("aria-selected", "true");
  await expect.element(page.getByRole("heading", { name: "test" })).toBeVisible();
  expect(page.getByText("Exited 1 — 2 of 1104 failed").query()).toBeNull();
  expect(page.getByText(/Blocked group/).query()).toBeNull();
});

test("a red run of test names the group it held back from the record, not by inference", async () => {
  await onThePlanList();
  await boundaryOf(3).getByRole("button", { name: "test, failed" }).click();
  await toTheRecord().click();
  await expect.element(page.getByRole("tab", { name: /^Record/ })).toHaveAttribute("aria-selected", "true");
  await expect.element(page.getByText("Exited 1 — 2 of 1104 failed")).toBeVisible();
  await expect.element(page.getByText("Blocked group 3 from passing.")).toBeVisible();
});

// The owner's, 2 Oct 2026: a group's state is a mark with a tooltip naming it,
// in the list as in the graph, and never a word.
test("a group's state is a mark naming it, not a word", async () => {
  await onThePlanList();
  // The head's own mark, by its slot: a task's mark names its own state.
  const head = (ordinal: number) => groupCard(ordinal).element().querySelector(".armada-plan-board__state");
  expect([1, 2, 3].map((ordinal) => [head(ordinal)?.getAttribute("role"), head(ordinal)?.getAttribute("aria-label")])).toEqual([
    ["img", "Passed"],
    ["img", "Passed"],
    ["img", "Failed"],
  ]);
  expect(groupCard(1).getByText("passed", { exact: true }).query()).toBeNull();
});

// The owner's, 2 Oct 2026: after a Judge refusal, run one task again is one
// press. T4 reads done, since the Checks passed, and its panel offers Restart.
test("a done task in a group the Judge refused offers Restart this task alone, and the press works it again", async () => {
  await page.viewport(2000, 900);
  mount("real/groups-judge-refused");
  await page.getByRole("tab", { name: /^Plan/ }).click();
  await page.getByRole("tab", { name: "List" }).click();
  await expect.element(taskRow("T4").getByRole("img", { name: "Done", exact: true })).toBeVisible();
  await taskRow("T4").getByRole("button").first().click();
  const panel = page.getByRole("dialog", { name: "Answer restart and move in the mock" });
  const restart = panel.getByRole("button", { name: "Restart this task" });
  await expect.element(restart).toBeVisible();
  expect(panel.getByRole("button", { name: "Pilot", exact: true }).query()).toBeNull();
  await restart.click();
  await userEvent.keyboard("{Escape}");
  await expect.element(taskRow("T4").getByRole("img", { name: "Working", exact: true })).toBeVisible();
  expect(page.getByText("Not implemented", { exact: true }).query()).toBeNull();
});

test("a done task in a group that passed offers no Restart", async () => {
  await onThePlanList();
  await taskRow("T1").getByRole("button").first().click();
  const panel = page.getByRole("dialog", { name: "Draw a group's runs on its card" });
  await expect.element(panel).toBeVisible();
  expect(panel.getByRole("button", { name: "Restart this task" }).query()).toBeNull();
});

// The owner's, 2 Oct 2026: a group the Judge only questioned waits for his
// answer, so its done task offers no Restart.
test("a done task in a group the Judge only questioned offers no Restart", async () => {
  await page.viewport(2000, 900);
  mount(onJob(featureJudgeQuestioned()));
  await page.getByRole("tab", { name: /^Plan/ }).click();
  await page.getByRole("tab", { name: "List" }).click();
  await expect.element(taskRow("T4").getByRole("img", { name: "Done", exact: true })).toBeVisible();
  await taskRow("T4").getByRole("button").first().click();
  const panel = page.getByRole("dialog", { name: "Answer restart and move in the mock" });
  await expect.element(panel).toBeVisible();
  expect(panel.getByRole("button", { name: "Restart this task" }).query()).toBeNull();
});
