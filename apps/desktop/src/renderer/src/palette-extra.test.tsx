// A row a host page registers (`palette-extra.ts`) is listed in the palette, found by its words, and choosing it runs it. Nothing registered, none is listed.

import { FlaskConical } from "lucide-react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import "./styles/index.css";
import { NOTHING_YET } from "../../shared/bridge";
import { PaletteMount, type PaletteMountProps } from "./PaletteMount";
import { setPaletteExtras } from "./palette-extra";

const roots: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
  Object.defineProperty(window, "armada", { value: undefined, configurable: true });
  setPaletteExtras([]);
});

function mount(live: boolean) {
  Object.defineProperty(window, "armada", { value: {}, configurable: true });
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
}

test("a registered row is listed with its glyph, and choosing it runs it", async () => {
  const run = vi.fn();
  setPaletteExtras([{ id: "mock-scenario", label: "Mock scenario", icon: FlaskConical, run }]);
  mount(true);
  await userEvent.keyboard("mock scen");
  const row = page.getByRole("option", { name: /^Mock scenario/ });
  await expect.element(row).toBeVisible();
  expect(row.element().querySelector(".armada-palette__glyph svg")).not.toBeNull();
  await userEvent.keyboard("{Enter}");
  await vi.waitFor(() => expect(run).toHaveBeenCalledTimes(1));
});

test("with nothing registered the palette lists no such row", async () => {
  mount(true);
  await userEvent.keyboard("mock scen");
  await expect.element(page.getByRole("option", { name: /^Mock scenario/ })).not.toBeInTheDocument();
});
