// The web view a Session's page or doc is shown in, over the panel's body.
//
// **The way the capture window holds a page**: a `WebContentsView` main owns, in a session of its
// own, with no preload and every permission refused. Bridge's renderer refuses navigation —
// `bridge.md`, *Security posture* — so the page never runs in it. The renderer reports the rect
// of the panel's body; main places the view there and removes it when the panel closes.
//
// **A page drawn small is a picture of it, not a view.** A native view cannot be scaled, so the live view asks for
// snapshots: main loads the page in a view that is never added to a window, takes `capturePage()` of it as it changes
// and every few seconds, and sends the PNG to the window that asked (`session-snapshot.ts`).

import { session, shell, WebContentsView, BrowserWindow } from "electron";
import type { Followed, SessionRecord } from "@armada/protocol";
import { SESSIONS_CHANNELS } from "../shared/api/sessions";
import { boundsOf, isEscape } from "./session-page-bounds";
import { namesPage } from "./session-file";
import { keepSnapshots } from "./session-snapshot";

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

/** The size the page is laid out at for its picture, and the widest the picture is sent. The renderer's live frame lays out at the same size. */
const SNAPSHOT = { width: 800, height: 440 } as const;

export class SessionPages {
  /** The view a window's live page is held in out of sight, and what ends its pictures. One per window. */
  private readonly shots = new Map<number, { address: string; view: WebContentsView; stop: () => void }>();

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

  /** Starts pictures of a page the Session's ledger names, for the window that asked. A second ask for the same page keeps the first. */
  watch(sender: Electron.WebContents, record: SessionRecord | undefined, address: string): Followed {
    if (!namesPage(record, address)) return { ok: false, why: "not_addressable", address };
    const window = BrowserWindow.fromWebContents(sender);
    if (window === null || window.isDestroyed()) return { ok: false, why: "refused", address, detail: "no window" };
    if (this.shots.get(window.id)?.address === address) return { ok: true };
    this.stop(sender);
    const view = new WebContentsView({
      webPreferences: { session: pageSession(), sandbox: true, contextIsolation: true, nodeIntegration: false, backgroundThrottling: false },
    });
    // Never added to a window: it has the size the page is laid out at and nothing else.
    view.setBounds({ x: 0, y: 0, ...SNAPSHOT });
    const contents = view.webContents;
    contents.setWindowOpenHandler(({ url }) => {
      if (/^https?:\/\//i.test(url)) void shell.openExternal(url);
      return { action: "deny" };
    });
    const stopTaking = keepSnapshots(
      {
        take: async () => {
          if (contents.isDestroyed()) return undefined;
          const image = await contents.capturePage();
          if (image.isEmpty()) return undefined;
          return (image.getSize().width > SNAPSHOT.width ? image.resize({ width: SNAPSHOT.width }) : image).toPNG();
        },
        onChange: (on) => {
          contents.on("did-finish-load", on);
          contents.on("did-navigate-in-page", on);
        },
      },
      (png) => {
        if (!sender.isDestroyed()) sender.send(SESSIONS_CHANNELS.sessionSnapshot, { address, png });
      },
    );
    this.shots.set(window.id, {
      address,
      view,
      stop: () => {
        stopTaking();
        if (!contents.isDestroyed()) contents.close();
      },
    });
    const id = window.id;
    window.once("closed", () => {
      this.shots.get(id)?.stop();
      this.shots.delete(id);
    });
    void contents.loadURL(address);
    return { ok: true };
  }

  /** Ends the window's pictures and closes the view they were taken in. */
  stop(sender: Electron.WebContents): void {
    const window = BrowserWindow.fromWebContents(sender);
    const held = window === null ? undefined : this.shots.get(window.id);
    if (window === null || held === undefined) return;
    this.shots.delete(window.id);
    held.stop();
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
