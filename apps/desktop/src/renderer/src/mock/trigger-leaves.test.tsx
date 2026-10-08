// Triggers as leaves on the Job's canvas: each stands off the step it fired at, inside that step's
// lane, and the Overview has no list of them under the canvas.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const RUN = () => page.getByRole("region", { name: "This Job's run" });

/** Whether every node a mark stands in lies wholly inside some lane's frame. */
function eachInALane(mark: string): boolean[] {
  const run = document.querySelector('section[aria-label="This Job\'s run"]')!;
  const frames = [...run.querySelectorAll(".armada-approval-canvas__frame")].map((one) => one.getBoundingClientRect());
  return [...run.querySelectorAll(`[role="img"][aria-label="${mark}"]`)].map((one) => {
    const leaf = one.closest(".react-flow__node")!.getBoundingClientRect();
    return frames.some((zone) => leaf.left >= zone.left && leaf.right <= zone.right && leaf.top >= zone.top && leaf.bottom <= zone.bottom);
  });
}

test("a repair's branch stands inside the lane of the step it grew from, and the lane holds it", async () => {
  mount("real/job-2-leaves");
  await onScreen();
  await expect.element(RUN().getByRole("img", { name: "Repair branch" })).toBeVisible();
  expect(eachInALane("Repair branch")).toEqual([true]);
});

test("a Drone's branch off a step in the Work lane and one off the pull request each stand inside their lane", async () => {
  mount("real/job-2-leaves");
  await onScreen();
  await expect.element(RUN().getByRole("img", { name: "Drone branch" }).first()).toBeVisible();
  expect(eachInALane("Drone branch")).toEqual([true, true]);
});

test("the Overview carries no list of Triggers under the canvas", async () => {
  mount("real/job-2-leaves");
  await onScreen();
  await expect.element(RUN().getByRole("img", { name: "Repair branch" })).toBeVisible();
  await expect.element(page.getByRole("region", { name: "Triggers" })).not.toBeInTheDocument();
  await expect.element(page.getByRole("list", { name: "Triggers" })).not.toBeInTheDocument();
});
