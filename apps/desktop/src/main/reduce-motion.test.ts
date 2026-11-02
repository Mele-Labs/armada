// `features.reduceMotion`: each Bridge window answers `prefers-reduced-motion` as if the Mac asked, and answers as the Mac does once it is off. `reduce-motion.ts`.
// The command itself was read in a hidden Electron window: it holds across a navigation, the page hears the change, and detaching gives the Mac's answer back.

import { afterEach, beforeEach, expect, test, vi } from "vitest";

type Fake = { webContents: { isDestroyed: () => boolean; debugger: { isAttached: () => boolean; attach: () => void; detach: () => void; sendCommand: ReturnType<typeof vi.fn> } } };

const made = vi.hoisted(() => ({ windows: [] as unknown[], created: [] as ((event: unknown, window: unknown) => void)[] }));

vi.mock("electron", () => ({
  app: { on: (_name: string, listener: (event: unknown, window: unknown) => void) => made.created.push(listener) },
  BrowserWindow: { getAllWindows: () => made.windows },
}));

const windowOf = (): Fake => {
  let attached = false;
  return {
    webContents: {
      isDestroyed: () => false,
      debugger: {
        isAttached: () => attached,
        attach: () => {
          attached = true;
        },
        detach: () => {
          attached = false;
        },
        sendCommand: vi.fn(async () => undefined),
      },
    },
  };
};

const REDUCE = ["Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "reduce" }] }];

let follow: (on: boolean) => void;

beforeEach(async () => {
  vi.resetModules();
  made.windows.length = 0;
  made.created.length = 0;
  ({ followReduceMotion: follow } = await import("./reduce-motion"));
});

afterEach(() => vi.restoreAllMocks());

test("on, every open window is told to reduce motion", () => {
  const one = windowOf();
  const two = windowOf();
  made.windows.push(one, two);
  follow(true);
  expect(one.webContents.debugger.sendCommand).toHaveBeenCalledWith(...REDUCE);
  expect(two.webContents.debugger.sendCommand).toHaveBeenCalledWith(...REDUCE);
});

test("off, a window answers as the Mac does again", () => {
  const one = windowOf();
  made.windows.push(one);
  follow(true);
  follow(false);
  expect(one.webContents.debugger.isAttached()).toBe(false);
});

test("off from the start, nothing is touched", () => {
  const one = windowOf();
  made.windows.push(one);
  follow(false);
  expect(one.webContents.debugger.isAttached()).toBe(false);
  expect(one.webContents.debugger.sendCommand).not.toHaveBeenCalled();
});

test("a window opened while it is on is told too, and one opened while it is off is not", () => {
  follow(true);
  const late = windowOf();
  for (const listener of made.created) listener({}, late);
  expect(late.webContents.debugger.sendCommand).toHaveBeenCalledWith(...REDUCE);

  follow(false);
  const later = windowOf();
  for (const listener of made.created) listener({}, later);
  expect(later.webContents.debugger.sendCommand).not.toHaveBeenCalled();
});

test("the same answer twice sends nothing more, as Fleet's list is published again and again", () => {
  const one = windowOf();
  made.windows.push(one);
  follow(true);
  follow(true);
  expect(one.webContents.debugger.sendCommand).toHaveBeenCalledTimes(1);
});
