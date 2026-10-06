// A Job whose tasks ran on the model their tier, or a person, picked (spike
// 022, slice 3): each task's model is its pick or its tier's off the Job's
// map, a tier the map leaves out draws no model, each Drone names the model it
// ran, and Edit this task keeps a new model on the task.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { entered, mount, unmountAfterEach } from "@armada/desktop/mock";

unmountAfterEach();

// Core and Jobs only: the surface's own members, and the scenario answers the rest.
const SLICES = { slices: ["core", "jobs"] } as const;

const taskRow = (id: string) => page.getByRole("listitem", { name: new RegExp(`^${id} `) });
// The cell's own text, without the tooltip's label beside it.
const modelOf = (id: string) =>
  taskRow(id).element().querySelector(".armada-plan-board__task-model")?.childNodes[0]?.textContent ?? "";

async function onThePlanList() {
  await page.viewport(2000, 900);
  mount("real/tiers-and-models", SLICES);
  await page.getByRole("tab", { name: /^Plan/ }).click();
  await page.getByRole("tab", { name: "List" }).click();
  await expect.element(page.getByRole("list", { name: "Groups, in the order they run" })).toBeVisible();
}

test("each task's row names the model its tier, or a person, picked, and a tier the map leaves out names none", async () => {
  await onThePlanList();
  await expect.poll(() => ["T1", "T2", "T3", "T4"].map(modelOf)).toEqual(["opus", "haiku", "", "opus"]);
});

test("a task's panel says its tier and model, and each of its Drones the model it ran", async () => {
  await onThePlanList();
  await taskRow("T4").getByRole("button", { name: /Answer restart and move/ }).click();
  const task = page.getByRole("dialog", { name: "Answer restart and move in the mock" });
  await entered(task);
  await expect.element(task.getByText("medium · opus")).toBeVisible();
  // The Drone row's own cells, without the tooltip's label beside them.
  const ranOn = () =>
    [...task.element().querySelectorAll(".armada-task-sheet__drone-spent")].map((cell) => cell.childNodes[0]?.textContent);
  await expect.poll(ranOn).toContain("sonnet");
});

test("the Drones tab names the model each Drone ran", async () => {
  await page.viewport(2000, 900);
  mount("real/tiers-and-models", SLICES);
  await page.getByRole("tab", { name: /^Drones/ }).click();
  await expect.element(page.getByRole("columnheader", { name: "Model" })).toBeVisible();
  const cells = () =>
    [...document.querySelectorAll(".armada-drones__model")].map((cell) => cell.textContent ?? "");
  await expect.poll(() => cells().sort()).toEqual(["haiku", "opus", "sonnet", "sonnet"]);
});

test("Edit this task on the failed task keeps the model picked, over the map", async () => {
  await onThePlanList();
  await taskRow("T4").getByRole("button", { name: /Answer restart and move/ }).click();
  const task = page.getByRole("dialog", { name: "Answer restart and move in the mock" });
  await entered(task);
  await task.getByRole("button", { name: "Edit this task" }).click();
  await userEvent.selectOptions(task.getByLabelText("Model"), "haiku");
  await task.getByRole("button", { name: "Save" }).click();
  await expect.element(task.getByText("medium · haiku")).toBeVisible();
  expect(page.getByText("Not implemented", { exact: true }).query()).toBeNull();
});
