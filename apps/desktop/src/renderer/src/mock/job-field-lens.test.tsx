// A Job's Field lens: the card follows the drone that started writing last, the file before becomes a tab, other writers lift a smaller
// file with a count, and picking a drone in the Drones menu pins the card while Newest writer lets go. Over the `job-field-lens` scenario.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();

const field = () => page.getByRole("region", { name: "Field", exact: true }).first();
// Found anywhere: with the pointer or focus in the panel, the card is drawn over the app, outside it.
const lens = (file: string) => page.getByRole("group", { name: `Lens: ${file}`, exact: true });
// The menu's trigger: it names the drone the card follows, and what that drone is doing.
const drones = () => field().getByRole("group", { name: "Drones" }).getByRole("button");
// Portalled: the menu is drawn on the body, outside the panel.
const menu = () => page.getByRole("menu");
const item = (name: string | RegExp) => menu().getByRole("menuitem", { name });
const JOB = /^Show the change on the Field/;

async function toSubagentsWriting() {
  mount("job-field-lens");
  await onScreen();
  await page.getByRole("region", { name: "Jobs", exact: true }).getByRole("button", { name: "Show the change on the Field" }).click();
  await expect.element(lens("layout.ts")).toBeVisible();
  timePasses();
  await expect.element(lens("FieldPanel.tsx")).toBeVisible();
  timePasses();
  await expect.element(lens("areas.rs")).toBeVisible();
}

test("the card follows the newest writer, the file before is a tab, and picking a drone pins the card and Newest writer lets go", async () => {
  mount("job-field-lens");
  await onScreen();
  await page.getByRole("region", { name: "Jobs", exact: true }).getByRole("button", { name: "Show the change on the Field" }).click();
  await expect.element(lens("layout.ts")).toBeVisible();

  timePasses();
  await expect.element(lens("FieldPanel.tsx")).toBeVisible();
  await expect.element(lens("FieldPanel.tsx")).toHaveTextContent("layout.ts");

  timePasses();
  // The subagents started together after the Job's drone: the card is on the last of them, the others lift with a count.
  await expect.element(lens("areas.rs")).toBeVisible();
  const writing = field().getByRole("list", { name: "Files being written" });
  await expect.element(writing.getByRole("listitem", { name: /^FieldPanel\.tsx \+\d+ −\d+$/ })).toBeInTheDocument();
  await expect.element(writing.getByRole("listitem", { name: /^wire\.ts \+\d+ −\d+$/ })).toBeInTheDocument();
  await expect.element(drones()).toHaveAccessibleName("Keep the stamp in the store: Writing areas.rs");

  await drones().click();
  // Newest writer first and chosen, then the Job's drone, then each subagent.
  const names = menu().getByRole("menuitem").elements().map((one) => one.textContent ?? "");
  expect(names[0]).toBe("Newest writer");
  expect(names[1]).toMatch(JOB);
  expect(names.slice(2).map((one) => one.replace(/Writing .*$/, ""))).toEqual(["Check the wire types", "Write the lens into the docs", "Keep the stamp in the store"]);
  await expect.element(item("Newest writer")).toHaveAttribute("aria-current", "true");

  await item(JOB).click();
  await expect.element(menu()).not.toBeInTheDocument();
  await expect.element(drones()).toHaveAccessibleName("Show the change on the Field: Writing FieldPanel.tsx");
  await expect.element(lens("FieldPanel.tsx")).toBeVisible();
  await expect.element(writing.getByRole("listitem", { name: /^areas\.rs / })).toBeInTheDocument();

  // Picked again, it stays pinned.
  await drones().click();
  await expect.element(item(JOB)).toHaveAttribute("aria-current", "true");
  await item(JOB).click();
  await expect.element(lens("FieldPanel.tsx")).toBeVisible();

  await drones().click();
  await item("Newest writer").click();
  await expect.element(drones()).toHaveAccessibleName("Keep the stamp in the store: Writing areas.rs");
  await expect.element(lens("areas.rs")).toBeVisible();
});

test("the Drones menu is driven from the keyboard: ↓ opens it, the arrows move, Enter picks, and Esc goes back to the trigger", async () => {
  await toSubagentsWriting();
  (drones().element() as HTMLElement).focus();

  await userEvent.keyboard("{ArrowDown}");
  await expect.element(item("Newest writer")).toHaveFocus();
  await userEvent.keyboard("{ArrowDown}");
  await expect.element(item(JOB)).toHaveFocus();
  await userEvent.keyboard("{Enter}");
  await expect.element(menu()).not.toBeInTheDocument();
  await expect.element(drones()).toHaveFocus();
  await expect.element(drones()).toHaveAccessibleName("Show the change on the Field: Writing FieldPanel.tsx");
  await expect.element(lens("FieldPanel.tsx")).toBeVisible();

  await userEvent.keyboard("{Enter}");
  await expect.element(item("Newest writer")).toHaveFocus();
  await userEvent.keyboard("{Escape}");
  await expect.element(menu()).not.toBeInTheDocument();
  await expect.element(drones()).toHaveFocus();
  // Esc shut the menu alone: focus is still in the Field, so its card stays floated over the app, outside the panel.
  await new Promise((done) => window.setTimeout(done, 500));
  await expect.element(lens("FieldPanel.tsx")).toBeVisible();
  expect(field().element().contains(lens("FieldPanel.tsx").element())).toBe(false);
});
