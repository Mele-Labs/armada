// Each Job's browser: one page per Browser tab of the Job's panes, keyed by the Job and the tab. The Job's agent drives the Job's page
// (`jobsPage`) through `browser.request` and the owner may take any over.
// **One view for the tab's life**, put over its pane and moved to a window of its own and back, hidden and not destroyed while another tab is in
// front of it, so the page keeps its history, its sign-in and what was typed; it is destroyed with its tab, or with its Job. The page is a
// `pageView` (`capture/views.ts`): no preload, sandboxed, every permission refused bar local network access while the owner drives, in its
// repository's session. Main places it over the rect the pane reports, as `SessionPages` does.
//
// `docs/practices/protocol/the-job-browser.md` is the wire. What may cross back from the page is the bounded answers there.

import { BrowserWindow, type WebContentsView } from "electron";
import { writeFile } from "node:fs/promises";
import { isAbsolute, join } from "node:path";

import type { AnswerBrowserRequest, BrowserRequest, Outcome, SendSessionNote, StudioCapture } from "@armada/protocol";

import type { JobBrowserState } from "../shared/api/job-browser";
import type { Answer, Choice, SitePermission } from "../shared/site-permissions";
import { pngSize, ringOn, type Ring } from "../shared/ring";
import type { CaptureHeld, CaptureRect } from "../shared/capture-window";
import { webOrigin } from "./capture/address";
import { aimed, bounded, chainOf, readOf, rectOf, type PageRead } from "./capture/bounds";
import { burnRing, type Burn } from "./capture/burn";
import { PageFinder } from "./capture/finder";
import { askSource } from "./capture/layer";
import { sessionNoteBody } from "./capture/pinned-note";
import { decidesPermissions } from "./capture/permissions";
import { check, decide, downloadOf, hostOf, permissionsOf, type Rules } from "./capture/site-permissions";
import { pageView } from "./capture/views";
import { actOn, LOAD_MS, type ActsHost, type JobPage } from "./job-browser-acts";
import { boundsOf } from "./session-page-bounds";
import type { SiteSettingsStore } from "./site-permission-store";

/** The window a pane is drawn in. Opaque here: Electron's `BrowserWindow` in the app, a token in a test. */
export type Host = object;

type Chord = "capture" | "back" | "forward" | "reload";

/** What a page tells its holder. */
export type PageTold = {
  changed: () => void;
  chord: (chord: Chord) => void;
  driving: () => boolean;
  /** Asks the owner; resolves when he answers, or `dismiss` where one is already waiting. */
  ask: (permission: SitePermission, host: string) => Promise<Answer>;
};

/** One tab's page, as `JobBrowsers` moves it. */
export type BrowserPage = JobPage & {
  /** Into `host`'s content at `bounds`, as the renderer reported them, held to the window, out of wherever it was. */
  placeIn(host: Host, bounds: unknown): void;
  /** Put away, where it is in `host`: hidden, and kept. */
  leave(host: Host): void;
  hostOf(): Host | null;
  /** Taken out of its window and closed, for good: its tab is gone. */
  destroy(): void;
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
  /** What this page's repository has decided for each site, without the request waiting. */
  settings(): { sites: JobBrowserState["site"]["sites"]; defaults: JobBrowserState["site"]["defaults"] };
  choose(host: string, permission: SitePermission, choice: Choice): void;
  setDefault(permission: SitePermission, choice: Choice): void;
};

/** The page as drawn, and where a box in its CSS pixels is on that picture. */
export type Picture = { png: Uint8Array; ring(box: CaptureRect): Ring | null };

/** One tab's window, beside its Job's thread. */
export type BrowserWindowHeld = { host: Host; focus(): void; close(): void };

export type JobBrowserMake = {
  page(sessionId: string, told: PageTold): BrowserPage;
  /** `closing`: before the window goes, so its page can be moved out of it. */
  window(sessionId: string, tabId: string, told: { closing: () => void }): BrowserWindowHeld;
};

