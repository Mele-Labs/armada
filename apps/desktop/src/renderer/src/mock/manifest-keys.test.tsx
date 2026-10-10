// The Manifest surface walked by keyboard alone, through `App`: each grid of tiles is one Tab stop
// and moves like the cockpit's glass (`tile-grid.ts`), Enter goes in a level and Esc comes back
// out, and the file view's editor keeps its keys.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

/** A Check or Command's tile, by the name it opens with — the rest of its name is its line and facts. */
const tile = (name: string) => page.getByRole("button", { name: new RegExp(`^${name} `) });

async function opened(): Promise<void> {
  mount("manifest");
  await onScreen();
  await userEvent.click(page.getByRole("button", { name: "Open the Manifest", exact: true }));
  await expect.element(tile("bootstrap")).toBeVisible();
  // From nowhere: the press that opened the surface leaves focus on the picker, whose keys are its own.
  (document.activeElement as HTMLElement | null)?.blur();
}

test("Manifest keys: the run grid is one Tab stop, and Run is the next one past it", async () => {
  await opened();

  await userEvent.keyboard("j");
  await expect.element(tile("bootstrap")).toHaveFocus();
  // Tab leaves the grid rather than walking it: the next stop is the readout's, never a second tile.
  await userEvent.keyboard("{Tab}");
  await expect.element(page.getByRole("button", { name: "Dismiss", exact: true })).toHaveFocus();
  await userEvent.keyboard("{Shift>}{Tab}{/Shift}");
  await expect.element(tile("bootstrap")).toHaveFocus();

  await userEvent.keyboard("{ArrowDown}{Enter}");
  await expect.element(tile("build")).toHaveAttribute("aria-current", "true");
  await userEvent.keyboard("{Tab}");
  await expect.element(page.getByRole("button", { name: "Run", exact: true })).toHaveFocus();
});

test("Manifest keys: j lands on the tiles, arrows move across the grid, and Enter or o picks one", async () => {
  await opened();

  await userEvent.keyboard("j");
  await expect.element(tile("bootstrap")).toHaveFocus();
  await userEvent.keyboard("{ArrowRight}");
  await expect.element(tile("browsers")).toHaveFocus();
  // Down keeps to the column: from Setup's first tile to Checks' first, not the next in order.
  await userEvent.keyboard("{ArrowLeft}{ArrowDown}");
  await expect.element(tile("build")).toHaveFocus();
  await userEvent.keyboard("jj");
  await expect.element(tile("typecheck")).toHaveFocus();
  await userEvent.keyboard("k");
  await expect.element(tile("test")).toHaveFocus();

  await userEvent.keyboard("{Enter}");
  await expect.element(tile("test")).toHaveAttribute("aria-current", "true");
  await expect.element(page.getByRole("button", { name: "Run", exact: true })).toBeVisible();
  await userEvent.keyboard("jo");
  await expect.element(tile("typecheck")).toHaveAttribute("aria-current", "true");
  await expect.element(tile("typecheck")).toHaveFocus();
});

test("Manifest keys: on the form, Esc leaves a field for its entry and its section, and Enter goes back in", async () => {
  await opened();
  await userEvent.click(page.getByRole("tab", { name: "Edit" }));

  const build = page.getByRole("group", { name: "build", exact: true });
  const command = build.getByRole("textbox", { name: "Command" });
  await userEvent.click(command);
  // A letter in a field is typing, never a move.
  await userEvent.keyboard("j");
  await expect.element(command).toHaveFocus();
  await expect.poll(() => (command.element() as HTMLInputElement).value).toContain("j");

  await userEvent.keyboard("{Escape}");
  await expect.element(build).toHaveFocus();
  await userEvent.keyboard("{ArrowRight}");
  await expect.element(page.getByRole("group", { name: "typecheck", exact: true })).toHaveFocus();
  await userEvent.keyboard("{Escape}");
  await expect.element(page.getByRole("region", { name: "Checks", exact: true })).toHaveFocus();
  await userEvent.keyboard("{ArrowDown}");
  await expect.element(page.getByRole("region", { name: "Commands", exact: true })).toHaveFocus();
  await userEvent.keyboard("{ArrowUp}{Enter}");
  await expect.element(build).toHaveFocus();
});

test("Manifest keys: the file view's editor keeps Esc and the arrows", async () => {
  await opened();
  await userEvent.click(page.getByRole("tab", { name: /armada\.yml/ }));

  const editor = page.getByRole("textbox", { name: /armada\.yml$/ });
  await userEvent.click(editor);
  await userEvent.keyboard("{Escape}{ArrowDown}j");
  await expect.element(editor).toHaveFocus();
  await expect.poll(() => (editor.element() as HTMLTextAreaElement).value).toContain("j");
});
