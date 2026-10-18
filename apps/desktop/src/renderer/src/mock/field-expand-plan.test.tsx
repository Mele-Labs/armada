// The Field of an open Job carries its plan along its foot, and opens large over the stage: from the control in its band and from its key,
// `deck_field`'s `e`. Esc is Back, with focus back on the control, and only one canvas is live while it is large.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const field = () => page.getByRole("region", { name: "Field", exact: true }).first();
const large = () => page.getByRole("group", { name: "Expanded Field" });
const canvases = () => document.querySelectorAll("canvas").length;

async function open() {
  mount("field-expand-plan");
  await onScreen();
  await page.getByRole("region", { name: "Jobs", exact: true }).getByRole("button", { name: "Rotate the audit log" }).click();
  await expect.element(field().getByRole("group", { name: "Plan" })).toBeVisible();
}

test("the Field carries the plan as a segment for each task, in the marks of the map card, each group named", async () => {
  await open();
  const plan = field().getByRole("group", { name: "Plan" });
  const segments = [...document.querySelectorAll('section[aria-label="Field"] .sv-fieldplan li')];
  expect(segments.map((one) => one.getAttribute("data-piece"))).toEqual(["solid", "solid", "solid", "solid", "scaffold", "scaffold", "ghost", "ghost", "ghost", "ghost", "hollow"]);
  expect(segments.map((one) => one.getAttribute("aria-label"))[4]).toBe("Mirror it in the TypeScript types, Working");
  await expect.element(plan.getByRole("group", { name: "Show the rotation" })).toBeVisible();
  await expect.element(plan.getByRole("group", { name: "Prune old files" })).toBeVisible();
});

test("the control opens the Field large, names its key from the registry, and Esc puts it back with focus on the control", async () => {
  await open();
  const control = field().getByRole("button", { name: "Expand", exact: true });
  await expect.element(control).toHaveAccessibleDescription("Expand e");
  await control.click();
  await expect.element(large()).toBeVisible();
  // Each group is named under its stretch of the plan, and the panel's own map has given up its canvas.
  await expect.element(large().getByText("Rotate at midnight")).toBeVisible();
  await expect.poll(canvases).toBe(1);
  expect(document.querySelectorAll('section[aria-label="Field"]').length).toBe(2);
  await userEvent.keyboard("{Escape}");
  await expect.element(large()).not.toBeInTheDocument();
  await expect.poll(() => document.activeElement?.getAttribute("aria-label")).toBe("Expand");
  await expect.poll(canvases).toBe(1);
});

test("e opens the Field large and e again puts it back, unless a box holds the keys", async () => {
  await open();
  (field().getByRole("button", { name: "Expand", exact: true }).element() as HTMLElement).focus();
  await userEvent.keyboard("e");
  await expect.element(large()).toBeVisible();
  await userEvent.keyboard("e");
  await expect.element(large()).not.toBeInTheDocument();
  // In the composer an e is a letter.
  const box = page.getByRole("region", { name: /^Job: / }).getByRole("textbox").first();
  await box.click();
  await userEvent.keyboard("e");
  await expect.element(large()).not.toBeInTheDocument();
});
