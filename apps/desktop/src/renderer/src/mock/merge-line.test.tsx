// The merge line draws only where there is a line: the owner, 2 Oct 2026. Where Fleet serves none,
// or nobody is in line, neither the rail row nor Overview's panel may draw.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const row = () =>
  page.getByRole("navigation", { name: "Work" }).getByRole("button", { name: "Merge line", exact: true });
const panel = () => page.getByRole("region", { name: "Merge line" });

test("with no line, neither the rail row nor the Overview panel draws", async () => {
  mount("every-state");
  await onScreen();

  await expect.element(page.getByRole("button", { name: "Overview", exact: true })).toBeVisible();
  expect(row().query()).toBeNull();
  expect(panel().query()).toBeNull();
});

test("with a line, the rail row and the Overview panel both draw it", async () => {
  mount("merge-line");
  await onScreen();

  await expect.element(row()).toBeVisible();
  await expect.element(panel().getByRole("list", { name: "Batch" })).toBeVisible();
});
