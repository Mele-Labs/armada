// A kept brief, read in Pulse's log panel, through `App`.
//
// The owner, 2 Oct 2026: a Judge's or a gaming check's brief opens in the log
// panel like a transcript does, off what Fleet serves (`get_brief`, 21.11),
// and the row's own Open still leaves the app.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { escalatedEvidenceSuspect } from "./fixtures/build/index";

import { mount, onJob, unmountAfterEach } from "@armada/desktop/mock";

unmountAfterEach();

// Core and Jobs only: the surface's own members, and the scenario answers the rest.
const SLICES = { slices: ["core", "jobs"] } as const;

async function onPulse(): Promise<void> {
  await page.getByRole("tab", { name: "Pulse" }).click();
  await expect.element(page.getByRole("tabpanel", { name: "Pulse" })).toBeVisible();
}

test("pressing a Judge brief's row reads the brief in the panel, ending on its question", async () => {
  mount(onJob(escalatedEvidenceSuspect()), SLICES);
  await onPulse();

  await page.getByRole("button", { name: "Judge brief, regression_verify · c2" }).click();

  const panel = page.getByRole("dialog", { name: "Judge brief" });
  await expect.element(panel).toBeVisible();
  await expect.element(panel.getByText("regression_verify · c2")).toBeVisible();
  await expect
    .element(panel.getByText(/still assert that selectVisibleColumns memoises/))
    .toBeVisible();
  // Numbered as the file is, so a Judge's citation lands on the line it names.
  await expect.element(panel.getByText(/regression_verify\.1\.c2\.md/)).toBeVisible();
});

test("a gaming check's brief opens the same way, and its row keeps the Open that leaves the app", async () => {
  mount(onJob(escalatedEvidenceSuspect()), SLICES);
  await onPulse();

  const row = page.getByRole("listitem").filter({ hasText: "regression_verify · gaming check" });
  await expect.element(row.getByRole("button", { name: "Open" })).toBeVisible();

  await page.getByRole("button", { name: "Judge brief, regression_verify · gaming check" }).click();

  const panel = page.getByRole("dialog", { name: "Judge brief" });
  await expect.element(panel.getByText(/asserts less than it did/)).toBeVisible();
});

test("a brief opens wrapped, and the panel's toggle unwraps it", async () => {
  mount(onJob(escalatedEvidenceSuspect()), SLICES);
  await onPulse();

  await page.getByRole("button", { name: "Judge brief, regression_verify · gaming check" }).click();

  const panel = page.getByRole("dialog", { name: "Judge brief" });
  const toggle = panel.getByRole("button", { name: "Wrap lines" });
  await expect.element(panel.getByText(/asserts less than it did/)).toBeVisible();
  await expect.element(toggle).toHaveAttribute("aria-pressed", "true");
  expect(document.querySelector(".armada-console")?.getAttribute("data-wrap")).toBe("true");

  await toggle.click();

  await expect.element(toggle).toHaveAttribute("aria-pressed", "false");
  expect(document.querySelector(".armada-console")?.hasAttribute("data-wrap")).toBe(false);
});

test("a transcript's panel draws no wrap toggle, having no console lines to wrap", async () => {
  mount(onJob(escalatedEvidenceSuspect()), SLICES);
  await onPulse();

  await page.getByRole("button", { name: /^Drone transcript/ }).click();

  const panel = page.getByRole("dialog", { name: "Drone transcript" });
  await expect.element(panel).toBeVisible();
  expect(panel.getByRole("button", { name: "Wrap lines" }).elements()).toHaveLength(0);
});

test("a brief Fleet no longer holds says so inside the panel", async () => {
  mount(onJob({ ...escalatedEvidenceSuspect(), briefs: {} }), SLICES);
  await onPulse();

  await page.getByRole("button", { name: "Judge brief, regression_verify · c1" }).click();

  const panel = page.getByRole("dialog", { name: "Judge brief" });
  await expect.element(panel.getByText(/no longer in the record|did not answer/)).toBeVisible();
});
