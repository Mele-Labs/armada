// The one walk window: every walk a Job showed and every pull request's walk, queued oldest shown first —
// `docs/practices/capture-window.md`. The owner approved the mock walk `walkQueue`.
//
// **One OS window, one bar and one panel; a page each.** A walk's page is made the first time it comes on screen and
// kept, with its place, until it leaves the window, so going back to it finds it as it was. Only the walk on screen
// draws the bar and lays the views out (`WalkSeat`). A walk that arrives while the window is open joins the queue and
// raises nothing, takes no focus and leaves the page on screen alone.

import { BaseWindow, screen, type WebContentsView } from "electron";

import { CHANNELS } from "../../shared/bridge";
import type { CaptureQueue, CaptureQueued, CaptureQueueKey } from "../../shared/capture-window";
import type { PanelSubject } from "./layout";
import { placeOf, queueKeyOf } from "./queue";
import { barView, panelView } from "./views";
import { isCaptureBinding, type CaptureWindow, type WalkSeat, type WalkSizes } from "./window";

/** One walk in the window. */
export type Queued = {
  key: string;
  /** When it was shown, which places it in the queue. */
  since: string;
  /** The Job it belongs to. Absent: a pull request's walk, which has no Job. */
  sessionId?: string;
  /** The address it was shown at, which the Job's waiting item and its window row are keyed by. */
  url: string;
  /** Makes its page, the first time it comes on screen. */
  make: (seat: WalkSeat) => CaptureWindow;
  walk: CaptureWindow | null;
  /** Its waiting item has been seen on its Job's record, so the item going means the walk was settled or is being worked on. */
  seen: boolean;
  /** Its item went while its window read working: the Job is at work on the notes sent, so the walk stays. */
  kept?: boolean;
  /** Told once, as it leaves the window: approved, or closed with it. Not when it is shown again in its own place. */
  left: () => void;
  /** Its entry on the bar. `onScreen`: it is the walk drawn, which carries no ask mark. */
  describe: (onScreen: boolean) => CaptureQueued;
  /** What its page was made with, so the same walk shown again is told from another. */
  shape: string;
  /** A pull request's held walk, which Bridge closes at Fleet as it quits. */
  held?: { manifestId: string; walkId: string };
};

/** What the window needs from whoever holds it. */
export type WalkWindowBoard = {
  /** The owner approved a walk, as it leaves. */
  approved: (walk: Queued) => void;
  /** A walk came on screen: it is given what it should be showing, a question among it. */
  shown: (walk: Queued) => void;
  focused: (on: boolean) => void;
  /** A walk joined or left. */
  changed: () => void;
  /** The window closed, whoever closed it. Every walk in it has been told it left. */
  closed: () => void;
};

export class WalkWindow {
  private readonly window: BaseWindow;
  private readonly bar: WebContentsView;
  private readonly panel: WebContentsView;
  private readonly seat: WalkSeat;
  private readonly board: WalkWindowBoard;
  private walks: Queued[] = [];
  private current: Queued | null = null;
  /** The queue as the bar was last told it, so a publish that moved nothing in it draws nothing. */
  private drawn = "";

  /** `first` is the Job the panel opens on, where the first walk has one. Shown with `reveal`, never by itself. */
  constructor(sizes: WalkSizes, board: WalkWindowBoard, first: PanelSubject | null) {
    this.board = board;
    this.window = new BaseWindow({
      // The page keeps the width it had before there was a panel, where the screen has room for both.
      width: Math.min(1280 + sizes.panel, screen.getPrimaryDisplay().workAreaSize.width),
      height: 900,
      // The OS frame and its title, where Bridge's own window is frameless and draws its own.
      title: "Walks",
      show: false,
    });
    this.bar = barView();
    this.panel = panelView(first);
    this.window.contentView.addChildView(this.bar);
    this.window.contentView.addChildView(this.panel);
    this.panel.setVisible(false);
    this.seat = {
      window: this.window,
      bar: this.bar,
      panel: this.panel,
      onScreen: (walk) => this.current?.walk === walk,
      queue: () => this.queue(),
      key: (key) => this.key(key),
      settled: (walk) => {
        const settled = this.walks.find((one) => one.walk === walk);
        if (settled === undefined) return;
        this.board.approved(settled);
        this.remove(settled.key);
      },
    };
    this.window.on("resize", () => this.current?.walk?.resized());
    this.window.on("focus", () => this.board.focused(true));
    this.window.on("blur", () => this.board.focused(false));
    this.window.on("closed", () => {
      this.board.focused(false);
      const left = this.walks;
      this.walks = [];
      this.current = null;
      for (const one of left) {
        one.walk?.dispose();
        one.left();
      }
      // A view's document outlives a closed `BaseWindow` unless it is closed with it.
      for (const view of [this.bar, this.panel]) if (!view.webContents.isDestroyed()) view.webContents.close();
      this.board.closed();
    });
    this.bar.webContents.on("did-finish-load", () => this.current?.walk?.redraw());
    // The bar and the panel take the capture binding for the walk on screen, and the queue's keys the bar would take.
    // ⌘↵ stays the bar's own, where a field may hold it as Send, and the panel's, where its composer does.
    for (const [view, own] of [[this.bar, true], [this.panel, false]] as const) {
      view.webContents.on("before-input-event", (event, input) => {
        if (isCaptureBinding(input)) {
          event.preventDefault();
          this.current?.walk?.toggleCapture();
          return;
        }
        const key = queueKeyOf(input);
        if (key === null || key === "approve" || (key === "reload" && !own)) return;
        event.preventDefault();
        this.key(key);
      });
    }
  }

