// The Drones tab through `App`: a Drone's sheet names its step and its task as
// ways to them. The owner's notes of 29 Sep: *Implement takes me to the step
// panel on the workflow tab, T5 takes me to the task panel on the plan tab.*

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { motion, mount, unmountAfterEach } from "@armada/desktop/mock";

unmountAfterEach();

// Core and Jobs only: the surface's own members, and the scenario answers the rest.
const SLICES = { slices: ["core", "jobs"] } as const;

/** The feature Job mid-implement, with T5's Drone open in the Drones sheet. */
async function droneOnT5(): Promise<void> {
  mount("arc/executing-sequential", SLICES);
  await page.getByRole("tab", { name: /^Drones/ }).last().click();
  await page.getByRole("button", { name: "Drone on T5" }).last().click();
  const sheet = page.getByRole("dialog", { name: "Drone on T5" }).last();
  await expect.element(sheet).toBeVisible();
  // **Visible is not arrived.** The sheet travels in from the trailing edge at
  // `--duration-sheet`, and a press aimed at its subtitle mid-travel went to
  // where T5 was a frame earlier — off the window's edge, reaching nothing.
  // Measured 29 Sep 2026: 2 in 12 runs. Wait the travel out.
  await Promise.all(sheet.element().getAnimations().map((one) => one.finished));
}

test("the table reads how long each Drone has run, not when it started", async () => {
  mount("arc/executing-sequential", SLICES);
  await page.getByRole("tab", { name: /^Drones/ }).last().click();
  await expect.element(page.getByRole("columnheader", { name: "Run time" }).last()).toBeVisible();
  expect(await page.getByRole("columnheader", { name: "Started" }).elements()).toHaveLength(0);
});

// The standing rule of 29 Sep: no count beside the rows it counts. The filter
// reads its choice alone; its menu entries keep theirs, not being drawn yet.
test("the Drones filter names its choice without a number", async () => {
  mount("arc/executing-sequential", SLICES);
  await page.getByRole("tab", { name: /^Drones/ }).last().click();
  const trigger = page.getByRole("region", { name: "Drones on this Job" }).last().getByRole("button").first();
  await expect.element(trigger).toHaveTextContent(/^All$/);
});

// The owner's note of 29 Sep: *a button that lets me kill the drone from here
// instead of on the job header* — and the header's kill becomes the Job's.

test("a running Drone's sheet offers its kill, and the header kills the Job", async () => {
  // Both kills are named for their hold, which reduced motion does not offer.
  await motion();
  await droneOnT5();
  const sheet = page.getByRole("dialog", { name: "Drone on T5" }).last();
  await expect.element(sheet.getByRole("button", { name: "Hold to kill drone" })).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Hold to kill job" }).last()).toBeInTheDocument();
});

test("a Drone that has stopped offers no kill, and neither does the header", async () => {
  // Under reduced motion no kill is named for a hold, so the absence would prove nothing.
  await motion();
  mount("arc/executing-sequential", SLICES);
  await page.getByRole("tab", { name: /^Drones/ }).last().click();
  await page.getByRole("button", { name: "Drone on T1" }).last().click();
  await expect.element(page.getByRole("dialog", { name: "Drone on T1" }).last()).toBeVisible();
  expect(await page.getByRole("button", { name: "Hold to kill drone" }).elements()).toHaveLength(0);
});

test("pressing the step in a Drone's sheet lands on Workflow with that step's panel open", async () => {
  await droneOnT5();
  await page.getByRole("dialog", { name: "Drone on T5" }).last().getByRole("button", { name: "Implement", exact: true }).click();
  await expect.element(page.getByRole("tab", { name: /^Workflow/, selected: true }).last()).toBeVisible();
  await expect.element(page.getByRole("dialog", { name: "Implement" }).last()).toBeVisible();
});

// The owner's note of 29 Sep: *a drone will now be running against a step and a
// task. This is already represented in the header.* The transcript draws no line
// naming the step again.
test("a Drone's transcript draws no step boundary under the head that names it", async () => {
  await droneOnT5();
  const sheet = page.getByRole("dialog", { name: "Drone on T5" }).last();
  await expect.element(sheet.getByRole("group", { name: "Drone" }).first()).toBeVisible();
  expect(await sheet.getByText("step", { exact: true }).elements()).toHaveLength(0);
});

test("pressing the task in a Drone's sheet lands on Plan with that task's sheet open", async () => {
  await droneOnT5();
  await page.getByRole("dialog", { name: "Drone on T5" }).last().getByRole("button", { name: "T5", exact: true }).click();
  await expect.element(page.getByRole("tab", { name: /^Plan/, selected: true }).last()).toBeVisible();
  const sheet = page.getByRole("dialog").last();
  await expect.element(sheet).toHaveTextContent("T5");
  await expect.element(sheet).toHaveTextContent("Draw what is running, in four lists");
  await expect.element(sheet).toHaveTextContent("Drone on T5");
});

// #1666: with two tasks running at once, a Drone's sheet stops or tells that
// Drone alone — the act carries its id, never the Job's.
test("a Drone's own sheet sends its message and its kill to that Drone, by id", async () => {
  mount("arc/executing-at-once", SLICES);
  const redirect = vi.spyOn(window.armada, "redirectDrone");
  const kill = vi.spyOn(window.armada, "killDrone");
  await page.getByRole("tab", { name: /^Drones/ }).last().click();
  await page.getByRole("button", { name: "Drone on T6" }).last().click();
  const sheet = page.getByRole("dialog", { name: "Drone on T6" }).last();
  await expect.element(sheet).toBeVisible();
  await Promise.all(sheet.element().getAnimations().map((one) => one.finished));

  await sheet.getByRole("textbox").click();
  await userEvent.keyboard("Stop at the last row");
  await sheet.getByRole("button", { name: "Send" }).click();
  await expect.poll(() => redirect.mock.calls.at(-1)?.[2]).toBe("01M2D5HKQP001DRONE0000T6");

  // Reduced motion offers the kill as a press and a confirmation.
  await sheet.getByRole("button", { name: "Kill drone" }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Kill drone" }).last().click();
  await expect.poll(() => kill.mock.calls.at(-1)?.[1]).toBe("01M2D5HKQP001DRONE0000T6");
});
