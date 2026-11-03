// Every Bridge document holds its loops to the clock and stands still unused. A walk panel with neither read 20 to 25% of a core,
// and a mock page left open 120 to 180%. The entries are read as text, since each one mounts a whole window on import.

import { afterEach, expect, test, vi } from "vitest";

import { IDLE_MS, watchStill } from "./still";

type Globbing = ImportMeta & {
  glob(
    pattern: string,
    options: {
      eager: true;
      query: "?raw";
      import: "default";
    },
  ): Record<string, string>;
};
const ENTRIES = (import.meta as Globbing).glob("./**/main.tsx", {
  eager: true,
  query: "?raw",
  import: "default",
});
const WALK_WINDOW = [
  "./mock/main.tsx",
  "./walk-panel/main.tsx",
  "./capture-window/main.tsx",
];

let stop: () => void = () => undefined;

afterEach(() => {
  stop();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

test("every document a window loads starts the motion rules", () => {
  expect(Object.keys(ENTRIES)).toEqual(expect.arrayContaining(WALK_WINDOW));
  const without = Object.entries(ENTRIES)
    .filter(([, source]) => !source.includes("startMotionRules("))
    .map(([entry]) => entry);
  expect(without).toEqual([]);
  // The page, bar and panel of a walk window hold its focus in turn, so none of them may stop for losing it.
  for (const entry of WALK_WINDOW)
    expect(ENTRIES[entry], entry).toMatch(/startMotionRules\(\{\s*focus: false,?\s*\}\)/);
});

test("a walk window's page keeps moving without focus, stands still after a minute with no input and wakes on the pointer", () => {
  vi.useFakeTimers();
  vi.spyOn(document, "hasFocus").mockReturnValue(false);
  stop = watchStill({
    focus: false,
  });
  const root = document.documentElement;

  window.dispatchEvent(new Event("blur"));
  expect(root.hasAttribute("data-still")).toBe(false);
  expect(root.hasAttribute("data-idle")).toBe(false);

  vi.advanceTimersByTime(IDLE_MS + 5_000);
  expect(root.hasAttribute("data-idle")).toBe(true);

  window.dispatchEvent(new PointerEvent("pointermove"));
  expect(root.hasAttribute("data-idle")).toBe(false);
});

test("Bridge's own windows still stop when they lose focus", () => {
  vi.spyOn(document, "hasFocus").mockReturnValue(false);
  stop = watchStill();
  expect(document.documentElement.hasAttribute("data-still")).toBe(true);
});
