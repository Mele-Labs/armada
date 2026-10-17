// Where the main window was when it last closed, so a relaunch puts it back.
//
// One small JSON file in `userData`: main kept no machine-local state of its own
// before this, so there was nothing to reuse. `place` is the whole rule and takes
// the displays as an argument so a test can unplug a monitor.

import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { app, screen, type BrowserWindow } from "electron";

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface WindowState extends Rect {
  maximized: boolean;
  fullScreen: boolean;
}

export const DEFAULT_SIZE = { width: 1280, height: 800 } as const;

/** Smallest overlap, in pixels either way, that counts as a window a person can still grab. */
const REACHABLE = 80;

/**
 * Where the window opens: the saved rectangle if it still meets a display, otherwise
 * the default size centred on the primary one. `workAreas` is every display's, primary first.
 */
export function place(saved: Rect | null, workAreas: readonly Rect[]): Rect {
  if (saved !== null && workAreas.some((area) => overlaps(saved, area))) return saved;
  const primary = workAreas[0] ?? { x: 0, y: 0, ...DEFAULT_SIZE };
  const width = Math.min(DEFAULT_SIZE.width, primary.width);
  const height = Math.min(DEFAULT_SIZE.height, primary.height);
  return {
    x: Math.round(primary.x + (primary.width - width) / 2),
    y: Math.round(primary.y + (primary.height - height) / 2),
    width,
    height,
  };
}

function overlaps(rect: Rect, area: Rect): boolean {
  const across = Math.min(rect.x + rect.width, area.x + area.width) - Math.max(rect.x, area.x);
  const down = Math.min(rect.y + rect.height, area.y + area.height) - Math.max(rect.y, area.y);
  return across >= REACHABLE && down >= REACHABLE;
}

const file = (): string => join(app.getPath("userData"), "window-state.json");

function read(): WindowState | null {
  try {
    const value = JSON.parse(readFileSync(file(), "utf8")) as Partial<WindowState>;
    const numbers = [value.x, value.y, value.width, value.height];
    if (!numbers.every((n) => typeof n === "number" && Number.isFinite(n))) return null;
    if (value.width! < 200 || value.height! < 200) return null;
    return value as WindowState;
  } catch {
    return null;
  }
}

/** Bounds and mode for a new window, from the last one saved. */
export function restore(): { bounds: Rect; maximized: boolean; fullScreen: boolean } {
  const saved = read();
  const areas = screen.getAllDisplays().map((d) => d.workArea);
  const primary = screen.getPrimaryDisplay().workArea;
  const ordered = [primary, ...areas.filter((a) => a !== primary)];
  return { bounds: place(saved, ordered), maximized: saved?.maximized === true, fullScreen: saved?.fullScreen === true };
}

/** Keeps the file current as the window moves, resizes and closes. */
export function remember(window: BrowserWindow): void {
  let timer: NodeJS.Timeout | undefined;
  const save = (): void => {
    if (window.isDestroyed()) return;
    // A maximized or full screen window's own bounds are the screen's; keep the normal ones.
    const bounds = window.getNormalBounds();
    const state: WindowState = { ...bounds, maximized: window.isMaximized(), fullScreen: window.isFullScreen() };
    try {
      writeFileSync(file(), JSON.stringify(state));
    } catch {
      // A window that cannot be remembered still opens.
    }
  };
  const later = (): void => {
    clearTimeout(timer);
    timer = setTimeout(save, 400);
  };
  for (const event of ["move", "resize", "maximize", "unmaximize", "enter-full-screen", "leave-full-screen"] as const) {
    window.on(event as "move", later);
  }
  window.on("close", () => {
    clearTimeout(timer);
    save();
  });
}
