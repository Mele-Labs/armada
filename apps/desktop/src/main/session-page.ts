// The web view a Session's page or doc is shown in, over the panel's body.
//
// **The way the capture window holds a page**: a `WebContentsView` main owns, in a session of its
// own, with no preload and every permission refused. Bridge's renderer refuses navigation —
// `bridge.md`, *Security posture* — so the page never runs in it. The renderer reports the rect
// of the panel's body; main places the view there and removes it when the panel closes.

import { session, shell, WebContentsView, BrowserWindow } from "electron";
import type { Followed, SessionRecord } from "@armada/protocol";
import { SESSIONS_CHANNELS } from "../shared/api/sessions";
import { boundsOf, isEscape } from "./session-page-bounds";
import { namesPage } from "./session-file";

const PARTITION = "persist:armada-session-pages";
let prepared = false;

function pageSession(): Electron.Session {
  const part = session.fromPartition(PARTITION);
  if (prepared) return part;
  prepared = true;
  part.setPermissionRequestHandler((_contents, _permission, decide) => decide(false));
  part.setPermissionCheckHandler(() => false);
  part.setDisplayMediaRequestHandler(() => {}, { useSystemPicker: false });
  part.on("will-download", (event) => event.preventDefault());
  return part;
}

export class SessionPages {
  /** One view per window, since a window has one panel. */
  private readonly views = new Map<number, { view: WebContentsView; window: BrowserWindow; address: string }>();

  show(sender: Electron.WebContents, record: SessionRecord | undefined, address: string, bounds: unknown): Followed {
    if (!namesPage(record, address)) return { ok: false, why: "not_addressable", address };
    const window = BrowserWindow.fromWebContents(sender);
    if (window === null || window.isDestroyed()) return { ok: false, why: "refused", address, detail: "no window" };
    const held = this.views.get(window.id);
    const rect = boundsOf(bounds, window.getContentBounds());
    if (held !== undefined && held.address === address) {
      held.view.setBounds(rect);
      return { ok: true };
    }
    this.hide(sender);
    const view = new WebContentsView({
      webPreferences: { session: pageSession(), sandbox: true, contextIsolation: true, nodeIntegration: false },
    });
    // A page that opens another opens it in the browser, never in a window of ours.
    view.webContents.setWindowOpenHandler(({ url }) => {
      if (/^https?:\/\//i.test(url)) void shell.openExternal(url);
      return { action: "deny" };
    });
    // The view has the keyboard while it is shown, so Esc never reaches the renderer's own binding.
    view.webContents.on("before-input-event", (event, input) => {
      if (!isEscape(input)) return;
      event.preventDefault();
      if (!sender.isDestroyed()) sender.send(SESSIONS_CHANNELS.sessionPageEscape);
    });
    window.contentView.addChildView(view);
    view.setBounds(rect);
    this.views.set(window.id, { view, window, address });
    window.once("closed", () => this.views.delete(window.id));
    void view.webContents.loadURL(address);
    return { ok: true };
  }

  move(sender: Electron.WebContents, bounds: unknown): void {
    const window = BrowserWindow.fromWebContents(sender);
    const held = window === null ? undefined : this.views.get(window.id);
    if (window === null || held === undefined || window.isDestroyed()) return;
    held.view.setBounds(boundsOf(bounds, window.getContentBounds()));
  }

  hide(sender: Electron.WebContents): void {
    const window = BrowserWindow.fromWebContents(sender);
    const held = window === null ? undefined : this.views.get(window.id);
    if (window === null || held === undefined) return;
    this.views.delete(window.id);
    if (!window.isDestroyed()) window.contentView.removeChildView(held.view);
    if (!held.view.webContents.isDestroyed()) held.view.webContents.close();
  }
}
