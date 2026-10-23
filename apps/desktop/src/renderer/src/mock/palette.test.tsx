// The command palette, through `App`: where it opens.
// `docs/contracts/design-system.md`, Command palette.

import { expect, test } from "vitest";
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

// **The owner, 15 Oct 2026**: even though the search is not centred in the title bar, the palette still opens in the
// middle of the screen, a little to the north. Its field stays fixed at 20vh, the top-anchored rule of `CommandPalette.css`.
test("the palette opens centred across the window, not on the search field", async () => {
  mount("default");
  await onScreen();
  const list = await palette();
  const box = list.element().getBoundingClientRect();
  expect(document.querySelector(".armada-palette-layer")?.hasAttribute("data-anchored")).toBe(false);
  expect(box.left + box.width / 2).toBeCloseTo(window.innerWidth / 2, 0);
  expect(box.top).toBeCloseTo(window.innerHeight * 0.2, 0);
});

// The Cockpit's list was called Sessions until 17 Oct 2026: typing the old word still finds what is called Jobs now.
test("session is still a search word for the renamed entries", async () => {
  mount("default");
  await onScreen();
  const list = await palette();
  await userEvent.type(list.getByRole("combobox"), "session");
  await expect.element(list.getByRole("option", { name: /^Jobs/ })).toBeVisible();
  await expect.element(list.getByRole("option", { name: /^Dispatch/ })).toBeVisible();
});

// A pasted path is a first row: ↵ opens the file in the panel, and a path that is no readable file says so.
test("a pasted path is offered first and opens in the file panel", async () => {
  mount("file-panel");
  await onScreen();
  const list = await palette();
  await userEvent.type(list.getByRole("combobox"), '  "/Users/user/Development/armada/crates/store/src/clock.rs" ');
  const row = list.getByRole("option").first();
  await expect.element(row).toHaveTextContent("Open /Users/user/Development/armada/crates/store/src/clock.rs");
  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByRole("dialog", { name: "clock.rs", exact: true })).toBeVisible();
});

test("a pasted path that is no file stays in the palette and says so", async () => {
  mount("file-panel");
  await onScreen();
  const list = await palette();
  await userEvent.type(list.getByRole("combobox"), "/nowhere/missing.txt");
  // The pretend disk reads a ledger's file as its own name, so only the opener's answer refuses: none is held for this path.
  await userEvent.keyboard("{Enter}");
  await expect.element(list.getByRole("alert")).toHaveTextContent("It is not a readable file here");
});
