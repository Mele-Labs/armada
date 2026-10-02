// The merge line draws wherever Fleet serves one: the owner, 2 Oct 2026. Where Fleet serves none,
// neither the rail row nor Overview's panel may draw. Where it serves one, the panel draws even
// with nobody in line, and on All each repository with a line has its own, named.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const row = () =>
  page.getByRole("navigation", { name: "Work" }).getByRole("button", { name: "Merge line", exact: true });
const panel = (name = "Merge line") => page.getByRole("region", { name, exact: true });

test("with no line, neither the rail row nor the Overview panel draws", async () => {
  mount("every-state");
  await onScreen();

  await expect.element(page.getByRole("button", { name: "Overview", exact: true })).toBeVisible();
  expect(row().query()).toBeNull();
  expect(page.getByRole("region", { name: "Merge line" }).query()).toBeNull();
});

test("on All, the rail row and one named panel for each repository with a line", async () => {
  mount("merge-line");
  await onScreen();

  await expect.element(row()).toBeVisible();
  await expect.element(panel("Merge line, armada").getByRole("list", { name: "Batch" })).toBeVisible();
  await expect.element(panel("Merge line, armada").getByRole("list", { name: "Sent back" })).toBeVisible();
  await expect.element(panel("Merge line, notes").getByRole("list", { name: "Recently landed" })).toBeVisible();
  expect(panel("Merge line, notes").getByRole("list", { name: "Sent back" }).query()).toBeNull();
  await expect.element(panel("Merge line, scratch").getByRole("img", { name: "Empty" })).toBeVisible();
  // The picture, and not one word under it.
  expect(panel("Merge line, scratch").element().querySelector(".armada-merge-line__empty")?.textContent).toBe("");
});

test("the rail surface draws the same panels", async () => {
  mount("merge-line");
  await onScreen();

  await row().click();
  await expect.element(panel("Merge line, notes").getByRole("list", { name: "Recently landed" })).toBeVisible();
  await expect.element(panel("Merge line, scratch").getByRole("img", { name: "Empty" })).toBeVisible();
});
