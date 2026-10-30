// Each Job's browser: one page per Job, which its agent drives through `browser.request` and the owner may take over.
// **One view for the Job's life in Bridge**, moved between the side panel and a window of its own, so the page keeps its
// history, its sign-in and what was typed. The page is a `pageView` (`capture/views.ts`): no preload, sandboxed, every
// permission refused bar local network access while the owner drives, in its repository's session. Main places it over the rect the pane reports, as `SessionPages` does.
//
// `docs/practices/protocol/the-job-browser.md` is the wire. What may cross back from the page is the bounded answers there.

import { BrowserWindow, type WebContentsView } from "electron";
import { writeFile } from "node:fs/promises";
import { isAbsolute, join } from "node:path";

import type { AnswerBrowserRequest, BrowserRequest, Outcome, SendSessionNote, StudioCapture } from "@armada/protocol";

import type { JobBrowserState } from "../shared/api/job-browser";
import { pngSize, ringOn, type Ring } from "../shared/ring";
import type { CaptureHeld, CaptureRect } from "../shared/capture-window";
import { webOrigin } from "./capture/address";
import { aimed, bounded, chainOf, readOf, rectOf, type PageRead } from "./capture/bounds";
import { burnRing, type Burn } from "./capture/burn";
import { PageFinder } from "./capture/finder";
import { askSource } from "./capture/layer";
import { sessionNoteBody } from "./capture/pinned-note";
import { reachesLocalNetwork, whileOwnerDrives } from "./capture/permissions";
import { pageView } from "./capture/views";
import { actOn, LOAD_MS, type ActsHost, type JobPage } from "./job-browser-acts";
import { boundsOf } from "./session-page-bounds";

/** The window a pane is drawn in. Opaque here: Electron's `BrowserWindow` in the app, a token in a test. */
export type Host = object;

type Chord = "capture" | "back" | "forward" | "reload";

/** What a page tells its holder. */
export type PageTold = { changed: () => void; chord: (chord: Chord) => void; driving: () => boolean };

/** One Job's page, as `JobBrowsers` moves it. */
export type BrowserPage = JobPage & {
  /** Into `host`'s content at `bounds`, as the renderer reported them, held to the window, out of wherever it was. */
  placeIn(host: Host, bounds: unknown): void;
  /** Put away, where it is in `host`: hidden, and kept. */
  leave(host: Host): void;
  hostOf(): Host | null;
  /** Hidden under Capture, which draws a picture of it instead. */
  hide(hidden: boolean): void;
  go(to: "back" | "forward" | "reload"): void;
  back(): boolean;
  forward(): boolean;
  viewport(): { width: number; height: number };
  /** The page as it is drawn now. */
  picture(): Promise<Picture | null>;
  /** The injected layer's ask at a point of the page (`capture/layer.ts`); `take` false answers only the box and its name. */
  ask(x: number, y: number, take?: boolean): Promise<unknown>;
};

/** The page as drawn, and where a box in its CSS pixels is on that picture. */
export type Picture = { png: Uint8Array; ring(box: CaptureRect): Ring | null };

/** One Job's window, beside its thread. */
export type BrowserWindowHeld = { host: Host; focus(): void; close(): void };

export type JobBrowserMake = {
  page(sessionId: string, told: PageTold): BrowserPage;
  /** `closing`: before the window goes, so its page can be moved out of it. */
  window(sessionId: string, told: { closing: () => void }): BrowserWindowHeld;
};

export type JobBrowserTell = {
  changed(state: JobBrowserState): void;
  /** Bridge's window is asked to show the Job's browser in its side panel. */
  show(sessionId: string): void;
  /** ⌥⌘C reached the page: the document drawing its pane is told. */
  capture(sessionId: string, host: Host): void;
  answer(body: AnswerBrowserRequest): Promise<unknown>;
  note(body: SendSessionNote): Promise<Outcome>;
};