  /** Whether the window is still up. */
  get open(): boolean {
    return !this.window.isDestroyed();
  }

  /** The walks, oldest shown first. */
  get queued(): readonly Queued[] {
    return this.walks;
  }

  /** The walk on screen. */
  get onScreen(): Queued | null {
    return this.current;
  }

  /** Put the window up: **behind, taking no focus**, unless the owner asked for it. */
  reveal(focus: boolean): void {
    if (this.window.isDestroyed()) return;
    if (focus) this.raise();
    else this.window.showInactive();
  }

  raise(): void {
    if (this.window.isDestroyed()) return;
    if (this.window.isMinimized()) this.window.restore();
    this.window.show();
    this.window.focus();
  }

  close(): void {
    if (!this.window.isDestroyed()) this.window.close();
  }

  find(key: string): Queued | undefined {
    return this.walks.find((one) => one.key === key);
  }

  /** A walk joins in its place by when it was shown. **The page on screen stays**, unless `show` or nothing is on screen yet. */
  add(walk: Queued, show: boolean): void {
    if (this.find(walk.key) !== undefined) return;
    const at = placeOf(this.walks, walk.since);
    this.walks = [...this.walks.slice(0, at), walk, ...this.walks.slice(at)];
    if (show || this.current === null) this.bring(walk);
    else this.current.walk?.redraw();
    this.board.changed();
  }

  /** A walk shown again with another page, other takes or a sign-in to run: it keeps its place, and its page is made afresh. */
  replace(walk: Queued): void {
    const standing = this.find(walk.key);
    if (standing === undefined) return this.add(walk, false);
    const fresh: Queued = { ...walk, since: standing.since, seen: standing.seen || walk.seen };
    this.walks = this.walks.map((one) => (one === standing ? fresh : one));
    standing.walk?.dispose();
    if (this.current === standing) {
      this.current = null;
      this.bring(fresh);
    } else this.current?.walk?.redraw();
  }

  /**
   * A walk leaves: approved, or its waiting item gone while its window reads ready. **Its page goes**, and the walk after it comes on screen
   * where it was on screen. The last one closes the window.
   */
  remove(key: string): void {
    const at = this.walks.findIndex((one) => one.key === key);
    const leaving = this.walks[at];
    if (leaving === undefined) return;
    this.walks = this.walks.filter((one) => one !== leaving);
    leaving.walk?.dispose();
    leaving.left();
    this.board.changed();
    if (this.walks.length === 0) {
      this.current = null;
      this.close();
      return;
    }
    if (this.current === leaving) {
      this.current = null;
      // The one that followed it, or the first again where it was the last.
      this.bring(this.walks[at] ?? this.walks[0]!);
    } else this.current?.walk?.redraw();
  }

  /** The owner chose another walk, by its place. */
  jump(at: number): void {
    const walk = this.walks[at];
    if (walk !== undefined && walk !== this.current) this.bring(walk);
  }

  /** Whether an IPC call came from the window's bar: the walk on screen, which every bar call is about. */
  fromBar(contents: Electron.WebContents): CaptureWindow | null {
    if (this.bar.webContents.isDestroyed() || contents.id !== this.bar.webContents.id) return null;
    return this.current?.walk ?? null;
  }

  /** Whether an IPC call came from the window's panel. */
  isPanel(contents: Electron.WebContents): boolean {
    return !this.panel.webContents.isDestroyed() && contents.id === this.panel.webContents.id;
  }

  get panelContents(): Electron.WebContents | null {
    return this.panel.webContents.isDestroyed() ? null : this.panel.webContents;
  }

  /** Draw the bar again for the walk on screen, where something it says about the queue moved since it was last drawn. */
  refresh(): void {
    if (JSON.stringify(this.describe()) !== this.drawn) this.current?.walk?.redraw();
  }

  private describe(): CaptureQueue {
    return { entries: this.walks.map((one) => one.describe(one === this.current)), at: Math.max(0, this.walks.findIndex((one) => one === this.current)) };
  }

  private queue(): CaptureQueue {
    const queue = this.describe();
    this.drawn = JSON.stringify(queue);
    return queue;
  }

  /** A key heard on the page or the panel, handed to the bar. A menu it opens takes the keyboard there. */
  private key(key: CaptureQueueKey): void {
    if (this.bar.webContents.isDestroyed()) return;
    if (key === "open" || key === "takes") this.bar.webContents.focus();
    this.bar.webContents.send(CHANNELS.captureWindowKey, key);
  }

  /** Put a walk on screen, making its page the first time. The panel is told its Job, and draws none for a pull request's. */
  private bring(walk: Queued): void {
    const before = this.current;
    this.current = walk;
    if (walk.walk === null) walk.walk = walk.make(this.seat);
    before?.walk?.hide();
    walk.walk.show();
    if (!this.panel.webContents.isDestroyed()) this.panel.webContents.send(CHANNELS.walkPanelSubjectChanged, walk.sessionId ?? null);
    this.board.shown(walk);
  }
}
