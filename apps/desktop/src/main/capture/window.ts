// The capture window — #1294, `docs/practices/capture-window.md`.
//
// **Three views in one frame.** The page runs in a view with no preload, in a
// persistent session of its own; Bridge's bar runs above it, and a Session's or
// a Job's panel beside it, each a view with Bridge's preload and Bridge's CSP.
// While capture is armed the bar's view grows over the page, so the outline is
// drawn where the page cannot read it, restyle it or forge it, and a press never
// reaches the app. `views.ts` builds the three; `layout.ts` places them.
//
// Bridge's own window is untouched: it still loads one file and refuses every
// navigation — `bridge.md`, *Security posture*.

import { BaseWindow, screen, shell, type WebContentsView } from "electron";

import type { Outcome, StagedFrame, StudioCapture, WalkStep, WindowPlace, WindowTake } from "@armada/protocol";
import type {
  CaptureAimed,
  CaptureAnswer,
  CaptureRect,
  CaptureHeld,
  CaptureRefusedWhat,
  CaptureWheel,
  CaptureReadiness,
  CaptureWalk,
  CaptureWindowState,
} from "../../shared/capture-window";
import { CHANNELS } from "../../shared/bridge";
import { isPinned, offerable, onOrigin, shown, webOrigin, type Pinned } from "./address";
import { aimed, bounded, chainOf, rectOf } from "./bounds";
import { panelFor, walkLayout, type LandsOn, type PanelSubject } from "./layout";
import { FailedLoads, lookSource, looked } from "./failed";
import { followedBy } from "./follow";
import type { Take } from "./session-note";
import { askSource } from "./layer";
import { locateSource } from "./locate";
import { PageFinder } from "./finder";
import { StepWalk } from "./stepwalk";
import { actSource, hostSource, readSource, readWalk, sameWalk, type WalkAct } from "./walk";
import { barView, pageView, panelView } from "./views";

export type { LandsOn } from "./layout";

/** What the window needs from main: a Note lands, and a frame is staged. */
export type CaptureBoard = {
  capture: (studioId: string, said: string, capture: StudioCapture, frame: StagedFrame | null) => Promise<Outcome>;
  /** A note kept on the Job whose server this window walks — protocol 23.18. */
  walkNote: (jobId: string, said: string, capture: StudioCapture, frame: StagedFrame | null) => Promise<Outcome>;
  /** A note taken on a page a Session showed: a message from the person, which wakes the Session. */
  sessionNote: (sessionId: string, said: string, capture: StudioCapture, address: string, take?: Take) => Promise<Outcome>;
  /** Approve on the bar, for a Job: the act JobDetail's own Approve sends. */
  walkApproved: (jobId: string) => Promise<Outcome>;
  /** Approve on the bar, for a Session: a message from the person naming the address, and the take on screen, which wakes it. */
  sessionApproved: (sessionId: string, address: string, take?: Take) => Promise<Outcome>;
  /** The Session's question, answered from the bar: `answer_waiting` on its item, which wakes it. */
  answerAsk: (sessionId: string, itemId: string, answer: CaptureAnswer) => Promise<Outcome>;
  /** The Session windows open changed: one opened, or one closed. Optional, for a board that draws none. */
  windowsChanged?: () => void;
  /** A Session's window closed, whoever closed it: its walk is over, so the Session may ask again. */
  sessionClosed: (sessionId: string, address: string) => void;
  /** A Session's window could not show its page: Fleet tells the Session. */
  sessionFailed: (sessionId: string, address: string, why: string) => void;
  /** A Job's walk window could not show its work: Fleet wakes its Commander. */
  walkFailed: (jobId: string, address: string, why: string) => void;
  stage: (png: Buffer, width: number, height: number) => Promise<StagedFrame | null>;
  /** This window took or gave up focus, by the server it is on. Bridge dims behind a focused one. */
  focused: (serverId: string, on: boolean) => void;
  /** A Job's window opened or took focus: its Job becomes Bridge's open Job. Never called for a Studio's or a Session's. */
  follow: (jobId: string) => void;
};

/** `⌥⌘C`, as the action registry spells it. The renderer's own reader is `capture/Layer.tsx`. */
function isCaptureBinding(input: Electron.Input): boolean {
  // ⌥⌘A too: the annotation chord a person already presses on Bridge, which
  // in this window would otherwise reach the page's own layer and write a note
  // into a worktree nothing reads.
  const chord = input.code === "KeyC" || input.code === "KeyA";
  return input.type === "keyDown" && chord && input.meta && input.alt && !input.control && !input.shift;
}

