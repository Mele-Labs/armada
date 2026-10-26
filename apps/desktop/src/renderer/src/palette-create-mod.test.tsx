// Create mod in the palette: one row, found by its words, that starts an empty Mod Job through Fleet. With Fleet down the row is dormant
// and says why, and choosing it starts nothing.

import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { StartModSession } from "@armada/protocol";

import "./styles/index.css";
import { NOTHING_YET } from "../../shared/bridge";
import { PaletteMount, type PaletteMountProps } from "./PaletteMount";

const roots: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
  Object.defineProperty(window, "armada", { value: undefined, configurable: true });
});

function mount(live: boolean) {
  const startModSession = vi.fn(async (_start: StartModSession) => ({ ok: true as const, value: { session_id: "01MOD", title: "Mod" } }));
  Object.defineProperty(window, "armada", { value: { startModSession }, configurable: true });
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  const props: PaletteMountProps = {
    open: true,
    onClose: () => {},
    shownStudio: null,
    surfaces: [],
    checkoutRunSheet: NOTHING_YET.checkoutRunSheet,
    failing: null,
    live,
    repositories: [],
    picked: null,
    acts: {} as PaletteMountProps["acts"],
    dispatch: { kind: "session", onDispatch: () => {} },
    linkRoutes: { kindOf: () => null, open: () => false },
  };
  root.render(<PaletteMount {...props} />);
  return startModSession;
}

test("Create mod is a row in the palette, and choosing it starts an empty Mod Job", async () => {
  const start = mount(true);
  await userEvent.keyboard("create mod");
  await expect.element(page.getByRole("option", { name: /Create mod/ })).toBeVisible();
  // The row carries a glyph, which a board row could not until the palette took an icon for it.
  expect(page.getByRole("option", { name: /Create mod/ }).element().querySelector(".armada-palette__glyph svg")).not.toBeNull();
  await userEvent.keyboard("{Enter}");
  await vi.waitFor(() => expect(start).toHaveBeenCalledTimes(1));
  expect(start).toHaveBeenCalledWith({});
});

test("with Fleet down the row says why, and choosing it starts nothing", async () => {
  const start = mount(false);
  await userEvent.keyboard("create mod");
  const row = page.getByRole("option", { name: /Create mod/ });
  await expect.element(row).toHaveAttribute("aria-disabled", "true");
  await expect.element(row).toHaveTextContent("Fleet is not connected");
  await userEvent.keyboard("{Enter}");
  expect(start).not.toHaveBeenCalled();
});
