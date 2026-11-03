// What a Job's walk needs of Electron in a test: Bridge's window to place a walk in, and the bar's view. A walk's page is a
// stand-in of each test's own.

import type { BaseWindow } from "electron";

import type { CaptureWindows } from "./windows";

/** Bridge's own window, 1200 by 800 inside. */
export const HOST = {
  isDestroyed: () => false,
  getContentBounds: () => ({
    width: 1200,
    height: 800,
  }),
} as unknown as BaseWindow;

/** Where a Job's pane reports its walk. */
export const FRAME = {
  x: 40,
  y: 60,
  width: 800,
  height: 500,
};

/** The bar's view, which a test's `vi.mock("./views", ...)` hands out: `webContents.sent` is what main told it. */
export const fakeViews = {
  barView: () => ({
    webContents: {
      id: 1,
      sent: [] as unknown[][],
      on: () => {},
      send(...args: unknown[]) {
        this.sent.push(args);
      },
      isDestroyed: () => false,
      close: () => {},
      focus: () => {},
    },
    setVisible: () => {},
  }),
  panelView: () => ({
    webContents: {
      id: 2,
      on: () => {},
      send: () => {},
      isDestroyed: () => false,
      close: () => {},
      focus: () => {},
    },
    setVisible: () => {},
  }),
};

/** The Job's pane puts its walk at `url` in `frame`, which is when its page is made. */
export const placeAt = (
  shown: CaptureWindows,
  sessionId: string,
  url: string,
  frame: unknown = FRAME,
): void => shown.place(HOST, sessionId, url, frame);