export type JobBrowserTell = {
  changed(state: JobBrowserState): void;
  /** Bridge's window is asked to open a Browser tab for the Job, since its agent opened a page where it has none. */
  show(sessionId: string): void;
  /** ⌥⌘C reached the page: the document drawing its pane is told. */
  capture(sessionId: string, tabId: string, host: Host): void;
  answer(body: AnswerBrowserRequest): Promise<unknown>;
  note(body: SendSessionNote): Promise<Outcome>;
};

type Held = {
  /** The tab this page belongs to. `UNATTACHED` until the first Browser tab is there to take the page an agent opened. */
  tabId: string;
  page: BrowserPage;
  driver: "agent" | "owner";
  window: BrowserWindowHeld | null;
  /** The picture Capture holds still, and the part of it the owner picked. */
  frozen: Picture | null;
  holding: { capture: StudioCapture; read: PageRead } | null;
  /** The page's request waiting on the owner. */
  asking: { permission: SitePermission; host: string; settle(answer: Answer): void } | null;
};

/** The tab id of the page an agent opened for a Job that had no Browser tab. The first tab the renderer reports takes it over. */
const UNATTACHED = "";

/**
 * **The Job's page**, the one its agent's acts reach: the first Browser tab in pane order that is shown, else the one shown last, else the first.
 * `tabs` is in pane order. The agent follows what the owner is looking at, and keeps a page of its own while he looks at none.
 */
export function jobsPage(tabs: readonly { id: string; shown: boolean }[], lastShown: string | undefined): string | undefined {
  return (tabs.find((one) => one.shown) ?? tabs.find((one) => one.id === lastShown) ?? tabs[0])?.id;
}

export class JobBrowsers {
  /** Each Job's pages by tab. */
  private readonly held = new Map<string, Map<string, Held>>();
  /** The Browser tabs of each Job in pane order, as the renderer last reported them. */
  private readonly order = new Map<string, readonly string[]>();
  /** The tab of each Job that was shown last. */
  private readonly lastShown = new Map<string, string>();

  constructor(
    private readonly make: JobBrowserMake,
    private readonly tell: JobBrowserTell,
    private readonly burn: Burn = burnRing,
  ) {}

  list(): JobBrowserState[] {
    return [...this.held.entries()].flatMap(([sessionId, tabs]) => [...tabs.keys()].map((tabId) => this.stateOf(sessionId, tabId)!));
  }

  private one(sessionId: string, tabId: string): Held | undefined {
    return this.held.get(sessionId)?.get(tabId);
  }

  private stateOf(sessionId: string, tabId: string): JobBrowserState | undefined {
    const one = this.one(sessionId, tabId);
    if (one === undefined) return undefined;
    return {
      sessionId,
      tabId,
      url: one.page.url(),
      title: one.page.title(), back: one.page.back(),
      forward: one.page.forward(),
      driver: one.driver,
      windowed: one.window !== null,
      site: { ...one.page.settings(), pending: one.asking === null ? null : { permission: one.asking.permission, host: one.asking.host } },
    };
  }

  private told(sessionId: string, tabId: string): void {
    const state = this.stateOf(sessionId, tabId);
    if (state !== undefined) this.tell.changed(state);
  }

  private ensure(sessionId: string, tabId: string): Held {
    const standing = this.one(sessionId, tabId);
    if (standing !== undefined) return standing;
    // The tab is read off `one` and not closed over, since the first tab takes over the page an agent opened (`tabs`).
    const page = this.make.page(sessionId, {
      changed: () => this.told(sessionId, one.tabId),
      driving: () => one.driver === "owner",
      ask: (permission, host) => {
        // One question at a time: a second is refused now, as a dismissed one.
        if (this.one(sessionId, one.tabId) !== one || one.asking !== null) return Promise.resolve<Answer>("dismiss");
        return new Promise<Answer>((settle) => {
          one.asking = { permission, host, settle };
          this.told(sessionId, one.tabId);
        });
      },
      chord: (chord) => {
        const host = page.hostOf();
        if (chord === "capture") {
          if (host !== null) this.tell.capture(sessionId, one.tabId, host);
        } else if (one.driver === "owner") page.go(chord);
      },
    });
    const one: Held = { tabId, page, driver: "agent", window: null, frozen: null, holding: null, asking: null };
    this.held.set(sessionId, (this.held.get(sessionId) ?? new Map()).set(tabId, one));
    return one;
  }

