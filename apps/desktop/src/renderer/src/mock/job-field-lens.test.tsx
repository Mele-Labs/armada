// A Job's Field lens: the card follows the drone that started writing last, the file before becomes a tab, other writers lift a smaller
// file with a count, and a press in the strip pins the card and a second lets go. Over the `job-field-lens` scenario.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();

const field = () => page.getByRole("region", { name: "Field", exact: true }).first();
// Found anywhere: with the pointer or focus in the panel, the card is drawn over the app, outside it.
const lens = (file: string) => page.getByRole("group", { name: `Lens: ${file}`, exact: true });
const main = () => field().getByRole("group", { name: "Drones" }).getByRole("button", { name: /^Show the change on the Field: Writing FieldPanel\.tsx$/ });

test("the card follows the newest writer, the file before is a tab, and a strip press pins the card and a second lets go", async () => {
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

  await main().click();
  await expect.element(main()).toHaveAttribute("aria-pressed", "true");
  await expect.element(lens("FieldPanel.tsx")).toBeVisible();
  await expect.element(writing.getByRole("listitem", { name: /^areas\.rs / })).toBeInTheDocument();

  await main().click();
  await expect.element(main()).toHaveAttribute("aria-pressed", "false");
  await expect.element(lens("areas.rs")).toBeVisible();
});
