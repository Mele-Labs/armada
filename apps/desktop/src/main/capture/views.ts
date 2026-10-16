// armada-allow-off-contract: the one colour here is Electron's own spelling for a view with no background of its own, not a design value
// The walk window's three views, built — `docs/practices/capture-window.md`. Out of `window.ts`,
// which holds what the window does with them.
//
// **The page holds nothing of Armada's.** The bar and the panel are Bridge documents loaded from disk
// with Bridge's preload and Bridge's CSP; the page is the only one that is not, and it has no preload.

import { app, session, WebContentsView, type Session } from "electron";
import { join } from "node:path";

import { chromeAgent, partitionFor } from "./address";
import { panelQuery, type PanelSubject } from "./layout";

/**
 * Every partition already given its handlers. **Kept because a partition is
 * per repository and two windows share one**, so the download refusal would
 * otherwise be added twice and the second window's close would take both.
 */
const held = new Set<string>();

/**
 * The session the page runs in: persistent, one per repository, and nothing of Bridge's in it.
 *
 * **Every permission is denied without asking.** A page that wants the camera,
 * the clipboard, a notification or the screen gets no dialog and no capability
 * — this window exists to look at a page, not to host one.
 *
 * **The user agent is Electron's with its own two tokens taken out**, for every page and popup in the
 * partition: a sign-in provider that refuses a browser it reads as Electron then reads Chrome.
 * Bridge's own views are on the default session and keep Electron's.
 */
function sessionFor(manifestId: string | null): Session {
  const name = partitionFor(manifestId);
  const part = session.fromPartition(name);
  if (held.has(name)) return part;
  held.add(name);
  part.setUserAgent(chromeAgent(app.userAgentFallback));
  part.setPermissionRequestHandler((_contents, _permission, decide) => decide(false));
  part.setPermissionCheckHandler(() => false);
  part.setDisplayMediaRequestHandler(() => {}, { useSystemPicker: false });
  part.on("will-download", (event) => event.preventDefault());
  return part;
}

/** The page: another repository's running app, with no preload and a session of its own. */
export function pageView(manifestId: string | null): WebContentsView {
  return new WebContentsView({
    webPreferences: {
      session: sessionFor(manifestId),
      // **No preload.** The only surface that cannot be widened by accident
      // is an absent one — `capture-window.md`, *What the page can reach into*.
      sandbox: true,
      contextIsolation: true,
      nodeIntegration: false,
      nodeIntegrationInSubFrames: false,
      webviewTag: false,
      spellcheck: false,
      // A dialog the page draws would be a dialog that looks like Armada's.
      disableDialogs: true,
      // Neither loosened. A page's own CSP is what governs it, and Bridge
      // rewrites no header to make an awkward app render.
      webSecurity: true,
      allowRunningInsecureContent: false,
    },
  });
}

/** A Bridge document in this window: Bridge's preload, Bridge's CSP, and it goes nowhere. */
function bridgeView(): WebContentsView {
  const view = new WebContentsView({
    webPreferences: {
      preload: join(__dirname, "../preload/index.js"),
      sandbox: true,
      contextIsolation: true,
      nodeIntegration: false,
    },
  });
  // Bridge's own window's rule holds over every Bridge document.
  view.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
  view.webContents.on("will-navigate", (event) => event.preventDefault());
  return view;
}

/** The bar. **Fully transparent**: it paints its own strip and nothing else, so the page shows through it. */
export function barView(): WebContentsView {
  const view = bridgeView();
  view.setBackgroundColor("#00000000");
  void view.webContents.loadFile(join(__dirname, "../renderer/capture.html"));
  return view;
}

/**
 * The panel beside the page: the Session's thread or the Job, drawn by Bridge's own components. **Told
 * its subject by the query alone**; what it draws it reads as Bridge's window does.
 */
export function panelView(subject: PanelSubject): WebContentsView {
  const view = bridgeView();
  void view.webContents.loadFile(join(__dirname, "../renderer/walk-panel.html"), { query: panelQuery(subject) });
  return view;
}