  /** The tab's page is gone: its window closed, its question dismissed, its view destroyed. */
  private drop(sessionId: string, tabId: string): void {
    const one = this.one(sessionId, tabId);
    if (one === undefined) return;
    this.held.get(sessionId)!.delete(tabId);
    this.settle(one, "dismiss");
    const window = one.window;
    one.window = null;
    window?.close();
    one.page.destroy();
    if (this.lastShown.get(sessionId) === tabId) this.lastShown.delete(sessionId);
  }

  /**
   * The Browser tabs a Job has, in pane order, as its panes say them: each has a page, and a page whose tab is not among them is destroyed.
   * The renderer says them as the panes change, since a tab closed while another is in front draws nothing to say so.
   */
  tabs(sessionId: string, tabIds: readonly string[]): void {
    this.order.set(sessionId, tabIds);
    for (const tabId of [...(this.held.get(sessionId)?.keys() ?? [])]) if (tabId !== UNATTACHED && !tabIds.includes(tabId)) this.drop(sessionId, tabId);
    const mine = this.held.get(sessionId);
    const opened = mine?.get(UNATTACHED);
    const first = tabIds.find((id) => mine?.has(id) !== true);
    if (mine !== undefined && opened !== undefined && first !== undefined) {
      mine.delete(UNATTACHED);
      opened.tabId = first;
      mine.set(first, opened);
      this.told(sessionId, first);
    }
    for (const tabId of tabIds) {
      if (this.one(sessionId, tabId) !== undefined) continue;
      this.ensure(sessionId, tabId);
      // Its pane has nothing to draw until it hears the page is there.
      this.told(sessionId, tabId);
    }
  }

  /** A Job is gone: every page it had is destroyed. */
  forget(sessionId: string): void {
    for (const tabId of [...(this.held.get(sessionId)?.keys() ?? [])]) this.drop(sessionId, tabId);
    this.held.delete(sessionId);
    this.order.delete(sessionId);
    this.lastShown.delete(sessionId);
  }

  /** Only these Jobs are alive: the pages of any other are destroyed. */
  keep(alive: ReadonlySet<string>): void {
    for (const sessionId of [...this.held.keys()]) if (!alive.has(sessionId)) this.forget(sessionId);
  }

  /** Back in its pane: out of the window it was in, which closes. The pane places the page again. */
  show(sessionId: string, tabId: string): void {
    const one = this.one(sessionId, tabId);
    if (one === undefined) return;
    const window = one.window;
    one.window = null;
    window?.close();
    this.told(sessionId, tabId);
  }

  /** In a window of its own beside the Job's thread. The pane it left draws nothing of the page. */
  toWindow(sessionId: string, tabId: string): void {
    const one = this.one(sessionId, tabId);
    if (one === undefined) return;
    if (one.window !== null) {
      one.window.focus();
      return;
    }
    const window = this.make.window(sessionId, tabId, {
      closing: () => {
        one.page.leave(window.host);
        if (one.window !== window) return;
        one.window = null;
        this.told(sessionId, one.tabId);
      },
    });
    one.window = window;
    this.told(sessionId, tabId);
  }

  place(host: Host, sessionId: string, tabId: string, bounds: unknown): void {
    const one = this.one(sessionId, tabId);
    if (one === undefined) return;
    if (one.page.hostOf() === null) this.lastShown.set(sessionId, tabId);
    one.page.placeIn(host, bounds);
  }

  unplace(host: Host, sessionId: string, tabId: string): void {
    this.one(sessionId, tabId)?.page.leave(host);
  }