type Held = {
  page: BrowserPage;
  driver: "agent" | "owner";
  window: BrowserWindowHeld | null;
  /** The picture Capture holds still, and the part of it the owner picked. */
  frozen: Picture | null;
  holding: { capture: StudioCapture; read: PageRead } | null;
};

export class JobBrowsers {
  private readonly held = new Map<string, Held>();

  constructor(
    private readonly make: JobBrowserMake,
    private readonly tell: JobBrowserTell,
    private readonly burn: Burn = burnRing,
  ) {}

  list(): JobBrowserState[] {
    return [...this.held.keys()].map((id) => this.stateOf(id)!);
  }

  private stateOf(sessionId: string): JobBrowserState | undefined {
    const one = this.held.get(sessionId);
    if (one === undefined) return undefined;
    return { sessionId, url: one.page.url(), title: one.page.title(), back: one.page.back(), forward: one.page.forward(), driver: one.driver, windowed: one.window !== null };
  }

  private told(sessionId: string): void {
    const state = this.stateOf(sessionId);
    if (state !== undefined) this.tell.changed(state);
  }

  private ensure(sessionId: string): Held {
    const standing = this.held.get(sessionId);
    if (standing !== undefined) return standing;
    const page = this.make.page(sessionId, {
      changed: () => this.told(sessionId),
      driving: () => this.held.get(sessionId)?.driver === "owner",
      chord: (chord) => {
        const host = page.hostOf();
        if (chord === "capture") {
          if (host !== null) this.tell.capture(sessionId, host);
        } else if (this.held.get(sessionId)?.driver === "owner") page.go(chord);
      },
    });
    const one: Held = { page, driver: "agent", window: null, frozen: null, holding: null };
    this.held.set(sessionId, one);
    return one;
  }

  /** In Bridge's side panel: made where the Job has none, and moved out of its window where it is in one. */
  show(sessionId: string): void {
    const one = this.ensure(sessionId);
    const window = one.window;
    one.window = null;
    window?.close();
    this.told(sessionId);
    this.tell.show(sessionId);
  }

  /** In a window of its own beside the Job's thread. The side panel it left closes itself. */
  toWindow(sessionId: string): void {
    const one = this.held.get(sessionId);
    if (one === undefined) return;
    if (one.window !== null) {
      one.window.focus();
      return;
    }
    const window = this.make.window(sessionId, {
      closing: () => {
        one.page.leave(window.host);
        if (one.window !== window) return;
        one.window = null;
        this.told(sessionId);
      },
    });
    one.window = window;
    this.told(sessionId);
  }

  place(host: Host, sessionId: string, bounds: unknown): void {
    this.held.get(sessionId)?.page.placeIn(host, bounds);
  }

  unplace(host: Host, sessionId: string): void {
    this.held.get(sessionId)?.page.leave(host);
  }

  /** Take over, or hand back to the agent. */
  drive(sessionId: string): void {
    const one = this.held.get(sessionId);
    if (one === undefined) return;
    one.driver = one.driver === "agent" ? "owner" : "agent";
    this.told(sessionId);
  }

  go(sessionId: string, to: "back" | "forward" | "reload"): void {
    this.held.get(sessionId)?.page.go(to);
  }

  /** An address the owner typed. Only while he drives, as the pane offers it, and only the web. */
  navigate(sessionId: string, url: string): void {
    const one = this.held.get(sessionId);
    if (one === undefined || one.driver !== "owner" || webOrigin(url) === null) return;
    void one.page.load(url, LOAD_MS);
  }

  /** The agent's act, answered through Fleet whatever becomes of it. */
  request(request: BrowserRequest): Promise<void> {
    return actOn(this.acts, request);
  }

  private readonly acts: ActsHost = {
    page: (sessionId) => this.held.get(sessionId)?.page ?? null,
    ownerDriving: (sessionId) => this.held.get(sessionId)?.driver === "owner",
    openClosedInPanel: (sessionId) => {
      this.show(sessionId);
      return this.held.get(sessionId)!.page;
    },
    answer: (body) => this.tell.answer(body),
  };

