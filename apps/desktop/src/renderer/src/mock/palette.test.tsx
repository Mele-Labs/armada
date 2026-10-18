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
