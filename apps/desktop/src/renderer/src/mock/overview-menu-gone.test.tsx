// Overview's menu, taken away — the owner's note of 1 Oct 2026: *"I hate this
// menu on the overview page. Two of the items are already in the left panel …
// Refresh could just be a keyboard shortcut … That means we can then just get
// rid of this dropdown."*
//
// Each of its six items went somewhere, and each is pinned here: Refresh to
// `⇧⌘R` and the palette, Reported to the palette alone, the two bulk sweeps to
// Cleanup's head and the palette. Held disk and Settings were already rail rows.

import { describe, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

/** ⌘K, and the palette's list once it is up. */
async function palette() {
  await userEvent.keyboard("{Meta>}k{/Meta}");
  const list = page.getByRole("dialog", { name: "Command palette" });
  await expect.element(list).toBeVisible();
  return list;
}

/** Cleanup, by its rail row. */
async function cleanup(): Promise<void> {
  await page.getByRole("navigation", { name: "Work" }).getByText("Worktree Slots", { exact: true }).click();
  await expect.element(page.getByRole("button", { name: "Back to the list" })).toBeVisible();
}

// One name, so the whole file runs as one Check: `armada check desktop_test "overview menu gone"`.
describe("overview menu gone", () => {
  test("Overview carries no menu at all", async () => {
    mount("every-state");
    await onScreen();

    // Overview, drawn: its Done section's head.
    await expect.element(page.getByRole("button", { name: "Expand Done" })).toBeVisible();
    expect(page.getByRole("button", { name: "Everything else", exact: true }).query()).toBeNull();
    expect(page.getByRole("button", { name: /^Clear \d+ finished/ }).query()).toBeNull();
  });

  test("⇧⌘R refreshes, from Overview with nothing on screen to press", async () => {
    const app = mount("every-state");
    await onScreen();
    const state = vi.spyOn(app.api, "state");

    await userEvent.keyboard("{Meta>}{Shift>}r{/Shift}{/Meta}");
    await expect.poll(() => state.mock.calls.length).toBe(1);
  });

  test("Clear and Delete records are on Cleanup's head, and each still asks first", async () => {
    mount("every-state");
    await onScreen();
    await cleanup();

    await page.getByRole("button", { name: /^Clear \d+ finished/ }).click();
    const clear = page.getByRole("dialog");
    await expect.element(clear.getByRole("heading", { name: /^Clear \d+ finished jobs?\?$/ })).toBeVisible();
    await clear.getByRole("button", { name: "Cancel" }).click();
    await expect.element(clear).not.toBeInTheDocument();

    await page.getByRole("button", { name: /^Delete \d+ jobs?'s? records$/ }).click();
    const forget = page.getByRole("dialog");
    await expect.element(forget.getByRole("heading", { name: /^Delete \d+ finished .* records\?$/ })).toBeVisible();
    await expect.element(forget.getByRole("button", { name: "Delete", exact: true })).toBeVisible();
  });

  test("the palette lists Refresh with its key, Reported, and both sweeps", async () => {
    mount("every-state");
    await onScreen();
    const list = await palette();

    await expect.element(list.getByRole("option", { name: /^Refresh.*⇧⌘R/ })).toBeInTheDocument();
    await expect.element(list.getByRole("option", { name: "Reported" })).toBeInTheDocument();
    await expect.element(list.getByRole("option", { name: /^Clear \d+ finished/ })).toBeInTheDocument();
    await expect.element(list.getByRole("option", { name: /^Delete \d+ .* records$/ })).toBeInTheDocument();
  });

  test("Reported, chosen from the palette, opens the reports", async () => {
    mount("every-state");
    await onScreen();
    const list = await palette();

    await list.getByRole("option", { name: "Reported" }).click();
    // The mock serves no `/reports`, so what draws is the refusal and the way out.
    await expect.element(page.getByRole("button", { name: "Back to the list" })).toBeVisible();
  });

  test("Clear, chosen from the palette, asks before it sweeps", async () => {
    mount("every-state");
    await onScreen();
    const list = await palette();

    await list.getByRole("option", { name: /^Clear \d+ finished/ }).click();
    const asking = page.getByRole("dialog", { name: /^Clear \d+ finished/ });
    await expect.element(asking).toBeVisible();
    await expect.element(asking.getByRole("button", { name: "Clear", exact: true })).toBeVisible();
  });
});
