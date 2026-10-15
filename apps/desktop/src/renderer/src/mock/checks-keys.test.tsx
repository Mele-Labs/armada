// The Checks page walked by keyboard alone, through `App`: its tiles are one Tab stop and move like
// the cockpit's glass (`tile-grid.ts`), and Enter or o opens the Check's panel.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const checks = () => page.getByRole("region", { name: "Checks", exact: true });
const tile = (name: string) => checks().getByRole("button", { name, exact: true });

async function opened(): Promise<void> {
  mount("job-checks");
  await onScreen();
  await userEvent.keyboard("{Meta>}k{/Meta}");
  await userEvent.keyboard("Checks{Enter}");
  await expect.element(tile("desktop_test")).toBeVisible();
}

test("Checks keys: the tiles are one Tab stop, entered once and left without visiting a second", async () => {
  await opened();
  (checks().getByRole("tab", { name: "All", exact: true }).element() as HTMLElement).focus();

  await userEvent.keyboard("{Tab}");
  await expect.element(tile("desktop_test")).toHaveFocus();
  // The next stop is past the glass, never the next tile.
  await userEvent.keyboard("{Tab}");
  expect(checks().element().contains(document.activeElement)).toBe(false);
  await userEvent.keyboard("{Shift>}{Tab}{/Shift}");
  await expect.element(tile("desktop_test")).toHaveFocus();
});

test("Checks keys: the arrows and j/k move between tiles, and Enter or o opens one", async () => {
  await opened();
  (document.activeElement as HTMLElement | null)?.blur();

  // From nowhere, j lands on the glass.
  await userEvent.keyboard("j");
  await expect.element(tile("desktop_test")).toHaveFocus();
  await userEvent.keyboard("{ArrowDown}");
  await expect.element(tile("hooks_test")).toHaveFocus();
  await userEvent.keyboard("jj");
  await expect.element(tile("ipc_test")).toHaveFocus();
  await userEvent.keyboard("{ArrowUp}");
  await expect.element(tile("test")).toHaveFocus();
  await userEvent.keyboard("k");
  await expect.element(tile("hooks_test")).toHaveFocus();

  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByRole("dialog", { name: "Check" })).toBeVisible();
  await expect.element(tile("hooks_test")).toHaveAttribute("aria-expanded", "true");

  (tile("ipc_test").element() as HTMLElement).focus();
  await userEvent.keyboard("o");
  await expect.element(tile("ipc_test")).toHaveAttribute("aria-expanded", "true");
  await expect.element(tile("hooks_test")).toHaveAttribute("aria-expanded", "false");
});