/** The OS title: the name and the origin, or the address alone where the window has no name of its own. */
function titled(pin: Pinned): string {
  const on = shown(pin);
  return on.name === null ? on.address : `${on.name} — ${on.address}`;
}

/** A question the Session asked about a place on this window's page — `ask_in_window`. */
export type WindowQuestion = {
  id: string;
  question: string;
  options: { label: string; description?: string }[];
  place: WindowPlace;
};

/** How often a walk in the page is read, and how often it is looked for after a load, and for how many looks. */
const WALK_READ_MS = 250;
const WALK_FIND_MS = 100;
const WALK_FIND_TRIES = 50;
/** The longest one read or act waits on the page before the next is allowed. */
const WALK_ASK_MS = 1000;
/** How often a loaded page is looked at for an error overlay or nothing drawn. */
const LOOK_MS = 2000;
/** The longest a reload for a replayed walk waits for the page to load. */
const RELOAD_MS = 15_000;

/** How the window is sized, in CSS pixels, from tokens: the bar's row, the panel, and the page's floor beside it. */
export type WalkSizes = { bar: number; panel: number; floor: number };

/** Whether the panel is drawn, as last left for this kind of window, and where a press on it is kept. */
export type PanelKept = { open: boolean; keep: (open: boolean) => void };

export class CaptureWindow {
  private readonly window: BaseWindow;
  private readonly page: WebContentsView;
  private readonly bar: WebContentsView;
  private readonly board: CaptureBoard;
  /** What the page is pinned to: **the take on screen's**, which moves only to another the Session declared. */
  private pin: Pinned;
  /** The address the window was opened on, which a Session knows it by whichever take is on screen. */
  private readonly opened: string;
  /** The takes the Session declared, the first the one opened on. Empty: one page. */
  private readonly takes: readonly WindowTake[];
  /** The steps the Session passed with them, empty for a page that plays its own walk or none. */
  private readonly steps: readonly WalkStep[];
  private at = 0;
  private readiness: CaptureReadiness | undefined = undefined;
  private readonly landsOn: LandsOn;
  private readonly sizes: WalkSizes;
  /** The Session or the Job beside the page, and the view it is drawn in. `null`: a Studio's window. */
  private readonly subject: PanelSubject | null;
  private readonly panel: WebContentsView | null;
  private readonly kept: PanelKept | null;
  private panelOpen: boolean;

  private armed = false;
  private serving = true;
  private framesRefused = 0;
  private refusedAddress: CaptureWindowState["refused"];
  /** The capture a press is holding, which never leaves this process until it lands. */
  private holding: StudioCapture | null = null;
  /** The Session's open question, its place kept here as a capture's selector is. */
  private asked: WindowQuestion | null = null;
  /** Whether the question is drawn over the page, or folded into the bar. */
  private askShown = true;
  /** Where its place is in the page's viewport: unasked yet, found, or `null` for nothing on the page. */
  private askBox: CaptureRect | null | undefined = undefined;
  /** The question whose place has been scrolled to, so a later look does not pull the page back. */
  private scrolledFor: string | null = null;
  private relocating: ReturnType<typeof setTimeout> | null = null;
  /** Questions answered from this bar, so a publish from before the answer cannot raise one again. */
  private readonly settled = new Set<string>();
  /** The walk the page is playing, as last read and bounded here. `null`: the page is not a walk. */
  private walk: CaptureWalk | null = null;
  private walkTimer: ReturnType<typeof setTimeout> | null = null;
  /** Whether this document has been told to leave its card to the bar. */
  private walkHosted = false;
  private walkReading = false;
  /**
   * The steps a Session passed, played from here. **Present: the page's own `__armadaWalk` is not
   * looked for**, since the walk is Bridge's and the page draws nothing of it.
   */
  private readonly stepwalk: StepWalk | null;
  private stepsBegun = false;
  /** Which load's looking is current, so an answer that lands after another load began is let go. */
  private walkRound = 0;
  /** A page that cannot be shown, told to whoever opened the walk. */
  private readonly failures = new FailedLoads((address, why) => this.failed(address, why));
  private readonly lookTimer: ReturnType<typeof setInterval>;
  private looking = false;

