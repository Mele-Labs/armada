// A Job's shell in a window of its own, one window per terminal (`docs/concepts/session/shared-terminal.md`). The window is a
// Bridge document with Bridge's preload and posture; the shell stays the Job's, so closing the window only puts it away.

import { BrowserWindow } from "electron";
import { join } from "node:path";

import type { TerminalWindowId } from "../shared/api/sessions";
import type { Rect } from "./window-state";

export type Size = {
  width: number;
  height: number;
};

/** What the window tells its holder: the size the person gave it, and that it closed. */
export type TerminalWindowTold = {
  resized: (size: Size) => void;
  closed: () => void;
};

/** One terminal's window, as `TerminalWindows` drives it. */
export type TerminalWindow = {
  focus: () => void;
  close: () => void;
};

/** Makes a terminal's window at `bounds`: Electron's in the app, a fake in a test. */
export type MakeTerminalWindow = (
  id: TerminalWindowId,
  bounds: Partial<Rect> & Size,
  told: TerminalWindowTold,
) => TerminalWindow;

/** About a docked shell's width, and tall enough for a screen of output. */
export const TERMINAL_WINDOW_SIZE: Size = {
  width: 900,
  height: 560,
};

/** Each window open after the first sits this far down and right, so none hides another whole. */
const CASCADE = 24;

/** A terminal's window under one key: the Job's id, and the terminal's ID where one is named. The Job's first, named by none, is the Job's alone. */
const keyOf = ({ sessionId, terminal }: TerminalWindowId): string =>
  terminal === undefined ? sessionId : `${sessionId}#${terminal}`;

export class TerminalWindows {
  private readonly open = new Map<
    string,
    {
      id: TerminalWindowId;
      window: TerminalWindow;
    }
  >();
  private readonly listeners = new Set<(windows: readonly TerminalWindowId[]) => void>();
  /** Shared by every terminal's window, and kept for the app's life only: a window is never restored. */
  private size: Size = TERMINAL_WINDOW_SIZE;

  /** `over` is Bridge's window, which a terminal window opens over; null where none is up. */
  constructor(
    private readonly make: MakeTerminalWindow,
    private readonly over: () => Rect | null,
  ) {}

  /** Opens the terminal's window, or brings the one it has to the front. */
  show(id: TerminalWindowId): void {
    const key = keyOf(id);
    const standing = this.open.get(key);
    if (standing !== undefined) {
      standing.window.focus();
      return;
    }
    const window = this.make(id, this.boundsOf(), {
      resized: (size) => {
        this.size = size;
      },
      closed: () => {
        if (this.open.get(key)?.window !== window) return;
        this.open.delete(key);
        this.changed();
      },
    });
    this.open.set(key, {
      id,
      window,
    });
    this.changed();
  }

  /** Closes the terminal's window, as a move back into Bridge does. Its close is told as the person's is. */
  close(id: TerminalWindowId): void {
    this.open.get(keyOf(id))?.window.close();
  }

  has(id: TerminalWindowId): boolean {
    return this.open.has(keyOf(id));
  }

  /** The terminals with a window open now. */
  ids(): readonly TerminalWindowId[] {
    return [
      ...this.open.values(),
    ].map((one) => one.id);
  }

  /** Hears the terminals with a window, whole, each time one opens or closes. Returns its remover. */
  onChanged(listener: (windows: readonly TerminalWindowId[]) => void): () => void {
    this.listeners.add(listener);
    return () => void this.listeners.delete(listener);
  }

  private boundsOf(): Partial<Rect> & Size {
    const over = this.over();
    if (over === null)
      return {
        ...this.size,
      };
    const step = CASCADE * this.open.size;
    return {
      ...this.size,
      x: Math.round(over.x + (over.width - this.size.width) / 2) + step,
      y: Math.round(over.y + (over.height - this.size.height) / 2) + step,
    };
  }

  private changed(): void {
    const ids = this.ids();
    this.listeners.forEach((one) => one(ids));
  }
}

/** A terminal's window in Electron: a Bridge document that goes nowhere, as Bridge's own window does (`index.ts`). */
export const makeTerminalWindow: MakeTerminalWindow = ({ sessionId, terminal }, bounds, told) => {
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
  window.webContents.setWindowOpenHandler(() => ({
    action: "deny",
  }));
  window.webContents.on("will-navigate", (event) => event.preventDefault());
  window.on("ready-to-show", () => window.show());
  window.on("resize", () => {
    const [width, height] = window.getSize();
    if (width !== undefined && height !== undefined)
      told.resized({
        width,
        height,
      });
  });
  window.on("closed", told.closed);
  void window.loadFile(join(__dirname, "../renderer/terminal-window.html"), {
    query: {
      session: sessionId,
      ...(terminal === undefined
        ? {}
        : {
            terminal,
          }),
    },
  });
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
