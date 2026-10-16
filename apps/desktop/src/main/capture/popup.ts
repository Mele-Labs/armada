// The popup a sign-in provider opens from a Session's walk window — `docs/practices/capture-window.md`,
// *A popup a Session's page opened*.
//
// **Only the window decides whether one opens**: this is what a popup that was allowed is. A child
// window of the walk's around the contents Electron made for it, which take the page's own session and
// preferences (no preload, sandboxed), held to the same origins as the page is. It closes when the page calls
// `window.close()`, when the person closes it, and when the walk window does.

import { BaseWindow, WebContentsView, type BrowserWindowConstructorOptions, type WebContents } from "electron";

import type { CaptureRefusedWhat } from "../../shared/capture-window";
import { webOrigin } from "./address";

/** What a provider's sign-in window is sized to: a column, as every provider draws one. */
const WIDTH = 520;
const HEIGHT = 720;

/** What the popup needs from the window around it. */
export type PopupHold = {
  /** The page that opened it, whose session it runs in. */
  opener: WebContents;
  /** The walk window it is a child of. */
  parent: BaseWindow;
  /** Whether an address is on the shown origin or a named one, and the window is still serving. */
  allowed: (address: string) => boolean;
  /** An address refused, said on the walk's bar. */
  refuse: (address: string, what: CaptureRefusedWhat) => void;
  /** It closed, whoever closed it. */
  closed: () => void;
};

/** The popup, once opened. */
export type Popup = { contents: WebContents; close: () => void };

/**
 * Open the popup and hold it. **Around `options.webContents`**, which Electron made for the page's
 * popup and requires the window to be built on. **Nothing here throws into main**: a popup that cannot
 * be built is closed and refused quietly, as every popup was.
 */
export function openPopup(hold: PopupHold, options: BrowserWindowConstructorOptions & { webContents?: WebContents }): Popup | null {
  const made = options.webContents;
  if (made === undefined) return null;
  let window: BaseWindow | null = null;
  try {
    window = new BaseWindow({ parent: hold.parent, width: WIDTH, height: HEIGHT, title: "", show: false });
    return build(hold, made, window);
  } catch {
    if (window !== null && !window.isDestroyed()) window.close();
    if (!made.isDestroyed()) made.close();
    return null;
  }
}

/**
 * Build it. **Every navigation off the origins the page may reach is refused**, as
 * the page's own are, and a window it tries to open in turn is refused outright: one popup is one hop.
 */
function build(hold: PopupHold, made: WebContents, window: BaseWindow): Popup {
  const view = new WebContentsView({ webContents: made });
  window.contentView.addChildView(view);
  const fit = (): void => {
    const { width, height } = window.getContentBounds();
    view.setBounds({ x: 0, y: 0, width, height });
  };
  fit();
  window.on("resize", fit);

  const contents = view.webContents;
  // The page's agent, set before the first request leaves: a popup does not take the session's on its own.
  contents.setUserAgent(hold.opener.getUserAgent());
  contents.setWindowOpenHandler(({ url }) => {
    hold.refuse(url, "window");
    return { action: "deny" };
  });
  contents.on("will-navigate", (event, url) => {
    if (hold.allowed(url)) return;
    event.preventDefault();
    hold.refuse(url, "navigation");
  });
  contents.on("will-redirect", (event, url) => {
    if (hold.allowed(url)) return;
    event.preventDefault();
    hold.refuse(url, "redirect");
  });
  contents.on("will-frame-navigate", (event) => {
    if (event.isMainFrame || hold.allowed(event.url)) return;
    event.preventDefault();
  });
  // The OS title is the origin the popup is on, so the person can tell which provider he is typing into.
  const titled = (): void => {
    if (!window.isDestroyed()) window.setTitle(webOrigin(contents.getURL()) ?? "");
  };
  contents.on("did-navigate", titled);
  contents.on("did-navigate-in-page", titled);
  // `window.close()` in the popup destroys its contents, and a window with nothing in it goes with them.
  contents.on("destroyed", () => {
    if (!window.isDestroyed()) window.close();
  });
  window.on("closed", () => {
    // A view's document outlives a closed `BaseWindow` unless it is closed with it.
    if (!contents.isDestroyed()) contents.close();
    hold.closed();
  });

  window.show();
  return { contents, close: () => (window.isDestroyed() ? undefined : window.close()) };
}