  constructor(
    pin: Pinned,
    landsOn: LandsOn,
    board: CaptureBoard,
    sizes: WalkSizes,
    kept: PanelKept | null,
    takes: readonly WindowTake[] = [],
    steps: readonly WalkStep[] = [],
  ) {
    this.pin = pin;
    this.opened = pin.url;
    this.takes = takes;
    this.steps = steps;
    this.landsOn = landsOn;
    this.board = board;
    this.sizes = sizes;
    this.kept = kept;

    this.window = new BaseWindow({
      // The page keeps the width it had before there was a panel, where the screen has room for both.
      width: Math.min(1280 + (panelFor(landsOn) === null ? 0 : sizes.panel), screen.getPrimaryDisplay().workAreaSize.width),
      height: 900,
      // The OS frame and its title, where Bridge's own window is frameless and
      // draws its own: two windows that are not the same kind of thing do not
      // have the same chrome.
      title: titled(pin),
      show: false,
    });

    this.page = pageView(pin.manifestId);
    this.bar = barView();
    this.subject = panelFor(landsOn);
    this.panel = this.subject === null ? null : panelView(this.subject);
    this.panelOpen = kept?.open ?? false;
    this.stepwalk = steps.length === 0 ? null : this.playing(pin.name, steps);
    // The walk's row is drawn from the first frame, so the page does not move down when it begins.
    if (this.stepwalk !== null) this.walk = this.stepwalk.state();

    this.window.contentView.addChildView(this.page);
    this.window.contentView.addChildView(this.bar);
    if (this.panel !== null) this.window.contentView.addChildView(this.panel);
    this.window.on("resize", () => {
      this.layout();
      this.relocateSoon();
    });
    this.window.on("focus", () => {
      this.board.focused(pin.run, true);
      this.follow();
    });
    this.window.on("blur", () => this.board.focused(pin.run, false));
    this.window.on("closed", () => {
      this.board.focused(pin.run, false);
      if ("session" in landsOn) {
        this.board.sessionClosed(landsOn.session.id, this.opened);
        this.board.windowsChanged?.();
      }
      this.stopWatchingWalk();
      this.stepwalk?.stop();
      clearInterval(this.lookTimer);
      // A view's document outlives a closed `BaseWindow` unless it is closed with it.
      for (const view of [this.page, this.bar, this.panel]) if (view !== null && !view.webContents.isDestroyed()) view.webContents.close();
    });
    this.layout();

    this.holdThePage();
    this.lookTimer = setInterval(() => void this.lookAtThePage(), LOOK_MS);
    void this.page.webContents.loadURL(pin.url);
    this.window.show();
    this.follow();
  }

  /** Make this window's Job Bridge's open Job, where it walks one. */
  private follow(): void {
    const jobId = followedBy(this.landsOn);
    if (jobId !== null) this.board.follow(jobId);
  }

  /** Whether this window is still on screen. */
  get open(): boolean {
    return !this.window.isDestroyed();
  }

  /** The address the window was opened on, whichever take is on screen. */
  get address(): string {
    return this.opened;
  }

  /** The origins a question about a place may be on: the page on screen's, and every take's. */
  get origins(): string[] {
    return [this.pin.origin, ...this.takes.flatMap((one) => webOrigin(one.url) ?? [])];
  }

  /** Whether this window shows exactly these takes and steps, so a Session that shows them again is raising it. */
  showing(takes: readonly WindowTake[], steps: readonly WalkStep[] = []): boolean {
    return (
      takes.length === this.takes.length &&
      takes.every((one, at) => one.url === this.takes[at]?.url && one.name === this.takes[at]?.name) &&
      JSON.stringify(steps) === JSON.stringify(this.steps)
    );
  }

  /**
   * Swap the page to another take the Session declared. **The bar names an index and main reads the
   * address off what the Session sent**, so the origin the page is pinned to moves only to one the
   * Session chose. What was pointed at on the page left is dropped.
   */
  pick(at: number): void {
    const take = this.takes[at];
    const origin = take === undefined ? null : webOrigin(take.url);
    if (take === undefined || origin === null || at === this.at || !this.serving || this.page.webContents.isDestroyed()) return;
    this.at = at;
    this.pin = { ...this.pin, origin, url: take.url };
    this.armed = false;
    this.holding = null;
    this.refusedAddress = undefined;
    this.framesRefused = 0;
    this.window.setTitle(titled(this.pin));
    this.tell();
    // The steps are the window's and not a take's: another take starts them over.
    if (this.stepwalk !== null) this.stepwalk.act("replay");
    else void this.page.webContents.loadURL(take.url);
  }

  /** What the Session or the Job says of the page: ready, or still being changed. The bar draws it and Approve waits on it. */
  setReadiness(readiness: CaptureReadiness | undefined): void {
    if (readiness === this.readiness) return;
    this.readiness = readiness;
    this.tell();
  }

  /** The take on screen as a note and an Approve name it, or `undefined` where the window shows one page. */
  private taken(): Take | undefined {
    const take = this.takes[this.at];
    return take === undefined ? undefined : { letter: String.fromCharCode(65 + this.at), name: take.name };
  }

