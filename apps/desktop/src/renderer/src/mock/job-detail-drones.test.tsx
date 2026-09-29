// The Drones tab through `App`: a Drone's sheet names its step and its task as
// ways to them. The owner's notes of 29 Sep: *Implement takes me to the step
// panel on the workflow tab, T5 takes me to the task panel on the plan tab.*

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/** The feature Job mid-implement, with T5's Drone open in the Drones sheet. */
async function droneOnT5(): Promise<void> {
  mount("arc/executing-sequential");
  await page.getByRole("tab", { name: /^Drones/ }).last().click();
  await page.getByRole("button", { name: "Drone on T5" }).last().click();
  await expect.element(page.getByRole("dialog", { name: "Drone on T5" }).last()).toBeVisible();
}

test("the table reads how long each Drone has run, not when it started", async () => {
  mount("arc/executing-sequential");
  await page.getByRole("tab", { name: /^Drones/ }).last().click();
  await expect.element(page.getByRole("columnheader", { name: "Run time" }).last()).toBeVisible();
  expect(await page.getByRole("columnheader", { name: "Started" }).elements()).toHaveLength(0);
});

// The owner's note of 29 Sep: *a button that lets me kill the drone from here
// instead of on the job header* — and the header's kill becomes the Job's.

test("a running Drone's sheet offers its kill, and the header kills the Job", async () => {
  await droneOnT5();
  const sheet = page.getByRole("dialog", { name: "Drone on T5" }).last();
  await expect.element(sheet.getByRole("button", { name: "Hold to kill drone" })).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Hold to kill job" }).last()).toBeInTheDocument();
});

test("a Drone that has stopped offers no kill, and neither does the header", async () => {
  mount("arc/executing-sequential");
  await page.getByRole("tab", { name: /^Drones/ }).last().click();
  await page.getByRole("button", { name: "Drone on T1" }).last().click();
  await expect.element(page.getByRole("dialog", { name: "Drone on T1" }).last()).toBeVisible();
  expect(await page.getByRole("button", { name: "Hold to kill drone" }).elements()).toHaveLength(0);
});

test("pressing the step in a Drone's sheet lands on Workflow with that step's panel open", async () => {
  await droneOnT5();
  await page.getByRole("dialog", { name: "Drone on T5" }).last().getByRole("button", { name: "Implement", exact: true }).click();
  await expect.element(page.getByRole("tab", { name: /^Workflow/, selected: true }).last()).toBeVisible();
  await expect.element(page.getByRole("region", { name: "Implement, step" }).last()).toBeVisible();
});

test("pressing the task in a Drone's sheet lands on Plan with that task's sheet open", async () => {
  await droneOnT5();
  await page.getByRole("dialog", { name: "Drone on T5" }).last().getByRole("button", { name: "T5", exact: true }).click();
  await expect.element(page.getByRole("tab", { name: /^Plan/, selected: true }).last()).toBeVisible();
  const sheet = page.getByRole("dialog").last();
  await expect.element(sheet).toHaveTextContent("T5");
  await expect.element(sheet).toHaveTextContent("Run by its own agent");
});