  /** Take over, or hand back to the agent. */
  drive(sessionId: string, tabId: string): void {
    const one = this.one(sessionId, tabId);
    if (one === undefined) return;
    one.driver = one.driver === "agent" ? "owner" : "agent";
    this.settle(one, "dismiss");
    this.told(sessionId, tabId);
  }

  private settle(one: Held, answer: Answer): void {
    const asking = one.asking;
    one.asking = null;
    asking?.settle(answer);
  }

  /** The owner's answer to the waiting request. */
  answer(sessionId: string, tabId: string, answer: Answer): void {
    const one = this.one(sessionId, tabId);
    if (one === undefined || one.asking === null) return;
    this.settle(one, answer);
    this.told(sessionId, tabId);
  }

  choose(sessionId: string, tabId: string, host: string, permission: SitePermission, choice: Choice): void {
    this.one(sessionId, tabId)?.page.choose(host, permission, choice);
    this.told(sessionId, tabId);
  }

  setDefault(sessionId: string, tabId: string, permission: SitePermission, choice: Choice): void {
    this.one(sessionId, tabId)?.page.setDefault(permission, choice);
    this.told(sessionId, tabId);
  }

  go(sessionId: string, tabId: string, to: "back" | "forward" | "reload"): void {
    this.one(sessionId, tabId)?.page.go(to);
  }

  /** An address the owner typed. Only while he drives, as the pane offers it, and only the web. */
  navigate(sessionId: string, tabId: string, url: string): void {
    const one = this.one(sessionId, tabId);
    if (one === undefined || one.driver !== "owner" || webOrigin(url) === null) return;
    void one.page.load(url, LOAD_MS);
  }

  /** The agent's act, answered through Fleet whatever becomes of it. */
  request(request: BrowserRequest): Promise<void> {
    return actOn(this.acts, request);
  }

  /** The Job's page: the tab `jobsPage` names. */
  private jobsHeld(sessionId: string): Held | undefined {
    const mine = this.held.get(sessionId);
    if (mine === undefined) return undefined;
    const tabs = [...(this.order.get(sessionId) ?? []), UNATTACHED].filter((id) => mine.has(id));
    const id = jobsPage(
      tabs.map((one) => ({ id: one, shown: mine.get(one)!.page.hostOf() !== null })),
      this.lastShown.get(sessionId),
    );
    return id === undefined ? undefined : mine.get(id);
  }

  private readonly acts: ActsHost = {
    page: (sessionId) => this.jobsHeld(sessionId)?.page ?? null,
    ownerDriving: (sessionId) => this.jobsHeld(sessionId)?.driver === "owner",
    openClosed: (sessionId) => {
      const one = this.ensure(sessionId, UNATTACHED);
      this.told(sessionId, UNATTACHED);
      this.tell.show(sessionId);
      return one.page;
    },
    answer: (body) => this.tell.answer(body),
  };

  /** Capture holds the page still: its picture, which the pane draws in its place, and the page hidden under it. */
  async freeze(sessionId: string, tabId: string): Promise<Uint8Array | null> {
    const one = this.one(sessionId, tabId);
    if (one === undefined) return null;
    const picture = await one.page.picture();
    if (picture === null) return null;
    one.frozen = picture;
    one.holding = null;
    one.page.hide(true);
    return picture.png;
  }

  thaw(sessionId: string, tabId: string): void {
    const one = this.one(sessionId, tabId);
    if (one === undefined) return;
    one.frozen = null;
    one.holding = null;
    one.page.hide(false);
  }

  /** The part of the page at a point of its picture, as the walk window's layer reads it. The pane gets the box and one line. */
  async hold(sessionId: string, tabId: string, x: number, y: number): Promise<CaptureHeld | null> {
    const one = this.one(sessionId, tabId);
    if (one === undefined || one.frozen === null) return null;
    const viewport = one.page.viewport();
    const answer = await one.page.ask(x, y);
    const capture = bounded(answer, viewport, { run: "", name: one.page.title(), address: webOrigin(one.page.url()) ?? one.page.url() });
    if (capture === null) return null;
    one.holding = { capture, read: readOf(answer) };
    return { box: rectOf(capture.bounds, viewport), chain: chainOf(capture) };
  }

