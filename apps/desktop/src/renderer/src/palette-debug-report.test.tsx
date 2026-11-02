// Copy debug report in the palette: a board row, offered with Fleet down too, under Debug Bridge, which stays first and
// selected for "debug" so Return still opens it. Choosing the row opens the sheet.

import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import "./styles/index.css";
import { NOTHING_YET } from "../../shared/bridge";
import { DebugReportFrom, type DebugRead } from "./debug-report/debug-report";
import { PaletteMount, type PaletteMountProps } from "./PaletteMount";

const roots: {
  root: Root;
  host: HTMLElement;
}[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
  Object.defineProperty(window, "armada", {
    value: undefined,
    configurable: true,
  });
});

function mount(live: boolean, read: DebugRead | undefined) {
  Object.defineProperty(window, "armada", {
    value: {},
    configurable: true,
  });
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({
    root,
    host,
  });
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
    dispatch: {
      kind: "session",
      onDispatch: () => {},
    },
    linkRoutes: {
      kindOf: () => null,
      open: () => false,
    },
  };
  root.render(
    <DebugReportFrom read={read}>
      <PaletteMount {...props} />
    </DebugReportFrom>,
  );
}

test('"debug" puts Debug Bridge first and selected, Copy debug report under it, and choosing it opens the sheet', async () => {
  const read = vi.fn(async () => "Armada debug report");
  mount(true, read);
  await userEvent.keyboard("debug");
  const rows = page.getByRole("option");
  await expect.element(rows.nth(0)).toHaveTextContent("Debug Bridge");
  await expect.element(rows.nth(0)).toHaveAttribute("aria-selected", "true");
  await expect.element(rows.nth(1)).toHaveTextContent("Copy debug report");
  await userEvent.keyboard("{ArrowDown}{Enter}");
  await expect
    .element(
      page.getByRole("dialog", {
        name: "Debug report",
      }),
    )
    .toBeVisible();
  await vi.waitFor(() => expect(read).toHaveBeenCalledTimes(1));
});

test("with Fleet down the row is still offered and is not dormant", async () => {
  mount(false, async () => "Armada debug report");
  await userEvent.keyboard("debug report");
  const row = page.getByRole("option", {
    name: /Copy debug report/,
  });
  await expect.element(row).toBeVisible();
  await expect.element(row).not.toHaveAttribute("aria-disabled", "true");
});

test("a window with no reader has no such row", async () => {
  mount(true, undefined);
  await userEvent.keyboard("debug report");
  await expect
    .element(
      page.getByRole("option", {
        name: /Copy debug report/,
      }),
    )
    .not.toBeInTheDocument();
});