  get runId(): string {
    return this.pin.run;
  }

  raise(): void {
    if (this.window.isDestroyed()) return;
    if (this.window.isMinimized()) this.window.restore();
    this.window.focus();
  }

  close(): void {
    if (!this.window.isDestroyed()) this.window.close();
  }

  /** Whether an IPC call came from this window's own bar and no other document. */
  owns(contents: Electron.WebContents): boolean {
    return !this.bar.webContents.isDestroyed() && contents.id === this.bar.webContents.id;
  }

  /**
   * The Job or the Session that opened this window, for the bar's link to it. **Read off the
   * window's own record**, never from the bar: a document that names what to open can name any.
   */
  get owner(): PanelSubject | null {
    return this.subject;
  }

  /** Whether an IPC call came from this window's own panel, and what it holds if it did. Never the page or the bar. */
  panelOf(contents: Electron.WebContents): PanelSubject | null {
    if (this.panel === null || this.panel.webContents.isDestroyed()) return null;
    return contents.id === this.panel.webContents.id ? this.subject : null;
  }

  /** The panel's document, live, to be told what Bridge's own windows are told. */
  get panelContents(): Electron.WebContents | null {
    return this.panel === null || this.panel.webContents.isDestroyed() ? null : this.panel.webContents;
  }

  /** Draw the panel beside the page or fold it away, from the bar. **Kept for the next window of its kind.** */
  showPanel(open: boolean): void {
    if (this.panel === null || this.panelOpen === open) return;
    this.panelOpen = open;
    this.kept?.keep(open);
    this.tell();
    this.relocateSoon();
  }

  /**
   * The Run stopped serving. **Capture ends here and the window loads nothing
   * further** — a loopback port is not an identity, and anything on the machine
   * may bind it once the server exits.
   */
  ended(): void {
    if (!this.serving) return;
    this.serving = false;
    this.armed = false;
    this.holding = null;
    this.tell();
  }

  state(): CaptureWindowState {
    return {
      served: { run: this.pin.run, name: this.pin.name, address: this.pin.origin },
      shown: shown(this.pin),
      studio: "studio" in this.landsOn ? this.landsOn.studio : null,
      ...("job" in this.landsOn ? { job: this.landsOn.job } : {}),
      ...("session" in this.landsOn ? { session: this.landsOn.session } : {}),
      ...(this.subject === null ? {} : { panel: { kind: this.subject.kind, open: this.panelOpen } }),
      ...(this.takes.length === 0 ? {} : { variations: { names: this.takes.map((one) => one.name), at: this.at } }),
      ...(this.readiness === undefined ? {} : { readiness: this.readiness }),
      ...(this.walk === null ? {} : { walk: this.walk }),
      serving: this.serving,
      armed: this.armed,
      framesRefused: this.framesRefused,
      ...(this.refusedAddress === undefined ? {} : { refused: this.refusedAddress }),
      ...(this.asked === null
        ? {}
        : {
            ask: {
              id: this.asked.id,
              question: this.asked.question,
              options: this.asked.options,
              from: "session" in this.landsOn ? this.landsOn.session.title : this.pin.name,
              shown: this.askShown,
              ...(this.askBox === undefined ? {} : { box: this.askBox }),
            },
          }),
    };
  }

  /**
   * The Session's open question, or `null` where none is. **A new one brings the window forward,
   * moves the page to its address and finds its place**: the owner, 11 Oct 2026, a question about the
   * walk takes him where it is asking about. The address is held to the window's own origin.
   */
  question(question: WindowQuestion | null): void {
    if (question === null) {
      if (this.asked === null) return;
      this.asked = null;
      this.askBox = undefined;
      this.tell();
      return;
    }
    if (this.asked?.id === question.id || this.settled.has(question.id)) return;
    // A place on another take's origin takes the window to that take first.
    const placed = webOrigin(question.place.url);
    const there = onOrigin(this.pin.origin, question.place.url) ? -1 : this.takes.findIndex((one) => webOrigin(one.url) === placed);
    if (there >= 0) this.pick(there);
    this.asked = question;
    this.askShown = true;
    this.askBox = undefined;
    this.tell();
    this.raise();
    if (!this.serving || this.page.webContents.isDestroyed() || !onOrigin(this.pin.origin, question.place.url)) {
      void this.locate();
      return;
    }
    if (sameAddress(this.page.webContents.getURL(), question.place.url)) void this.locate();
    // The rest is `did-finish-load`'s: the place is looked for once the page it is on has drawn.
    else void this.page.webContents.loadURL(question.place.url);
  }

