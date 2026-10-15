// Kit's keyboard, through `App`: one Tab stop for the tiles, which move like the cockpit's glass, a
// tile's controls stepped into and back out of, and a field handing the keys back. `tile-grid.ts`.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { allowlistRead, KIT_INVENTORY, KIT_SERVERS, manifesting, mount, toSystemPage, unmountAfterEach } from "@armada/desktop/mock";

unmountAfterEach();

const SLICES = { slices: ["core", "manifest"] } as const;

async function kit(): Promise<void> {
  mount(
    manifesting({ kitServers: KIT_SERVERS, kitInventory: allowlistRead([{ name: "grep -n", source: "always allow" }]) }),
    SLICES,
  );
  await toSystemPage("Kit");
  await expect.element(page.getByRole("region", { name: "Skills" })).toBeVisible();
}

const region = (name: string) => page.getByRole("region", { name, exact: true });

/**
 * **The grid is one Tab stop** — the owner's choice. Tab from before it lands on one tile and Tab
 * onward goes to the add form and out, never through the other tiles.
 */
test("Kit's tiles are one Tab stop", async () => {
  await kit();
  (page.getByRole("navigation", { name: "System" }).getByRole("tab", { name: "Kit" }).element() as HTMLElement).focus();

  await userEvent.tab();
  await expect.element(region("Skills")).toHaveFocus();
  await userEvent.tab();
  await expect.element(page.getByRole("textbox", { name: "Name" })).toHaveFocus();
  await userEvent.tab();
  await userEvent.tab();
  await userEvent.tab();
  await expect.element(page.getByRole("button", { name: "Add" })).toHaveFocus();
  // Past the add row and off the grid: no server row is a second stop.
  await userEvent.tab();
  expect(document.activeElement?.closest("[data-kit-tile]") ?? null).toBeNull();

  // The tile last held is the stop, and Shift+Tab from the form comes back to it.
  (region("Plugins").element() as HTMLElement).focus();
  await userEvent.tab();
  await expect.element(page.getByRole("textbox", { name: "Name" })).toHaveFocus();
  await userEvent.tab({ shift: true });
  await expect.element(region("Plugins")).toHaveFocus();
});

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
  await expect.element(region("MCP servers")).toHaveFocus();
  await userEvent.keyboard("{ArrowLeft}");
  await expect.element(region("Commands")).toHaveFocus();
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
  // Nothing held yet, so the grid's one stop: its first tile.
  await expect.element(region("Skills")).toHaveFocus();

  (page.getByRole("row", { name: /nexus/ }).element() as HTMLElement).focus();
  await name.click();
  await userEvent.keyboard("abc{ArrowDown}");
  await expect.element(name).toHaveFocus();
  await userEvent.keyboard("{Escape}");
  // The tile last held.
  await expect.element(page.getByRole("row", { name: /nexus/ })).toHaveFocus();
  await expect.element(name).toHaveValue("abc");
});

/**
 * **A long list scrolls inside its tile** and the keys reach it: Down moves the list a little while
 * it has more that way, and PageDown a screen, without moving on to another tile.
 */
test("a tile's long list scrolls on the arrows while it has more", async () => {
  const names = Array.from({ length: 50 }, (_, at) => `skill-${String(at + 1).padStart(2, "0")}`);
  const long = {
    ...KIT_INVENTORY,
    kinds: KIT_INVENTORY.kinds.map((one) =>
      one.kind === "skills" && one.read.what === "read"
        ? { ...one, read: { ...one.read, items: names.map((name) => ({ name, source: `/skills/${name}` })), unreadable: [] } }
        : one,
    ),
  };
  mount(manifesting({ kitInventory: long }), SLICES);
  await toSystemPage("Kit");
  await expect.element(region("Skills")).toBeVisible();

  const list = region("Skills").element().querySelector<HTMLElement>("[data-scrolls]");
  expect(list).not.toBeNull();
  expect(list!.scrollHeight).toBeGreaterThan(list!.clientHeight);

  list!.focus();
  await userEvent.keyboard("{ArrowDown}");
  await expect.poll(() => list!.scrollTop).toBeGreaterThan(0);
  const one = list!.scrollTop;
  await userEvent.keyboard("{PageDown}");
  await expect.poll(() => list!.scrollTop).toBeGreaterThan(one);
  expect(document.activeElement).toBe(list);
});