  /** Capture holds the page still: its picture, which the pane draws in its place, and the page hidden under it. */
  async freeze(sessionId: string): Promise<Uint8Array | null> {
    const one = this.held.get(sessionId);
    if (one === undefined) return null;
    const picture = await one.page.picture();
    if (picture === null) return null;
    one.frozen = picture;
    one.holding = null;
    one.page.hide(true);
    return picture.png;
  }

  thaw(sessionId: string): void {
    const one = this.held.get(sessionId);
    if (one === undefined) return;
    one.frozen = null;
    one.holding = null;
    one.page.hide(false);
  }

  /** The part of the page at a point of its picture, as the walk window's layer reads it. The pane gets the box and one line. */
  async hold(sessionId: string, x: number, y: number): Promise<CaptureHeld | null> {
    const one = this.held.get(sessionId);
    if (one === undefined || one.frozen === null) return null;
    const viewport = one.page.viewport();
    const answer = await one.page.ask(x, y);
    const capture = bounded(answer, viewport, { run: "", name: one.page.title(), address: webOrigin(one.page.url()) ?? one.page.url() });
    if (capture === null) return null;
    one.holding = { capture, read: readOf(answer) };
    return { box: rectOf(capture.bounds, viewport), chain: chainOf(capture) };
  }

  /** The box of the part at a point of the picture, in the picture's own pixels (the page's CSS pixels): the pane's hover ring. */
  async aim(sessionId: string, x: number, y: number): Promise<CaptureRect | null> {
    const one = this.held.get(sessionId);
    if (one === undefined || one.frozen === null) return null;
    return aimed(await one.page.ask(x, y, false), one.page.viewport())?.box ?? null;
  }

  /**
   * The held part's note, sent to the Job as the owner's: `send_session_note`. Its picture is the whole frozen page with the
   * part ringed, tagged 1, since it goes as a turn of its own.
   */
  async note(sessionId: string, said: string): Promise<Outcome> {
    const one = this.held.get(sessionId);
    const holding = one?.holding;
    if (one === undefined || holding === null || holding === undefined) return { ok: false, why: "nothing_held" };
    const frozen = one.frozen;
    const ring = frozen === null ? null : frozen.ring(rectOf(holding.capture.bounds, one.page.viewport()));
    const crop = frozen === null || ring === null ? null : Buffer.from(this.burn(frozen.png, ring, 1)).toString("base64");
    const body = sessionNoteBody({ sessionId, said, capture: holding.capture, read: holding.read, address: one.page.url(), walk: null, crop });
    const outcome = await this.tell.note(body);
    if (outcome.ok) this.thaw(sessionId);
    return outcome;
  }
}

// Electron's half.

/** An expression an agent runs: a world of its own, apart from the finder's (2000), so it cannot reach `ArmadaFinder`. */
const EVALUATE_WORLD = 2001;
/** Past Fleet's cuts (48 KiB and 32 KiB), so Fleet still sees an answer that was cut and says so. */
const MOST_SNAPSHOT = 64 * 1024;
const MOST_RESULT = 64 * 1024;
const SETTLE_MS = 300;
// ⌥⌘A starts Capture as ⌥⌘C does; the app's annotation layer keeps it while no Job browser is on screen (`chord.ts` in the pane).
const CHORDS: Record<string, Chord> = { "alt+meta+KeyC": "capture", "alt+meta+KeyA": "capture", "meta+BracketLeft": "back", "meta+BracketRight": "forward", "meta+KeyR": "reload" };

const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

/** The source an agent's expression runs as: awaited, and its value written by `JSON.stringify` inside the page's world. */
export function evaluateSource(expression: string): string {
  return `(async () => { const value = await (\n${expression}\n); const said = JSON.stringify(value); return said === undefined ? undefined : said.slice(0, ${MOST_RESULT}); })()`;
}

