// A key given to a palette row runs that row from anywhere in Bridge, with the palette down, as ↵ would. The window reads it
// before a field or a terminal can keep it, and leaves a key that is being recorded in Settings to the recorder.

import { Eye } from "lucide-react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { userEvent } from "vitest/browser";

import "./styles/index.css";
import { NOTHING_YET } from "../../shared/bridge";
import { PaletteMount, type PaletteMountProps } from "./PaletteMount";
import { forgetPaletteMemory, PALETTE_KEY, seedPaletteMemory } from "./palette-memory";
import { setKnownRows } from "./palette-known";

const roots: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
  setKnownRows([]);
  window.localStorage.removeItem(PALETTE_KEY);
  forgetPaletteMemory();
});

beforeEach(() => seedPaletteMemory({ hotkeys: { "nav:reviews": "⇧⌘J" } }));

/** The palette mounted and down, over a window with a Reviews page. */
function mountClosed(open = false) {
  const surface = vi.fn();
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  const props: PaletteMountProps = {
    open,
    onClose: () => {},
    shownStudio: null,
    surfaces: [{ id: "reviews", label: "Reviews", icon: Eye }],
    checkoutRunSheet: NOTHING_YET.checkoutRunSheet,
    failing: null,
    live: true,
    repositories: [],
    picked: null,
    acts: { surface } as unknown as PaletteMountProps["acts"],
    dispatch: { kind: "session", onDispatch: () => {} },
    linkRoutes: { kindOf: () => null, open: () => false },
  };
  root.render(<PaletteMount {...props} />);
  return surface;
}

test("a kept key runs its row with the palette closed, as ↵ on the row would", async () => {
  const surface = mountClosed();
  await userEvent.keyboard("{Meta>}{Shift>}j{/Shift}{/Meta}");
  await vi.waitFor(() => expect(surface).toHaveBeenCalledWith("reviews"));
  expect(surface).toHaveBeenCalledTimes(1);
});

test("another key runs nothing", async () => {
  const surface = mountClosed();
  await userEvent.keyboard("{Meta>}{Shift>}l{/Shift}{/Meta}");
  await userEvent.keyboard("j");
  expect(surface).not.toHaveBeenCalled();
});

test("a key is read ahead of a field that has the focus", async () => {
  const surface = mountClosed();
  const field = document.createElement("input");
  document.body.append(field);
  field.focus();
  await userEvent.keyboard("{Meta>}{Shift>}j{/Shift}{/Meta}");
  field.remove();
  await vi.waitFor(() => expect(surface).toHaveBeenCalledWith("reviews"));
});

test("a key being recorded in Settings is not taken by the row that holds it", async () => {
  const surface = mountClosed();
  const recorder = document.createElement("button");
  recorder.setAttribute("data-recording", "");
  document.body.append(recorder);
  recorder.focus();
  await userEvent.keyboard("{Meta>}{Shift>}j{/Shift}{/Meta}");
  recorder.remove();
  expect(surface).not.toHaveBeenCalled();
});

test("with the palette up the window leaves the key to the palette", async () => {
  const surface = mountClosed(true);
  await userEvent.keyboard("{Meta>}{Shift>}j{/Shift}{/Meta}");
  expect(surface).not.toHaveBeenCalled();
});
