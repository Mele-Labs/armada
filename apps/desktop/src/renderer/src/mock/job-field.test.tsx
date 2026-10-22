// A Job's Field draws the other live Jobs of its repository beside its own drones, a press on one opens that Job, and its slider scrubs back through the Job's reads and
// writes and returns to live at its right end. Over the `job-field-whole-repo` scenario.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const field = () => page.getByRole("region", { name: "Field", exact: true }).first();
const slider = () => field().getByRole("slider", { name: "When in the Job" });
const replay = () => field().getByRole("status", { name: "Replay" });

async function open() {
  mount("job-field-whole-repo");
  await onScreen();
  await page.getByRole("region", { name: "Jobs", exact: true }).getByRole("button", { name: "Rotate the audit log" }).click();
  await expect.element(slider()).toBeVisible();
}

test("the other live Jobs of the repository are on the Field beside the Job's own drones", async () => {
  await open();
  const map = field().getByRole("listbox", { name: "Map" });
  await expect.element(map.getByRole("option", { name: /^Rotate the audit log, Job/ })).toBeInTheDocument();
  await expect.element(map.getByRole("option", { name: /^Pin the store clock, Job/ })).toBeInTheDocument();
  await expect.element(map.getByRole("option", { name: /^Retry the slot lease, Job/ })).toBeInTheDocument();
});

test("a press on another Job's faded drone opens that Job on the stage, and a press on this Job's own opens nothing", async () => {
  await open();
  const map = field().getByRole("listbox", { name: "Map" });
  (map.getByRole("option", { name: /^Rotate the audit log, Job/ }).element() as HTMLElement).click();
  await expect.element(page.getByRole("region", { name: "Job: Rotate the audit log" })).toBeVisible();
  (map.getByRole("option", { name: /^Pin the store clock, Job/ }).element() as HTMLElement).click();
  await expect.element(page.getByRole("region", { name: "Job: Pin the store clock" })).toBeVisible();
  await expect.element(page.getByRole("region", { name: "Job: Rotate the audit log" })).not.toBeInTheDocument();
});

test("the slider rests live, steps back a read or write at a time, and End or a release at its right end is live again", async () => {
  await open();
  await expect.element(slider()).toHaveAttribute("aria-valuetext", "Live");
  await expect.element(replay()).not.toBeInTheDocument();

  (slider().element() as HTMLElement).focus();
  await userEvent.keyboard("{ArrowLeft}");
  await expect.element(replay()).toBeVisible();
  await expect.element(slider()).not.toHaveAttribute("aria-valuetext", "Live");
  await userEvent.keyboard("{ArrowLeft}");
  await expect.element(replay()).toBeVisible();

  await userEvent.keyboard("{End}");
  await expect.element(replay()).not.toBeInTheDocument();
  await expect.element(slider()).toHaveAttribute("aria-valuetext", "Live");

  // Pressed back, then let go at the right end of the track.
  await userEvent.keyboard("{Home}");
  await expect.element(replay()).toBeVisible();
  const track = slider().element() as HTMLElement;
  const box = track.getBoundingClientRect();
  const at = { clientX: box.right - 1, clientY: box.top + box.height / 2, pointerId: 1, bubbles: true };
  track.dispatchEvent(new PointerEvent("pointerdown", at));
  track.dispatchEvent(new PointerEvent("pointerup", at));
  await expect.element(replay()).not.toBeInTheDocument();
  await expect.element(slider()).toHaveAttribute("aria-valuetext", "Live");
});
