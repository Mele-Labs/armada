// A task's own Drone, read in Plan's task sheet: what it is doing now, its live
// log, its last edit, and a stop for it alone. `#1536`, through `App`.
//
// **Mock-fed, and nothing on real Fleet.** Fleet runs one Drone per Job until
// slices 1 and 5 of `docs/spikes/022`, so a Job shaped the way Fleet serves it
// opens the same sheet with none of these parts — never an empty one.

import { describe, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import { featureOnItsPlan } from "@armada/jobs/fake";
import { onJob } from "./scenario";
import { entered, mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/** A task's panel, opened from its row on the Plan tab's list. */
async function panelOf(id: string, title: string) {
  await page.getByRole("tab", { name: /^Plan/ }).last().click();
  await page.getByRole("tab", { name: "List" }).last().click();
  await page.getByRole("listitem", { name: `${id} ${title}` }).getByRole("button").click();
  const panel = page.getByRole("dialog", { name: title });
  await entered(panel);
  return panel;
}

const T5 = { id: "T5", title: "Draw what is running, in four lists" };

describe("a task with a Drone of its own", () => {
  test("its panel opens on what it is doing, its brief, its log and its last edit", async () => {
    mount("arc/executing-sequential");
    const panel = await panelOf(T5.id, T5.title);
    // "Now" says it, and so does its row in the task's Drones (2 Oct 2026).
    await expect.element(panel.getByText("14 turns", { exact: true }).first()).toBeVisible();
    await expect.element(panel.getByRole("list", { name: "Drones on this task" })).toHaveTextContent("14 turns");
    await expect.element(panel.getByText("Keep the four lists in this order", { exact: false })).toBeVisible();
    // The live log is the Drone's own tail, which the sheet already drew.
    await expect.element(panel.getByRole("group", { name: "Drone on T5" })).toHaveTextContent("Running.tsx");
    await expect
      .element(panel.getByRole("list", { name: "Last edit" }))
      .toHaveTextContent("packages/screens/src/Running.tsx");
  });

  test("a running task's stop asks, then ends the Job's Drone, as the mock stands in for one task's", async () => {
    const app = mount("arc/executing-sequential");
    const killDrone = vi.spyOn(app.api, "killDrone");
    const panel = await panelOf(T5.id, T5.title);
    // Reduced motion offers no hold, so the press asks.
    await panel.getByRole("button", { name: "Stop this task" }).click();
    const confirm = page.getByRole("dialog", { name: "Kill the drone on this job?" });
    await expect.element(confirm).toBeVisible();
    await confirm.getByRole("button", { name: "Kill drone" }).click();
    await expect.poll(() => killDrone.mock.calls.length).toBe(1);
  });

  test("a finished task shows what its agent spent, and offers no stop", async () => {
    mount("arc/executing-concurrent");
    const panel = await panelOf(T5.id, T5.title);
    await expect.element(panel.getByText("27 turns · ~$1.90", { exact: true })).toBeVisible();
    expect(panel.getByRole("button", { name: "Stop this task" }).query()).toBeNull();
  });
});

describe("a Job shaped the way Fleet serves it", () => {
  test("a working task's panel opens with none of the three", async () => {
    mount(onJob(featureOnItsPlan({ T1: "working" })));
    const panel = await panelOf("T1", "Remove guides 8 and 20 from the catalogue and retire their numbers");
    await expect.element(panel.getByText("Files", { exact: true })).toBeVisible();
    expect(panel.getByText("Now", { exact: true }).query()).toBeNull();
    expect(panel.getByText("Last edit", { exact: true }).query()).toBeNull();
    expect(panel.getByRole("button", { name: "Stop this task" }).query()).toBeNull();
    expect(panel.getByRole("button", { name: "Hold to stop this task" }).query()).toBeNull();
  });
});
