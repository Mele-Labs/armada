// The command palette, through `App`: what it leaves out, and what it still draws dimmed.
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

test("an act on one Job is left out, since Bridge has no page that reads a Job", async () => {
  mount("default");
  await onScreen();
  const list = await palette();
  for (const verb of ["Open", "Review", "Attest", "Redirect", "Kill", "Pilot"]) {
    expect(list.getByRole("option", { name: new RegExp(`^${verb}\\b`) }).query()).toBeNull();
  }
  expect(list.getByText("no job focused", { exact: true }).query()).toBeNull();
});

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