/**
 * A Job's page in Electron. **Put away in Bridge's window, hidden**, while no pane shows it, so an act still reaches a page
 * that is laid out at its last size; `parkIn` is that window, or null while none is up.
 */
export function makeBrowserPage(manifestId: string | null, parkIn: () => BrowserWindow | null, told: PageTold): BrowserPage {
  const view: WebContentsView = pageView({ run: "", name: "", origin: "", url: "about:blank", manifestId, through: [] });
  const contents = view.webContents;
  // The pin names no origin, so `pageView` gave it no local network access: a sign-in app on this machine is reachable only by the owner's hand.
  reachesLocalNetwork(contents, whileOwnerDrives(told.driving, () => contents.getURL()));
  // Its acts wait on frames, which a hidden page would otherwise stop drawing.
  contents.setBackgroundThrottling(false);
  // A page that opens another opens it here, and only on the web.
  contents.setWindowOpenHandler(({ url }) => {
    if (webOrigin(url) !== null) void contents.loadURL(url).catch(() => undefined);
    return { action: "deny" };
  });
  contents.on("will-navigate", (event, url) => {
    if (webOrigin(url) === null) event.preventDefault();
  });
  contents.on("before-input-event", (event, input) => {
    if (input.type !== "keyDown") return;
    const chord = CHORDS[`${input.alt ? "alt+" : ""}${input.meta ? "meta+" : ""}${input.code}`];
    if (chord === undefined) return;
    event.preventDefault();
    told.chord(chord);
  });
  contents.on("did-navigate", () => told.changed());
  contents.on("did-navigate-in-page", () => told.changed());
  contents.on("page-title-updated", () => told.changed());
  const finder = new PageFinder(
    () => (contents.isDestroyed() ? null : contents),
    () => viewportOf(),
  );
  let parent: BrowserWindow | null = null;
  let shown = false;
  let hidden = false;
  const viewportOf = () => {
    const box = view.getBounds();
    return { width: Math.max(1, box.width), height: Math.max(1, box.height) };
  };
  const into = (window: BrowserWindow | null): void => {
    if (parent === window) return;
    if (parent !== null && !parent.isDestroyed()) parent.contentView.removeChildView(view);
    parent = window;
    if (window !== null && !window.isDestroyed()) window.contentView.addChildView(view);
  };
  view.setVisible(false);
  into(parkIn());
  return {
    // A Job browser made empty shows no address until a page is opened in it.
    url: () => (contents.isDestroyed() || contents.getURL() === "about:blank" ? "" : contents.getURL()),
    title: () => (contents.isDestroyed() ? "" : contents.getTitle()),
    load: (url, patience) =>
      new Promise((resolve) => {
        const finish = (said: { loaded: true } | { failed: string } | { timedOut: true }) => {
          clearTimeout(giveUp);
          contents.off("did-finish-load", onLoaded);
          contents.off("did-fail-load", onFailed);
          resolve(said);
        };
        const onLoaded = () => finish({ loaded: true });
        // An aborted load (-3) is one a redirect or the next load replaced; the load that replaced it is waited for.
        const onFailed = (_event: unknown, code: number, description: string, _url: string, mainFrame: boolean) => {
          if (mainFrame && code !== -3) finish({ failed: description });
        };
        const giveUp = setTimeout(() => finish({ timedOut: true }), patience);
        contents.on("did-finish-load", onLoaded);
        contents.on("did-fail-load", onFailed);
        void contents.loadURL(url).catch(() => undefined);
      }),
    find: (target, patience) => finder.arrive(target, patience),
    focus: async () => {
      const kind = await finder.focus();
      // Keys reach the focused contents.
      if (!contents.isDestroyed()) contents.focus();
      return kind;
    },
    send: (sent) => {
      if (contents.isDestroyed()) return;
      if ("insert" in sent) void contents.insertText(sent.insert);
      else contents.sendInputEvent(sent);
    },
    selectAll: () => contents.selectAll(),
    settle: () => sleep(SETTLE_MS),
    snapshot: () => finder.snapshot(MOST_SNAPSHOT),
    capture: async (path) => {
      if (!isAbsolute(path) || !path.endsWith(".png") || contents.isDestroyed()) return false;
      const image = await contents.capturePage();
      if (image.isEmpty()) return false;
      await writeFile(path, image.toPNG());
      return true;
    },
    evaluate: async (expression) => {
      try {
        const said: unknown = await contents.executeJavaScriptInIsolatedWorld(EVALUATE_WORLD, [{ code: evaluateSource(expression) }]);
        return typeof said === "string" ? { result: said.slice(0, MOST_RESULT) } : {};
      } catch (error) {
        return { threw: error instanceof Error ? error.message : String(error) };
      }
    },
    placeIn: (host, bounds) => {
      const window = host as BrowserWindow;
      if (window.isDestroyed()) return;
      into(window);
      shown = true;
      view.setVisible(!hidden);
      view.setBounds(boundsOf(bounds, window.getContentBounds()));
    },
    leave: (host) => {
      if (parent !== host) return;
      shown = false;
      view.setVisible(false);
      const park = parkIn();
      into(park !== null && !park.isDestroyed() ? park : null);
    },
    hostOf: () => (shown ? parent : null),
    hide: (on) => {
      hidden = on;
      view.setVisible(shown && !on);
      // A hidden page keeps the keyboard, and ⌥⌘A that closes the annotation layer would go to it.
      if (on && parent !== null && !parent.isDestroyed()) parent.webContents.focus();
    },
    go: (to) => {
      if (contents.isDestroyed()) return;
      if (to === "reload") contents.reload();
      else if (to === "back" && contents.navigationHistory.canGoBack()) contents.navigationHistory.goBack();
      else if (to === "forward" && contents.navigationHistory.canGoForward()) contents.navigationHistory.goForward();
    },
    back: () => !contents.isDestroyed() && contents.navigationHistory.canGoBack(),
    forward: () => !contents.isDestroyed() && contents.navigationHistory.canGoForward(),
    viewport: viewportOf,
    picture: async () => {
      if (contents.isDestroyed()) return null;
      const image = await contents.capturePage();
      if (image.isEmpty()) return null;
      const png = image.toPNG();
      const width = pngSize(png)?.width ?? image.getSize().width;
      return { png, ring: (box) => (box.width === 0 || box.height === 0 ? null : ringOn(box, viewportOf().width, width)) };
    },
    ask: async (x, y, take = true) => {
      if (contents.isDestroyed()) return null;
      try {
        return await contents.executeJavaScript(askSource(x, y, take), false);
      } catch {
        return null;
      }
    },
  };
}

/** A Job's browser in a window of its own: a Bridge document with Bridge's preload, which goes nowhere, as the terminal's window is. */
export function makeBrowserWindow(sessionId: string, over: { x: number; y: number; width: number; height: number } | null, told: { closing: () => void }): BrowserWindowHeld {
  const size = { width: 1280, height: 820 };
  const window = new BrowserWindow({
    ...size,
    ...(over === null ? {} : { x: Math.round(over.x + (over.width - size.width) / 2), y: Math.round(over.y + (over.height - size.height) / 2) }),
    show: false,
    minWidth: 720,
    minHeight: 420,
    webPreferences: { preload: join(__dirname, "../preload/index.js"), sandbox: true, contextIsolation: true, nodeIntegration: false },
  });
  window.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
  window.webContents.on("will-navigate", (event) => event.preventDefault());
  window.on("ready-to-show", () => window.show());
  window.on("close", told.closing);
  void window.loadFile(join(__dirname, "../renderer/browser-window.html"), { query: { session: sessionId } });
  return {
    host: window,
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
}