  /** Draw the question over the page, or fold it into the bar so the page takes presses again. */
  showAsk(on: boolean): void {
    if (this.asked === null || this.askShown === on) return;
    this.askShown = on;
    this.tell();
    if (on) void this.locate();
  }

  /** Answer the open question. **Main names the Session and the item**; the bar sends only what was said. */
  async answerAsk(answer: CaptureAnswer): Promise<Outcome> {
    const asked = this.asked;
    if (asked === null || !("session" in this.landsOn)) return { ok: false, why: "nothing_held" };
    const outcome = await this.board.answerAsk(this.landsOn.session.id, asked.id, answer);
    if (outcome.ok) {
      this.settled.add(asked.id);
      if (this.asked?.id === asked.id) this.question(null);
    }
    return outcome;
  }

  arm(on: boolean): CaptureWindowState {
    this.armed = on && this.serving;
    if (!this.armed) this.holding = null;
    this.tell();
    return this.state();
  }

  /** What is under the pointer, asked of the page and bounded here. */
  async aim(x: number, y: number): Promise<CaptureAimed | null> {
    if (!this.armed) return null;
    return aimed(await this.ask(x, y, false), this.viewport());
  }

  /**
   * Take the capture under the pointer and hold it. **The bar gets the box and
   * one line**: the markup, the styles and the selector stop here.
   */
  async hold(x: number, y: number): Promise<CaptureHeld | null> {
    if (!this.armed || !this.serving) return null;
    const answer = await this.ask(x, y, true);
    const capture = bounded(answer, this.viewport(), {
      run: this.pin.run,
      name: this.pin.name,
      address: this.pin.origin,
    });
    if (capture === null) return null;
    this.holding = capture;
    return { box: rectOf(capture.bounds, this.viewport()), chain: chainOf(capture) };
  }

  release(): void {
    this.holding = null;
  }

  /**
   * Put the held Note on the Studio this window opened on. **The frame is
   * main's**, taken of the page's own contents cropped to the element's box.
   */
  async save(said: string): Promise<Outcome> {
    const capture = this.holding;
    if (capture === null) return { ok: false, why: "nothing_held" };
    // The run ended while the note was being written: the capture would name a
    // port that is no longer the Run's.
    if (!this.serving) return { ok: false, why: "run_ended" };
    const frame = await this.frame(capture);
    const outcome =
      "studio" in this.landsOn
        ? await this.board.capture(this.landsOn.studio.id, said, capture, frame)
        : "session" in this.landsOn
          ? await this.board.sessionNote(this.landsOn.session.id, said, capture, this.pin.url, this.taken())
          : await this.board.walkNote(this.landsOn.job.id, said, capture, frame);
    if (outcome.ok) {
      this.holding = null;
      this.armed = false;
      this.tell();
    }
    return outcome;
  }

  /**
   * The person walked what this window shows and it is right. **Told to its owner, and nothing
   * closes.** A Studio's window has no owner to tell, and its bar offers no Approve.
   */
  async approve(): Promise<Outcome> {
    if ("job" in this.landsOn) return this.board.walkApproved(this.landsOn.job.id);
    if ("session" in this.landsOn) return this.board.sessionApproved(this.landsOn.session.id, this.pin.url, this.taken());
    return { ok: false, why: "no_owner" };
  }

  /** Reload the pinned origin. There is no Back, no Forward and no history. A page that has lost its place starts its steps over. */
  reload(): void {
    if (!this.serving || this.page.webContents.isDestroyed()) return;
    if (this.stepwalk !== null) this.stepwalk.act("replay");
    else void this.page.webContents.loadURL(this.pin.url);
  }

  /**
   * Hand the last refused address to whatever browses the web here. **`http:`
   * and `https:` only**, and never followed in this window: an app a person is
   * building redirects to an auth provider, and the honest answer is the
   * browser they already use.
   */
  async followRefused(): Promise<void> {
    const refused = this.refusedAddress;
    if (refused === undefined || !refused.offerable) return;
    await shell.openExternal(refused.address);
  }

  /** A wheel the bar took while it covers the page, so the page still scrolls under the outline. */
  scroll(wheel: CaptureWheel): void {
    if (!this.covered() || this.page.webContents.isDestroyed()) return;
    this.page.webContents.sendInputEvent({
      type: "mouseWheel",
      x: Math.round(wheel.x),
      y: Math.round(wheel.y),
      deltaX: Math.round(wheel.deltaX),
      deltaY: Math.round(wheel.deltaY),
      canScroll: true,
    } as Electron.MouseWheelInputEvent);
    this.relocateSoon();
  }

  /** Whether the bar covers the whole frame: capture is armed, a question is drawn over the page, or a walk Bridge plays has its ring up. */
  private covered(): boolean {
    return this.armed || (this.asked !== null && this.askShown) || this.guiding();
  }

