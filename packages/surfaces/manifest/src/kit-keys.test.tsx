// Kit's keyboard, through `App`: the tiles move like the cockpit's glass, a tile's controls are
// stepped into and back out of, and a field hands the keys back. `kit-keys.ts`.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { allowlistRead, KIT_SERVERS, manifesting, mount, unmountAfterEach } from "@armada/desktop/mock";

unmountAfterEach();

const SLICES = { slices: ["core", "manifest"] } as const;

async function kit(): Promise<void> {
  mount(
    manifesting({ kitServers: KIT_SERVERS, kitInventory: allowlistRead([{ name: "grep -n", source: "always allow" }]) }),
    SLICES,
  );
  await page.getByRole("button", { name: "Kit", exact: true }).click();
  await expect.element(page.getByRole("region", { name: "Skills" })).toBeVisible();
}

const region = (name: string) => page.getByRole("region", { name, exact: true });

/**
 * Bare arrows move across the glass by where the tiles are drawn, and `j`/`k` through them in the
 * order they are drawn — the cockpit's own two motions.
 */
test("the arrows move across Kit's tiles as they are drawn, and j and k step through them", async () => {
  await kit();
  (region("Skills").element() as HTMLElement).focus();

  await userEvent.keyboard("{ArrowRight}");
  await expect.element(region("Plugins")).toHaveFocus();
  await userEvent.keyboard("{ArrowDown}");
  await expect.element(region("Commands")).toHaveFocus();
  await userEvent.keyboard("{ArrowLeft}");
  await expect.element(region("Sub agents")).toHaveFocus();
  await userEvent.keyboard("{ArrowUp}");
  await expect.element(region("Skills")).toHaveFocus();

  await userEvent.keyboard("j");
  await expect.element(region("Plugins")).toHaveFocus();
  await userEvent.keyboard("k");
  await userEvent.keyboard("k");
  // Clamped at the first tile rather than wrapped.
  await expect.element(region("Skills")).toHaveFocus();
});

/** The servers are tiles on the same keys: Down from the last row of kinds reaches them. */
test("the keys reach the servers from the kinds above them", async () => {
  await kit();
  (region("Models").element() as HTMLElement).focus();

  await userEvent.keyboard("{ArrowDown}");
  await expect.element(page.getByRole("row", { name: /tracker/ })).toHaveFocus();
  await userEvent.keyboard("{ArrowDown}");
  await expect.element(page.getByRole("row", { name: /nexus/ })).toHaveFocus();
});

/** Enter steps into a tile's first control and Esc steps back out to the tile. */
test("Enter steps into a tile and Esc steps back out", async () => {
  await kit();
  (region("Allowlist").element() as HTMLElement).focus();

  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByRole("button", { name: "Remove grep -n" })).toHaveFocus();
  await userEvent.keyboard("{Escape}");
  await expect.element(region("Allowlist")).toHaveFocus();

  (page.getByRole("row", { name: /nexus/ }).element() as HTMLElement).focus();
  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByRole("combobox", { name: "In Kit: nexus" })).toHaveFocus();
  await userEvent.keyboard("{Escape}");
  await expect.element(page.getByRole("row", { name: /nexus/ })).toHaveFocus();
});

/**
 * A field keeps what is typed into it and lets the keys go: Esc always, Down only with nothing
 * typed, where Down is the field's own.
 */
test("a field hands the keys back to the tiles on Esc, and on Down when it is empty", async () => {
  await kit();
  const name = page.getByRole("textbox", { name: "Name" });

  await name.click();
  await userEvent.keyboard("{ArrowDown}");
  // Nothing held yet, so the tile under the field.
  await expect.element(page.getByRole("row", { name: /tracker/ })).toHaveFocus();

  await name.click();
  await userEvent.keyboard("abc{ArrowDown}");
  await expect.element(name).toHaveFocus();
  await userEvent.keyboard("{Escape}");
  await expect.element(page.getByRole("row", { name: /tracker/ })).toHaveFocus();
  await expect.element(name).toHaveValue("abc");
});