  /** The box of the part at a point of the picture, in the picture's own pixels (the page's CSS pixels): the pane's hover ring. */
  async aim(sessionId: string, tabId: string, x: number, y: number): Promise<CaptureRect | null> {
    const one = this.one(sessionId, tabId);
    if (one === undefined || one.frozen === null) return null;
    return aimed(await one.page.ask(x, y, false), one.page.viewport())?.box ?? null;
  }

  /**
   * The held part's note, sent to the Job as the owner's: `send_session_note`. Its picture is the whole frozen page with the
   * part ringed, tagged 1, since it goes as a turn of its own.
   */
  async note(sessionId: string, tabId: string, said: string): Promise<Outcome> {
    const one = this.one(sessionId, tabId);
    const holding = one?.holding;
    if (one === undefined || holding === null || holding === undefined) return { ok: false, why: "nothing_held" };
    const frozen = one.frozen;
    const ring = frozen === null ? null : frozen.ring(rectOf(holding.capture.bounds, one.page.viewport()));
    const crop = frozen === null || ring === null ? null : Buffer.from(this.burn(frozen.png, ring, 1)).toString("base64");
    const body = sessionNoteBody({ sessionId, said, capture: holding.capture, read: holding.read, address: one.page.url(), walk: null, crop });
    const outcome = await this.tell.note(body);
    if (outcome.ok) this.thaw(sessionId, tabId);
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
 * A Browser tab's page in Electron. **Put away in Bridge's window, hidden**, while no pane shows it, so an act still reaches a page
 * that is laid out at its last size; `parkIn` is that window, or null while none is up.
 */
export function makeBrowserPage(manifestId: string | null, parkIn: () => BrowserWindow | null, told: PageTold, store: SiteSettingsStore): BrowserPage {
  const view: WebContentsView = pageView({ run: "", name: "", origin: "", url: "about:blank", manifestId, through: [] });
  const contents = view.webContents;
  const key = manifestId ?? "none";
  // **Only while the owner drives**, from the choices kept for this repository; a question goes to the owner, and an unparsable host is refused.
  const rules: Rules = {
    settings: () => ({ ...store.settingsOf(key), pending: null }),
    driving: told.driving,
    ask: told.ask,
    remember: (host, permission, choice) => {
      store.choose(key, host, permission, choice);
      told.changed();
    },
  };
  decidesPermissions(contents, {
    request: async (permission, requestingUrl, details) => {
      const wants = permissionsOf(permission, details);
      const host = hostOf(requestingUrl);
      return wants !== null && host !== null && decide(wants, host, rules);
    },
    check: (permission, requestingOrigin, details) => {
      const wants = permissionsOf(permission, details);
      const host = hostOf(requestingOrigin);
      return wants !== null && host !== null && check(wants, host, rules);
    },
    download: (event, url, redownload) => {
      const host = hostOf(url);
      const choice = host === null ? "block" : downloadOf(host, rules);
      if (choice === "allow") return;
      event.preventDefault();
      if (choice !== "ask" || host === null) return;
      void told.ask("downloads", host).then((answer) => {
        if (answer === "dismiss") return;
        rules.remember(host, "downloads", answer);
        if (answer === "allow") redownload();
      });
    },
  });
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
    destroy: () => {
      shown = false;
      into(null);
      if (!contents.isDestroyed()) contents.close();
    },
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
    settings: () => store.settingsOf(key),
    choose: (host, permission, choice) => store.choose(key, host, permission, choice),
    setDefault: (permission, choice) => store.setDefault(key, permission, choice),
  };
}

/** A Browser tab's page in a window of its own: a Bridge document with Bridge's preload, which goes nowhere, as the terminal's window is. */
export function makeBrowserWindow(sessionId: string, tabId: string, over: { x: number; y: number; width: number; height: number } | null, told: { closing: () => void }): BrowserWindowHeld {
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
  void window.loadFile(join(__dirname, "../renderer/browser-window.html"), { query: { session: sessionId, tab: tabId } });
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
