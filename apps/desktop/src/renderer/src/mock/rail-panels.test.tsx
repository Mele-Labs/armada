// The rail as two panels — the owner, 28 Sep 2026:
//
// *"We should split this into two panels. Not sure the name of the first one,
// but its about jobs and studios. It should contain Overview, Studios, Cleanup.
// The second one should contain Kit, Settings, Guides."*
//
// Through `App`, because the claim is about what the column draws and in what
// order, which a story sized by a `div` cannot answer.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

/** One panel's rows, in the order the column draws them. */
function rowsOf(panel: string): string[] {
  const nav = document.querySelector(`nav[aria-label="${panel}"]`);
  if (nav === null) throw new Error(`no rail panel named ${panel}`);
  return [...nav.querySelectorAll(".armada-sidebar__label")].map((one) => one.textContent ?? "");
}

test("the rail draws two panels, named Work and Machine", async () => {
  mount("every-state");
  await onScreen();

  await expect.element(page.getByRole("navigation", { name: "Work" })).toBeVisible();
  await expect.element(page.getByRole("navigation", { name: "Machine" })).toBeVisible();
});

test("Machine holds Kit, Settings and Guides, in that order", async () => {
  mount("every-state");
  await onScreen();

  expect(rowsOf("Machine")).toEqual(["Kit", "Settings", "Guides"]);
});

test("Work holds the jobs-and-studios rows, Overview first", async () => {
  mount("every-state");
  await onScreen();

  // The Job Board is still one of them: its page exists until #1594 folds it
  // into Overview, and a rail row for a page that is still reachable is not a
  // row to drop early.
  expect(rowsOf("Work")).toEqual(["Overview", "Job Board", "Studios", "Cleanup"]);
});

test("Manifest has left the rail, and the control beside the picker still opens it", async () => {
  mount("manifest");
  await onScreen();

  // Neither panel carries it — #1595 folded it into the title row's picker.
  expect([...rowsOf("Work"), ...rowsOf("Machine")]).not.toContain("Manifest");
  await page.getByRole("button", { name: "Open the Manifest", exact: true }).click();
  await expect.element(page.getByRole("tab", { name: /armada\.yml/ })).toBeVisible();
});

test("one collapse control for the column, in the first panel's head", async () => {
  mount("every-state");
  await onScreen();

  const collapse = page.getByRole("button", { name: "Collapse the left column" });
  await expect.element(collapse).toBeVisible();
  expect(document.querySelectorAll(".armada-sidebar__collapse")).toHaveLength(1);
  expect(collapse.element().closest("nav")?.getAttribute("aria-label")).toBe("Work");
});