  /** Whether the ring and caption of the steps Bridge plays are drawn over the page. Not once the walk is over, held on a step, or put away. */
  private guiding(): boolean {
    const guide = this.walk?.guide;
    return guide !== undefined && !guide.folded && !this.walk?.done && guide.stopped === undefined;
  }

  /** Look for the question's place again once the page has settled from a wheel or a resize. */
  private relocateSoon(): void {
    if (this.asked === null || !this.askShown) return;
    if (this.relocating !== null) clearTimeout(this.relocating);
    this.relocating = setTimeout(() => {
      this.relocating = null;
      void this.locate();
    }, 80);
  }

  /**
   * Find the question's place in the page, scrolling it into view the first time. **One ask, on the
   * promise main created**, as a capture's is, and the box re-bounded here against the viewport.
   */
  private async locate(): Promise<void> {
    const asked = this.asked;
    if (asked === null) return;
    let answer: unknown = null;
    if (this.serving && !this.page.webContents.isDestroyed() && onOrigin(this.pin.origin, this.page.webContents.getURL())) {
      const scroll = this.scrolledFor !== asked.id;
      try {
        answer = await this.page.webContents.executeJavaScript(
          locateSource(asked.place.selector, asked.place.text ?? null, scroll),
          false,
        );
        if (scroll) this.scrolledFor = asked.id;
      } catch {
        answer = null;
      }
    }
    if (this.asked?.id !== asked.id) return;
    const box = typeof answer === "object" && answer !== null ? (answer as Record<string, unknown>)["box"] : undefined;
    this.askBox = box === undefined ? null : rectOf(box, this.viewport());
    this.tell();
  }

  /** The page's viewport, which every box is clamped to. */
  private viewport(): { width: number; height: number } {
    const bounds = this.page.getBounds();
    return { width: Math.max(1, bounds.width), height: Math.max(1, bounds.height) };
  }

  /** One ask, answered on the promise main created. A page that throws answers nothing. */
  private async ask(x: number, y: number, take: boolean): Promise<unknown> {
    if (this.page.webContents.isDestroyed()) return null;
    try {
      return await this.page.webContents.executeJavaScript(askSource(x, y, take), false);
    } catch {
      return null;
    }
  }

  /** A PNG of the page cropped to what the Note points at, staged for Fleet to keep. */
  private async frame(capture: StudioCapture): Promise<StagedFrame | null> {
    try {
      const box = rectOf(capture.bounds, this.viewport());
      if (box.width === 0 || box.height === 0) return null;
      const image = await this.page.webContents.capturePage(box);
      const png = image.toPNG();
      if (png.byteLength === 0) return null;
      const size = image.getSize();
      return await this.board.stage(png, size.width, size.height);
    } catch {
      return null;
    }
  }

  /**
   * The bar over the page while armed, and a strip at the top while not; the panel beside both.
   *
   * **Two rows when there is something to say** — a refusal, a count of frames,
   * or the run having ended — which is the same condition the bar's own
   * `hasASecondRow` draws on, so the page starts exactly where the bar ends.
   */
  private layout(): void {
    if (this.window.isDestroyed()) return;
    const { width, height } = this.window.getContentBounds();
    const folded = this.asked !== null && !this.askShown;
    const said = !this.serving || this.refusedAddress !== undefined || this.framesRefused > 0 || folded;
    // The takes and the walk are rows of their own, under the bar's, and the second row follows them.
    const strip = this.sizes.bar * (1 + (this.takes.length === 0 ? 0 : 1) + (this.walk === null ? 0 : 1) + (said ? 1 : 0));
    const shown = this.panel !== null && this.panelOpen;
    const at = walkLayout({ width, height, strip, covered: this.covered(), panel: shown ? this.sizes.panel : 0 }, this.sizes.floor);
    this.page.setBounds(at.page);
    this.bar.setBounds(at.bar);
    if (this.panel === null) return;
    this.panel.setBounds(at.panel);
    this.panel.setVisible(at.panel.width > 0);
  }

  private tell(): void {
    this.layout();
    if (this.bar.webContents.isDestroyed()) return;
    this.bar.webContents.send(CHANNELS.captureWindowChanged, this.state());
  }

  private refuse(address: string, what: CaptureRefusedWhat): void {
    this.refusedAddress = { address, what, offerable: offerable(address) };
    this.tell();
  }

