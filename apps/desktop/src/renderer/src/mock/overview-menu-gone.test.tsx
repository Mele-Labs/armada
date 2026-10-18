// Overview's menu, taken away — the owner's note of 1 Oct 2026: *"I hate this
// menu on the overview page. Two of the items are already in the left panel …
// Refresh could just be a keyboard shortcut … That means we can then just get
// rid of this dropdown."*
//
// Its items went somewhere, and each is pinned here: Refresh to `⇧⌘R` and the
// palette, Reported to the palette alone. The two bulk sweeps went with the Jobs
// they swept. Held disk is reached by name from the palette, and Settings from the System strip.

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

// One name, so the whole file runs as one Check: `armada check desktop_test "overview menu gone"`.
describe("overview menu gone", () => {
  test("Overview carries no menu at all", async () => {
    mount("default");
    await onScreen();

    // The Cockpit, drawn: its list.
    await expect.element(page.getByRole("region", { name: "Jobs", exact: true })).toBeVisible();
    expect(page.getByRole("button", { name: "Everything else", exact: true }).query()).toBeNull();
    expect(page.getByRole("button", { name: /^Clear \d+ finished/ }).query()).toBeNull();
  });

  test("⇧⌘R refreshes, from Overview with nothing on screen to press", async () => {
    const app = mount("default");
    await onScreen();
    const state = vi.spyOn(app.api, "state");

    await userEvent.keyboard("{Meta>}{Shift>}r{/Shift}{/Meta}");
    await expect.poll(() => state.mock.calls.length).toBe(1);
  });

  test("the palette lists Refresh with its key, and Reported", async () => {
    mount("default");
    await onScreen();
    const list = await palette();

    await expect.element(list.getByRole("option", { name: /^Refresh.*⇧⌘R/ })).toBeInTheDocument();
    await expect.element(list.getByRole("option", { name: "Reported" })).toBeInTheDocument();
    expect(list.getByRole("option", { name: /^(Clear|Delete) \d+/ }).query()).toBeNull();
  });

  test("Reported, chosen from the palette, opens the reports", async () => {
    mount("default");
    await onScreen();
    const list = await palette();

    await list.getByRole("option", { name: "Reported" }).click();
    // The mock serves no `/reports`, so what draws is the refusal and the way out.
    await expect.element(page.getByRole("button", { name: "Back to the list" })).toBeVisible();
  });
});
