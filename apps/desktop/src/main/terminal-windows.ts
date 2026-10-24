// A Job's shell in a window of its own, one window per Job (`docs/concepts/session/shared-terminal.md`). The window is a
// Bridge document with Bridge's preload and posture; the shell stays the Job's, so closing the window only puts it away.

import { BrowserWindow } from "electron";
import { join } from "node:path";

import type { Rect } from "./window-state";

export type Size = { width: number; height: number };

/** What the window tells its holder: the size the person gave it, and that it closed. */
export type TerminalWindowTold = { resized: (size: Size) => void; closed: () => void };

/** One Job's terminal window, as `TerminalWindows` drives it. */
export type TerminalWindow = { focus: () => void; close: () => void };

/** Makes a Job's window at `bounds`: Electron's in the app, a fake in a test. */
export type MakeTerminalWindow = (sessionId: string, bounds: Partial<Rect> & Size, told: TerminalWindowTold) => TerminalWindow;

/** About a docked shell's width, and tall enough for a screen of output. */
export const TERMINAL_WINDOW_SIZE: Size = { width: 900, height: 560 };

/** Each window open after the first sits this far down and right, so none hides another whole. */
const CASCADE = 24;

export class TerminalWindows {
  private readonly open = new Map<string, TerminalWindow>();
  private readonly listeners = new Set<(sessionIds: readonly string[]) => void>();
  /** Shared by every Job's window, and kept for the app's life only: a window is never restored. */
  private size: Size = TERMINAL_WINDOW_SIZE;

  /** `over` is Bridge's window, which a terminal window opens over; null where none is up. */
  constructor(
    private readonly make: MakeTerminalWindow,
    private readonly over: () => Rect | null,
  ) {}

  /** Opens the Job's window, or brings the one it has to the front. */
  show(sessionId: string): void {
    const standing = this.open.get(sessionId);
    if (standing !== undefined) {
      standing.focus();
      return;
    }
    const window = this.make(sessionId, this.boundsOf(), {
      resized: (size) => {
        this.size = size;
      },
      closed: () => {
        if (this.open.get(sessionId) !== window) return;
        this.open.delete(sessionId);
        this.changed();
      },
    });
    this.open.set(sessionId, window);
    this.changed();
  }

  /** Closes the Job's window, as a move back into Bridge does. Its close is told as the person's is. */
  close(sessionId: string): void {
    this.open.get(sessionId)?.close();
  }

  has(sessionId: string): boolean {
    return this.open.has(sessionId);
  }

  /** The Jobs with a window open now. */
  sessionIds(): readonly string[] {
    return [...this.open.keys()];
  }

  /** Hears the Jobs with a window, whole, each time one opens or closes. Returns its remover. */
  onChanged(listener: (sessionIds: readonly string[]) => void): () => void {
    this.listeners.add(listener);
    return () => void this.listeners.delete(listener);
  }

  private boundsOf(): Partial<Rect> & Size {
    const over = this.over();
    if (over === null) return { ...this.size };
    const step = CASCADE * this.open.size;
    return {
      ...this.size,
      x: Math.round(over.x + (over.width - this.size.width) / 2) + step,
      y: Math.round(over.y + (over.height - this.size.height) / 2) + step,
    };
  }

  private changed(): void {
    const ids = this.sessionIds();
    this.listeners.forEach((one) => one(ids));
  }
}

/** A Job's terminal window in Electron: a Bridge document that goes nowhere, as Bridge's own window does (`index.ts`). */
export const makeTerminalWindow: MakeTerminalWindow = (sessionId, bounds, told) => {
  const window = new BrowserWindow({
    ...bounds,
    show: false,
    minWidth: 360,
    minHeight: 200,
    webPreferences: {
      preload: join(__dirname, "../preload/index.js"),
      sandbox: true,
      contextIsolation: true,
      nodeIntegration: false,
    },
  });
  window.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
  window.webContents.on("will-navigate", (event) => event.preventDefault());
  window.on("ready-to-show", () => window.show());
  window.on("resize", () => {
    const [width, height] = window.getSize();
    if (width !== undefined && height !== undefined) told.resized({ width, height });
  });
  window.on("closed", told.closed);
  void window.loadFile(join(__dirname, "../renderer/terminal-window.html"), { query: { session: sessionId } });
  return {
    focus: () => {
      if (window.isDestroyed()) return;
      if (window.isMinimized()) window.restore();
      window.show();
      window.focus();
    },
    close: () => {
      if (!window.isDestroyed()) window.close();
    },
  };
};