  /**
   * Every way the page could move, refused unless it stays on the origin this
   * window opened on — and refused outright once the Run has ended.
   *
   * **A subframe is counted rather than named.** A page draws many, and a bar
   * naming each would be noise where a count says the same thing.
   */
  private holdThePage(): void {
    const contents = this.page.webContents;
    const allowed = (address: string): boolean => this.serving && onOrigin(this.pin.origin, address);

    contents.setWindowOpenHandler(({ url }) => {
      this.refuse(url, "window");
      return { action: "deny" };
    });
    contents.on("will-navigate", (event, url) => {
      if (allowed(url)) return;
      event.preventDefault();
      this.refuse(url, "navigation");
    });
    contents.on("will-redirect", (event, url) => {
      if (allowed(url)) return;
      event.preventDefault();
      this.refuse(url, "redirect");
    });
    contents.on("will-frame-navigate", (event) => {
      if (event.isMainFrame || allowed(event.url)) return;
      event.preventDefault();
      this.framesRefused += 1;
      this.tell();
    });
    // A certificate error is not overridden: Electron's own default refuses,
    // and nothing here listens for one.
    // The chord arms capture from any of the three views, so it is the same press wherever focus is.
    for (const view of [this.page, this.bar, ...(this.panel === null ? [] : [this.panel])]) {
      view.webContents.on("before-input-event", (event, input) => {
        if (!isCaptureBinding(input)) return;
        event.preventDefault();
        this.arm(!this.armed);
      });
    }
    this.bar.webContents.on("did-finish-load", () => this.tell());
    // A page the question moved to, or one that moved under it: its place is looked for again.
    contents.on("did-finish-load", () => void this.locate());
    contents.on("did-navigate-in-page", () => void this.locate());
    // A walk mounts a moment after its page loads, and Back loads the page again.
    // `dom-ready` is the new document's, so it is told to leave its card again; the walk itself mounts after it.
    contents.on("dom-ready", () => this.findWalk());
    contents.on("did-finish-load", () => this.findWalk());
    // The steps Bridge plays begin on the first page that loads, and a load after that is theirs to wait out.
    contents.on("did-finish-load", () => {
      if (this.stepwalk === null || this.stepsBegun) return;
      this.stepsBegun = true;
      this.stepwalk.begin();
    });
    contents.on("did-fail-load", (_event, code, description, address, mainFrame) =>
      this.failures.failedLoad(code, description, address, mainFrame),
    );
    contents.on("did-navigate", (_event, address, status) => this.failures.navigated(address, status, this.pin.origin));
    contents.on("render-process-gone", (_event, details) => this.failures.gone(contents.getURL(), details.reason));
  }

  /**
   * Look at the loaded page for Vite's error overlay or nothing drawn. **Not while it is still
   * changing**: an overlay between two of an agent's edits is not news to the agent making them.
   */
  private async lookAtThePage(): Promise<void> {
    // A Studio's window has nobody to tell.
    if (this.subject === null || this.looking || this.readiness === "working" || this.page.webContents.isDestroyed() || this.page.webContents.isLoading()) return;
    this.looking = true;
    try {
      const address = this.page.webContents.getURL();
      this.failures.look(address, looked(await this.inPage(lookSource())));
    } finally {
      this.looking = false;
    }
  }

  /** Tell the Session or the Job. A Studio's window has nobody to tell, and a Run that ended is already known. */
  private failed(address: string, why: string): void {
    if (!this.serving) return;
    if ("session" in this.landsOn) this.board.sessionFailed(this.landsOn.session.id, address, why);
    else if ("job" in this.landsOn) this.board.walkFailed(this.landsOn.job.id, address, why);
  }

  /**
   * Look for a walk in the page: **main asks, the page never tells**. Each look is one script that
   * returns a string, as a capture's does, so the page keeps no listener of Bridge's and holds no way
   * to speak first. Looked for after every load for a few seconds; once found it is read on a beat for
   * as long as it answers. The row stays through a reload, so Back does not flicker the layout.
   */
  private findWalk(): void {
    // Steps Bridge plays are the walk; the page's own handle is not looked for.
    if (this.stepwalk !== null) return;
    this.stopWatchingWalk();
    this.walkHosted = false;
    const round = this.walkRound;
    let missed = 0;
    const look = async (): Promise<void> => {
      if (!this.open) return;
      const found = await this.readWalk();
      // A later load has started its own looking.
      if (!this.open || round !== this.walkRound) return;
      if (found) missed = 0;
      else if ((missed += 1) > WALK_FIND_TRIES) {
        this.setWalk(null);
        return;
      }
      this.walkTimer = setTimeout(() => void look(), found ? WALK_READ_MS : WALK_FIND_MS);
    };
    void look();
  }

