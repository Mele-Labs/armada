// armada-allow-off-contract: the one colour here is Electron's own spelling for a view with no background of its own, not a design value
// The walk window's three views, built — `docs/practices/capture-window.md`. Out of `window.ts`,
// which holds what the window does with them.
//
// **The page holds nothing of Armada's.** The bar and the panel are Bridge documents loaded from disk
// with Bridge's preload and Bridge's CSP; the page is the only one that is not, and it has no preload.

import tokens from "@armada/tokens/tokens.json";
import { app, session, WebContentsView, type Session } from "electron";
import { join } from "node:path";

import { chromeAgent, granted, onAnyOrigin, partitionFor, type Pinned } from "./address";
import { panelQuery, type PanelSubject } from "./layout";
import { deciderOf, may, reachesLocalNetwork } from "./permissions";

/**
 * Every partition already given its handlers. **Kept because a partition is
 * per repository and two windows share one**, so the download refusal would
 * otherwise be added twice and the second window's close would take both.
 */
const held = new Set<string>();

/**
 * The session the page runs in: persistent, one per repository, and nothing of Bridge's in it.
 *
 * **Every permission is denied without asking, bar one, and a download is refused.** The walk window exists to look at
 * a page, not to host one. The exception is local network access, for a sign-in provider's page reaching the sign-in
 * app on this machine, and only on an origin the window may reach (`permissions.ts`). A Job's browser page registers a
 * decider instead, which answers from the owner's saved choices and may ask him (`site-permissions.ts`).
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
  const agent = chromeAgent(app.userAgentFallback);
  part.setUserAgent(agent);
  // A popup's first request left with Electron's agent in the owner's walk: the header is set on every request too.
  part.webRequest.onBeforeSendHeaders(({ requestHeaders }, done) => done({ requestHeaders: { ...requestHeaders, "User-Agent": agent } }));
  // `permission` is compared as a string: Electron sends "local-network-access", which its typed union omits.
  part.setPermissionRequestHandler((contents, permission: string, decide, details) => {
    const own = deciderOf(contents);
    if (own === undefined) decide(granted(permission, details.requestingUrl, may(contents)));
    else void own.request(permission, details.requestingUrl, "mediaTypes" in details ? details : {}).then(decide);
  });
  part.setPermissionCheckHandler((contents, permission: string, requestingOrigin, details) => {
    const own = deciderOf(contents);
    return own === undefined ? granted(permission, requestingOrigin, may(contents)) : own.check(permission, requestingOrigin, { mediaTypes: details.mediaType === "video" || details.mediaType === "audio" ? [details.mediaType] : [] });
  });
  part.setDisplayMediaRequestHandler(() => {}, { useSystemPicker: false });
  part.on("will-download", (event, item, contents) => {
    const own = deciderOf(contents);
    if (own === undefined) event.preventDefault();
    else own.download(event, item.getURL(), () => contents.downloadURL(item.getURL()));
  });
  return part;
}

/**
 * Bridge's ground, `--bg-base` as `tokens.json` holds it: the dark theme's, since main is not told which theme the
 * renderer drew. Electron paints a view white until its page draws, which read as a broken page.
 */
const GROUND = tokens.tokens.find((one) => one.name === "--bg-base")?.value;

/** The page: another repository's running app, with no preload and a session of its own. */
export function pageView(pin: Pinned): WebContentsView {
  const view = new WebContentsView({
    webPreferences: {
      session: sessionFor(pin.manifestId),
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
  if (GROUND !== undefined) view.setBackgroundColor(GROUND);
  reachesLocalNetwork(view.webContents, (address) => onAnyOrigin(pin, address));
  return view;
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
 * The panel beside the page: the Job, drawn by Bridge's own components. **Told its first subject by the
 * query**, and the walk window's every one after by `walkPanel.subject`; what it draws it reads as Bridge's window does.
 * `null`: no Job yet, as a walk window that opened on a pull request's walk has.
 */
export function panelView(subject: PanelSubject | null): WebContentsView {
  const view = bridgeView();
  void view.webContents.loadFile(join(__dirname, "../renderer/walk-panel.html"), subject === null ? {} : { query: panelQuery(subject) });
  return view;
}