  private stopWatchingWalk(): void {
    this.walkRound += 1;
    if (this.walkTimer !== null) clearTimeout(this.walkTimer);
    this.walkTimer = null;
  }

  /** One script in the page, or `null` where there is no page to ask. A page that never answers is let go of. */
  private async inPage(source: string): Promise<unknown> {
    const contents = this.page.webContents;
    if (contents.isDestroyed() || !this.serving || !onOrigin(this.pin.origin, contents.getURL())) return null;
    let giveUp: ReturnType<typeof setTimeout> | undefined;
    try {
      return await Promise.race([
        contents.executeJavaScript(source, false),
        new Promise<null>((resolve) => (giveUp = setTimeout(() => resolve(null), WALK_ASK_MS))),
      ]);
    } catch {
      return null;
    } finally {
      clearTimeout(giveUp);
    }
  }

  /** Read the walk once and tell the bar if it moved. Whether the page has a walk. */
  private async readWalk(): Promise<boolean> {
    if (this.walkReading) return this.walk !== null;
    this.walkReading = true;
    try {
      const walk = readWalk(await this.inPage(readSource()));
      if (walk !== null && !this.walkHosted) {
        this.walkHosted = true;
        await this.inPage(hostSource());
      }
      if (walk !== null) this.setWalk(walk);
      return walk !== null;
    } finally {
      this.walkReading = false;
    }
  }

  private setWalk(walk: CaptureWalk | null): void {
    if (sameWalk(this.walk, walk)) return;
    this.walk = walk;
    this.tell();
  }

  /** Draw the ring and caption of the steps Bridge plays over the page, or leave the page to take presses. */
  walkGuide(shown: boolean): void {
    this.stepwalk?.guide(shown);
  }

  /**
   * The steps a Session passed, played here. **Bridge's side of every act**: the target is found by
   * `finder.ts` in an isolated world, and what it is told to do is real input to the page's contents,
   * so the page hears a pointer and a keyboard and is handed nothing.
   */
  private playing(name: string, steps: readonly WalkStep[]): StepWalk {
    const contents = this.page.webContents;
    return new StepWalk(name, steps, {
      finder: new PageFinder(
        () => (this.serving && !contents.isDestroyed() && onOrigin(this.pin.origin, contents.getURL()) ? contents : null),
        () => this.viewport(),
      ),
      send: (sent) => {
        if (contents.isDestroyed()) return;
        if ("insert" in sent) void contents.insertText(sent.insert);
        else contents.sendInputEvent(sent);
      },
      command: (which) => {
        if (contents.isDestroyed()) return;
        if (which === "paste") contents.paste();
        else contents.selectAll();
      },
      // Keys reach the focused view, so the page takes the keyboard for a field and gives it back to the bar's own.
      keyboard: (page) => {
        const view = page ? this.page : this.bar;
        if (!view.webContents.isDestroyed()) view.webContents.focus();
      },
      reload: () => this.reloadPage(),
      changed: () => this.setWalk(this.stepwalk?.state() ?? null),
      stuck: (why) => {
        if ("session" in this.landsOn) this.board.sessionFailed(this.landsOn.session.id, this.pin.url, why);
      },
      open: () => this.open && this.serving,
    });
  }

  /** The pinned address loaded again, resolving once it has loaded or failed to. */
  private reloadPage(): Promise<void> {
    const contents = this.page.webContents;
    if (!this.serving || contents.isDestroyed()) return Promise.resolve();
    return new Promise<void>((resolve) => {
      const done = (): void => {
        clearTimeout(giveUp);
        if (!contents.isDestroyed()) {
          contents.off("did-finish-load", done);
          contents.off("did-fail-load", done);
        }
        resolve();
      };
      const giveUp = setTimeout(done, RELOAD_MS);
      contents.on("did-finish-load", done);
      contents.on("did-fail-load", done);
      void contents.loadURL(this.pin.url).catch(() => undefined);
    });
  }

  /**
   * One of the walk's five acts, called in the page. **The bar names the act and nothing else**: the
   * source is fixed text per act, and it is refused where the page has no walk.
   */
  async walkAct(act: WalkAct): Promise<void> {
    if (this.stepwalk !== null) {
      this.stepwalk.act(act);
      return;
    }
    if (this.walk === null) return;
    await this.inPage(actSource(act));
    // The bar is told what the act did without waiting for the next beat.
    if (!this.open) return;
    setTimeout(() => void this.readWalk(), 50);
  }
}

/** Whether two addresses are the same page, as the browser spells each. */
function sameAddress(a: string, b: string): boolean {
  try {
    return new URL(a).href === new URL(b).href;
  } catch {
    return a === b;
  }
}

export { isCaptureBinding, isPinned };
