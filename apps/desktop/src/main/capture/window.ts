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

import type { Outcome, SendSessionNote, StagedFrame, StudioCapture, WalkStep, WindowPlace, WindowTake } from "@armada/protocol";
import type {
  CaptureAimed,
  CaptureAnswer,
  CaptureRect,
  CaptureHeld,
  CaptureRefusedWhat,
  CaptureWheel,
  CaptureQueue,
  CaptureQueueKey,
  CaptureReadiness,
  CaptureWalk,
  CaptureWindowState,
} from "../../shared/capture-window";
import { CHANNELS } from "../../shared/bridge";
import { isPinned, loopbackOrigin, offerable, onAnyOrigin, onNamedOrigin, onOrigin, shown, webOrigin, type Pinned } from "./address";
import { aimed, bounded, chainOf, readOf, rectOf, type PageRead } from "./bounds";
import { barRows, panelFor, walkLayout, type LandsOn, type PanelSubject, type ReviewWalkOwner } from "./layout";
import { FailedLoads, lookSource, looked } from "./failed";
import type { Take } from "./session-note";
import { sessionNoteBody } from "./pinned-note";
import { heldOn } from "./held-notes";
import { askSource } from "./layer";
import { locateSource } from "./locate";
import { PageFinder } from "./finder";
import { mayOpenPopup, openPopup, type Popup } from "./popup";
import { FIELD_SOURCE, queueKeyOf } from "./queue";
import { beginsOn, StepWalk } from "./stepwalk";
import { StoppedSteps, actSource, hostSource, readSource, readWalk, sameWalk, type WalkAct } from "./walk";
import { barView, pageView, panelView } from "./views";

export type { LandsOn } from "./layout";

/** What the window needs from main: a Note lands, and a frame is staged. */
export type CaptureBoard = {
  capture: (studioId: string, said: string, capture: StudioCapture, frame: StagedFrame | null) => Promise<Outcome>;
  /**
   * A note taken on a page a Job showed, or in a review's walk the Job holds the pull request of: **held on that Job**,
   * not sent, until the owner sends its notes as one turn (`held-notes.ts`).
   */
  holdNote: (note: SendSessionNote) => void;
  /** How many notes that Job holds that have not been sent. */
  heldOn: (sessionId: string) => number;
  /** A note taken in a review's walk no Session holds the pull request of: kept in Armada. */
  reviewNote: (review: ReviewWalkOwner, said: string, capture: StudioCapture) => Promise<Outcome>;
  /** A review's walk window closed, whoever closed it: its held walk is over. */
  reviewClosed: (review: ReviewWalkOwner) => void;
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
  stage: (png: Buffer, width: number, height: number) => Promise<StagedFrame | null>;
  /** This window took or gave up focus, by the server it is on. Bridge dims behind a focused one. */
  focused: (serverId: string, on: boolean) => void;
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
/** The longest one sign-in address is waited on before the next. */
const SIGN_IN_MS = 15000;

/** How the window is sized, in CSS pixels, from tokens: the bar's row, the panel, and the page's floor beside it. */
export type WalkSizes = { bar: number; panel: number; floor: number };

/** Whether the panel is drawn, as last left for this kind of window, and where a press on it is kept. */
export type PanelKept = { open: boolean; keep: (open: boolean) => void };

/**
 * Where a walk of the one walk window draws: **the window, the bar and the panel are the window's, and the page is the
 * walk's own**, kept with its place while another walk is on screen. Only the walk on screen lays the views out and
 * draws the bar. `walk-window.ts` holds it.
 */
export type WalkSeat = {
  window: BaseWindow;
  bar: WebContentsView;
  panel: WebContentsView | null;
  onScreen: (walk: CaptureWindow) => boolean;
  /** Every walk in the window, for the bar. */
  queue: () => CaptureQueue;
  /** One of the queue's keys, heard while the page had the keyboard: the bar acts on it. */
  key: (key: CaptureQueueKey) => void;
  /** The owner approved this walk: it leaves the window. */
  settled: (walk: CaptureWindow) => void;
};

/** The world the page is asked in whether a field has the cursor. Its own, apart from the finder's. */
const FIELD_WORLD = 2001;

export class CaptureWindow {
  private readonly window: BaseWindow;
  private readonly page: WebContentsView;
  private readonly bar: WebContentsView;
  private readonly board: CaptureBoard;
  /** The one walk window this walk is queued in. `null`: a window of its own, as a Studio's is. */
  private readonly seat: WalkSeat | null;
  /** Taken out of the walk window: its page is gone, and it answers nothing. */
  private disposed = false;
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
  /** The Session beside the page, and the view it is drawn in. `null`: a Studio's window. */
  private readonly subject: PanelSubject | null;
  private readonly panel: WebContentsView | null;
  private readonly kept: PanelKept | null;
  private panelOpen: boolean;

  private armed = false;
  private serving = true;
  /** The page is loading, which the bar says so a page not drawn yet does not read as a broken one. */
  private loading = false;
  private framesRefused = 0;
  private refusedAddress: CaptureWindowState["refused"];
  /** The capture a press is holding, which never leaves this process until it lands. */
  private holding: StudioCapture | null = null;
  /** What the same press read beside it, for a Session's note. Set with `holding`, read only while it is. */
  private held: PageRead = { nearby: [], selected: [] };
  /** The Session's open question, its place kept here as a capture's selector is. */
  private asked: WindowQuestion | null = null;
  /** Whether the question is drawn over the page, or folded into the bar. */
  private askShown = true;
  /** How far down from its top the bar's open menu or tooltip draws, or `null` while none is open. */
  private hang: number | null = null;
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
  /** The steps the page's walk stopped on, each told to the Job once. */
  private readonly stoppedSteps = new StoppedSteps();
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
  /** Whether the Session's sign-in addresses are loading, which no failure is told to anyone for: their text is a secret. */
  private signingIn = false;
  /** The popup the page opened, one at a time: a provider's sign-in window, on an origin the page may reach. */
  private popup: Popup | null = null;

  constructor(
    pin: Pinned,
    landsOn: LandsOn,
    board: CaptureBoard,
    sizes: WalkSizes,
    kept: PanelKept | null,
    takes: readonly WindowTake[] = [],
    signIn: readonly string[] = [],
    steps: readonly WalkStep[] = [],
    seat?: WalkSeat,
  ) {
    this.pin = pin;
    this.opened = pin.url;
    this.takes = takes;
    this.steps = steps;
    this.landsOn = landsOn;
    this.board = board;
    this.sizes = sizes;
    this.kept = kept;
    this.seat = seat ?? null;
    this.subject = panelFor(landsOn);

    if (seat === undefined) {
      this.window = new BaseWindow({
        // The page keeps the width it had before there was a panel, where the screen has room for both.
        width: Math.min(1280 + (this.subject === null ? 0 : sizes.panel), screen.getPrimaryDisplay().workAreaSize.width),
        height: 900,
        // The OS frame and its title, where Bridge's own window is frameless and
        // draws its own: two windows that are not the same kind of thing do not
        // have the same chrome.
        title: titled(pin),
        show: false,
      });
      this.bar = barView();
      this.panel = this.subject === null ? null : panelView(this.subject);
    } else {
      this.window = seat.window;
      this.bar = seat.bar;
      this.panel = seat.panel;
    }
    this.page = pageView(pin);
    this.panelOpen = kept?.open ?? false;
    this.stepwalk = steps.length === 0 ? null : this.playing(pin.name, steps);
    // The walk's row is drawn from the first frame, so the page does not move down when it begins.
    if (this.stepwalk !== null) this.walk = this.stepwalk.state();

    if (seat !== undefined) {
      // Under the bar and the panel, which the walk window put up before any page.
      this.window.contentView.addChildView(this.page, 0);
    } else {
      this.window.contentView.addChildView(this.page);
      this.window.contentView.addChildView(this.bar);
      if (this.panel !== null) this.window.contentView.addChildView(this.panel);
      this.window.on("resize", () => this.resized());
      this.window.on("focus", () => {
        this.board.focused(pin.run, true);
      });
      this.window.on("blur", () => this.board.focused(pin.run, false));
      this.window.on("closed", () => {
        this.board.focused(pin.run, false);
        if ("session" in landsOn) {
          this.board.sessionClosed(landsOn.session.id, this.opened);
          this.board.windowsChanged?.();
        }
        if ("review" in landsOn) this.board.reviewClosed(landsOn.review);
        this.stop();
        // A view's document outlives a closed `BaseWindow` unless it is closed with it.
        for (const view of [this.page, this.bar, this.panel]) if (view !== null && !view.webContents.isDestroyed()) view.webContents.close();
      });
    }
    this.layout();

    this.holdThePage();
    this.lookTimer = setInterval(() => void this.lookAtThePage(), LOOK_MS);
    void this.start(pin.url, signIn);
    if (seat === undefined) this.window.show();
  }

  /** Everything that runs on its own beside the page, stopped. */
  private stop(): void {
    this.stopWatchingWalk();
    this.popup?.close();
    this.stepwalk?.stop();
    this.failures.stopped();
    clearInterval(this.lookTimer);
  }

  /**
   * Out of the walk window for good: approved, shown again in its place, or the window closed. **Its page goes with it**, and
   * nobody is told here: the window tells the walk's owner.
   */
  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.stop();
    if (!this.window.isDestroyed()) this.window.contentView.removeChildView(this.page);
    if (!this.page.webContents.isDestroyed()) this.page.webContents.close();
  }

  /**
   * Another walk comes on screen. **This page stays as it is, out of sight**, with nothing armed and nothing playing by
   * itself, so coming back finds it where it was left.
   */
  hide(): void {
    this.armed = false;
    this.holding = null;
    this.hang = null;
    if (this.walk?.autoplay === true) void this.walkAct("autoplay_off");
    this.page.setVisible(false);
  }

  /** This walk comes on screen: its page, its title and the bar drawn for it, and the panel as last left. */
  show(): void {
    if (this.kept !== null) this.panelOpen = this.kept.open;
    this.page.setVisible(true);
    if (!this.window.isDestroyed()) this.window.setTitle(titled(this.pin));
    this.tell();
    this.relocateSoon();
  }

  /** The window was resized: the views placed again, and a question's place looked for again. */
  resized(): void {
    this.layout();
    this.relocateSoon();
  }

  /** The bar's document loaded again, or the queue around this walk moved: the bar is drawn afresh. */
  redraw(): void {
    this.tell();
  }

  /** Whether this walk is the one its window draws: always, for a window of its own. */
  private onScreen(): boolean {
    return this.seat === null || this.seat.onScreen(this);
  }

  /**
   * Load the page, after the addresses a Session signs in with, each in the window's own cookie jar
   * and in order. **They are a secret**: none is kept past this call, logged or told to anyone, and a
   * load of one that fails says nothing (`signingIn`). Each is also held to the origins the window
   * may be on, as Fleet held it when it was named. One that hangs is let go of after `SIGN_IN_MS`.
   */
  private async start(url: string, signIn: readonly string[]): Promise<void> {
    const contents = this.page.webContents;
    this.signingIn = signIn.length > 0;
    for (const address of signIn) {
      if (contents.isDestroyed()) return;
      if (!onAnyOrigin(this.pin, address)) continue;
      let giveUp: ReturnType<typeof setTimeout> | undefined;
      try {
        await Promise.race([
          contents.loadURL(address).catch(() => undefined),
          new Promise<void>((resolve) => (giveUp = setTimeout(resolve, SIGN_IN_MS))),
        ]);
      } finally {
        clearTimeout(giveUp);
      }
    }
    this.signingIn = false;
    if (!contents.isDestroyed()) void contents.loadURL(url);
  }

  /** Whether this window is still on screen, or this walk still in its window. */
  get open(): boolean {
    return !this.disposed && !this.window.isDestroyed();
  }

  /** The held walk this window plays, where it is a pull request's. */
  get walkId(): string | null {
    return "review" in this.landsOn ? this.landsOn.review.walkId : null;
  }

  get walkManifestId(): string | null {
    return "review" in this.landsOn ? this.landsOn.review.manifestId : null;
  }

  /** The address the window was opened on, whichever take is on screen. */
  get address(): string {
    return this.opened;
  }

  /** The origins a question about a place may be on: the page on screen's, and every take's. */
  get origins(): string[] {
    return [this.pin.origin, ...this.takes.flatMap((one) => webOrigin(one.url) ?? [])];
  }

  /** Whether the window may pass through exactly these origins, so a Session that shows the page again with them is raising it. */
  passes(through: readonly string[]): boolean {
    return through.length === this.pin.through.length && through.every((one, at) => one === this.pin.through[at]);
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
    if (this.onScreen()) this.window.setTitle(titled(this.pin));
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

  /** Close the window, or take this walk out of the walk window. */
  close(): void {
    if (this.seat !== null) this.dispose();
    else if (!this.window.isDestroyed()) this.window.close();
  }

  /** Whether an IPC call came from this window's own bar and no other document. **A queued walk's only while it is on screen.** */
  owns(contents: Electron.WebContents): boolean {
    return this.open && this.onScreen() && !this.bar.webContents.isDestroyed() && contents.id === this.bar.webContents.id;
  }

  /**
   * The Session that opened this window, for the bar's link to it. **Read off the
   * window's own record**, never from the bar: a document that names what to open can name any.
   */
  get owner(): PanelSubject | null {
    return this.subject;
  }

  /** Whether an IPC call came from this window's own panel, and what it holds if it did. Never the page or the bar. */
  panelOf(contents: Electron.WebContents): PanelSubject | null {
    if (this.panel === null || this.panel.webContents.isDestroyed() || !this.onScreen()) return null;
    return contents.id === this.panel.webContents.id ? this.subject : null;
  }

  /** The panel's document, live, to be told what Bridge's own windows are told. */
  get panelContents(): Electron.WebContents | null {
    return this.panel === null || this.panel.webContents.isDestroyed() ? null : this.panel.webContents;
  }

  /** Draw the panel beside the page or fold it away, from the bar. **Kept for the next window of its kind.** */
  showPanel(open: boolean): void {
    if (this.panel === null || this.subject === null || this.panelOpen === open) return;
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
      ...(this.seat === null ? {} : { queue: this.seat.queue() }),
      served: { run: this.pin.run, name: this.pin.name, address: this.pin.origin },
      shown: shown(this.pin),
      studio: "studio" in this.landsOn ? this.landsOn.studio : null,
      ...("session" in this.landsOn ? { session: this.landsOn.session } : {}),
      ...("review" in this.landsOn ? { review: { title: this.landsOn.review.title, ...(this.landsOn.review.holder === undefined ? {} : { holder: this.landsOn.review.holder.title }) } } : {}),
      ...(this.subject === null ? {} : { panel: { kind: this.subject.kind, open: this.panelOpen } }),
      ...(this.takes.length === 0 ? {} : { variations: { names: this.takes.map((one) => one.name), at: this.at } }),
      ...(this.readiness === undefined ? {} : { readiness: this.readiness }),
      ...(this.walk === null ? {} : { walk: this.walk }),
      serving: this.serving,
      armed: this.armed,
      ...(this.loading ? { loading: true as const } : {}),
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
      ...this.heldState(),
    };
  }

  private heldState(): Pick<CaptureWindowState, "held"> {
    const count = this.notesOn === undefined ? 0 : this.board.heldOn(this.notesOn);
    return count === 0 ? {} : { held: count === 1 ? "one" : "several" };
  }

  /**
   * The Session's open question, or `null` where none is. **A new one brings the window forward,
   * moves the page to its address and finds its place**: the owner, 11 Oct 2026, a question about the
   * walk takes him where it is asking about. The address is held to the window's own origin. Not
   * `raise`: a walk the walk window opened behind, or came to on its own, which takes no focus.
   */
  question(question: WindowQuestion | null, raise = true): void {
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
    if (raise) this.raise();
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

  /** The capture binding: capture on where it is off, and off where it is on. */
  toggleCapture(): void {
    this.arm(!this.armed);
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
    this.held = readOf(answer);
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
    const holds = heldOn(this.landsOn, this.pin.url);
    const outcome =
      "studio" in this.landsOn
        ? await this.board.capture(this.landsOn.studio.id, said, capture, await this.frame(capture))
        : holds !== null
          ? await this.holdNote(holds.sessionId, holds.address, said, capture)
          : "review" in this.landsOn
            ? await this.board.reviewNote(this.landsOn.review, said, capture)
            : ({ ok: false, why: "no_owner" } as const);
    if (outcome.ok) {
      this.holding = null;
      this.armed = false;
      this.tell();
    }
    return outcome;
  }

  /** A Job's note, with the crop, the walk's step and the take on screen it is about, held on the Job until the owner sends it. */
  private async holdNote(sessionId: string, address: string, said: string, capture: StudioCapture): Promise<Outcome> {
    const cropped = await this.crop(capture);
    const take = this.taken();
    const note = sessionNoteBody({
      sessionId,
      said,
      capture,
      read: this.held,
      address,
      ...(take === undefined ? {} : { take }),
      walk: this.walk,
      crop: cropped === null ? null : cropped.png.toString("base64"),
    });
    this.board.holdNote(note);
    return { ok: true };
  }

  /** The Job this window's notes are held on, where they are held. */
  get notesOn(): string | undefined {
    return heldOn(this.landsOn, this.pin.url)?.sessionId;
  }

  /** A Job's held notes changed: the bar is told again where they are this window's. */
  heldChanged(sessionId: string): void {
    if (this.notesOn === sessionId) this.tell();
  }

  /**
   * The person walked what this window shows and it is right. **Told to its owner, and the walk leaves the walk window**,
   * which moves to the next. A pull request's walk tells nobody: it is over, and its held walk is closed with it. A Studio's
   * window has no owner to tell, and its bar offers no Approve.
   */
  async approve(): Promise<Outcome> {
    const outcome: Outcome =
      "session" in this.landsOn
        ? await this.board.sessionApproved(this.landsOn.session.id, this.pin.url, this.taken())
        : "review" in this.landsOn && this.seat !== null
          ? { ok: true }
          : { ok: false, why: "no_owner" };
    if (outcome.ok && this.open) this.seat?.settled(this);
    return outcome;
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

  /**
   * A menu or a tooltip of the bar's opened, closed or changed size. **The bar's view comes down over the page's column
   * that far while it is open**, so the menu is not cut at the strip's edge, and the page takes no presses under it.
   */
  overhang(height: number | null): void {
    if (height === this.hang) return;
    this.hang = height;
    this.layout();
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
    return guide !== undefined && !guide.folded && !this.walk?.done && guide.stopped === undefined && guide.signingIn !== true;
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

  /** A PNG of the page cropped to what the Note points at, or null where none could be taken. */
  private async crop(capture: StudioCapture): Promise<{ png: Buffer; width: number; height: number } | null> {
    try {
      const box = rectOf(capture.bounds, this.viewport());
      if (box.width === 0 || box.height === 0) return null;
      const image = await this.page.webContents.capturePage(box);
      const png = image.toPNG();
      if (png.byteLength === 0) return null;
      const size = image.getSize();
      return { png, width: size.width, height: size.height };
    } catch {
      return null;
    }
  }

  /** The crop, staged for Fleet to keep beside a Studio's Note. */
  private async frame(capture: StudioCapture): Promise<StagedFrame | null> {
    const cropped = await this.crop(capture);
    if (cropped === null) return null;
    try {
      return await this.board.stage(cropped.png, cropped.width, cropped.height);
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
   * Notes held on the Job take the queued bar's second row as well, and the page loading takes it on either.
   */
  private layout(): void {
    if (this.window.isDestroyed() || !this.open || !this.onScreen()) return;
    const { width, height } = this.window.getContentBounds();
    const folded = this.asked !== null && !this.askShown;
    const said = !this.serving || this.refusedAddress !== undefined || this.framesRefused > 0 || folded;
    const queued = this.seat !== null && this.seat.queue().entries.length > 0;
    const held = this.heldState().held !== undefined;
    const strip = this.sizes.bar * barRows({ queued, takes: this.takes.length > 0, walk: this.walk !== null, said, held, loading: this.loading });
    // A pull request's walk in the walk window has no Job, so the window's panel is not drawn beside it.
    const shown = this.panel !== null && this.subject !== null && this.panelOpen;
    const at = walkLayout({ width, height, strip, covered: this.covered(), hang: this.hang ?? 0, panel: shown ? this.sizes.panel : 0 }, this.sizes.floor);
    this.page.setBounds(at.page);
    this.bar.setBounds(at.bar);
    if (this.panel === null) return;
    this.panel.setBounds(at.panel);
    this.panel.setVisible(at.panel.width > 0);
  }

  private tell(): void {
    if (!this.open || !this.onScreen()) return;
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
    const allowed = (address: string): boolean => this.serving && onAnyOrigin(this.pin, address);

    // **A Session's page may open one popup, and only onto an origin the window may reach**: the shown
    // one or one it named. Any other address, a second popup, and every popup of a Run's page or a Job's
    // are refused, as every popup was.
    contents.setWindowOpenHandler(({ url }) => {
      if (!mayOpenPopup({ session: "session" in this.landsOn || "review" in this.landsOn, standing: this.popup !== null, allowed: allowed(url) })) {
        this.refuse(url, "window");
        return { action: "deny" };
      }
      return { action: "allow", createWindow: (options) => this.openPopup(allowed, options) };
    });
    contents.on("will-navigate", (event, url) => {
      if (allowed(url)) {
        this.watchSignIn(url);
        return;
      }
      event.preventDefault();
      this.refuse(url, "navigation");
    });
    contents.on("will-redirect", (event, url) => {
      if (allowed(url)) {
        if (event.isMainFrame) this.watchSignIn(url);
        return;
      }
      event.preventDefault();
      this.refuse(url, "redirect");
    });
    // Where the page is, for the walk: on a provider's page it waits, and on the shown origin again it goes on.
    contents.on("did-navigate", () => this.watchSignIn());
    contents.on("did-navigate-in-page", () => this.watchSignIn());
    contents.on("did-fail-load", () => this.watchSignIn());
    contents.on("will-frame-navigate", (event) => {
      if (event.isMainFrame || allowed(event.url)) return;
      event.preventDefault();
      this.framesRefused += 1;
      this.tell();
    });
    // A certificate error is not overridden: Electron's own default refuses,
    // and nothing here listens for one.
    // The chord arms capture from any of the three views, so it is the same press wherever focus is. The walk window hears
    // its bar and its panel itself, for whichever walk is on screen.
    const own = this.seat === null ? [this.page, this.bar, ...(this.panel === null ? [] : [this.panel])] : [this.page];
    for (const view of own) {
      view.webContents.on("before-input-event", (event, input) => {
        if (!isCaptureBinding(input)) return;
        event.preventDefault();
        this.toggleCapture();
      });
    }
    if (this.seat === null) this.bar.webContents.on("did-finish-load", () => this.tell());
    // The queue's keys reach the bar from the page, which holds the keyboard most of a walk. ⌘↵ is the page's own in a
    // field, so it is never taken from the page: it approves only once the page says no field has the cursor.
    const seat = this.seat;
    if (seat !== null) {
      contents.on("before-input-event", (event, input) => {
        const key = queueKeyOf(input);
        if (key === null) return;
        if (key === "approve") {
          void this.inAField().then((field) => !field && this.open && seat.key("approve"));
          return;
        }
        event.preventDefault();
        seat.key(key);
      });
    }
    // A page the question moved to, or one that moved under it: its place is looked for again.
    contents.on("did-finish-load", () => void this.locate());
    contents.on("did-navigate-in-page", () => void this.locate());
    // A walk mounts a moment after its page loads, and Back loads the page again.
    // `dom-ready` is the new document's, so it is told to leave its card again; the walk itself mounts after it.
    contents.on("dom-ready", () => this.findWalk());
    contents.on("did-finish-load", () => this.findWalk());
    // The steps Bridge plays begin on the first load of the shown page: not a sign-in address, and not
    // a provider's page the window passed through, whose words the steps were not written for.
    contents.on("did-finish-load", () => {
      const onShownOrigin = onOrigin(this.pin.origin, contents.getURL());
      if (this.stepwalk === null || !beginsOn({ begun: this.stepsBegun, signingIn: this.signingIn, onShownOrigin })) return;
      this.stepsBegun = true;
      this.stepwalk.begin();
    });
    contents.on("did-fail-load", (_event, code, description, address, mainFrame) => {
      if (mainFrame) {
        this.loadingNow(false);
        this.failures.stopped();
      }
      if (!this.signingIn) this.failures.failedLoad(code, description, address, mainFrame);
    });
    contents.on("did-start-loading", () => {
      this.loadingNow(true);
      // A stalled load has committed no address yet, so the one it was sent to stands in.
      if (!this.signingIn) this.failures.started(() => contents.isDestroyed() || contents.getURL() === "" ? this.pin.url : contents.getURL());
    });
    contents.on("did-stop-loading", () => {
      this.loadingNow(false);
      this.failures.stopped();
    });
    contents.on("did-navigate", (_event, address, status) => {
      if (!this.signingIn) this.failures.navigated(address, status, this.pin.origin);
    });
    contents.on("render-process-gone", (_event, details) => {
      if (!this.signingIn) this.failures.gone(contents.getURL(), details.reason);
    });
  }

  /** The page started or stopped loading: the bar is told. */
  private loadingNow(loading: boolean): void {
    if (this.loading === loading) return;
    this.loading = loading;
    this.tell();
  }

  /**
   * The popup a provider's sign-in opens, as a child window of this one. **Held to the same origins
   * as the page**, with no preload and the page's own session (`popup.ts`).
   */
  private openPopup(allowed: (address: string) => boolean, options: Electron.BrowserWindowConstructorOptions): Electron.WebContents {
    const popup = openPopup(
      {
        parent: this.window,
        allowed,
        refuse: (address, what) => this.refuse(address, what),
        closed: () => {
          this.popup = null;
          this.watchSignIn();
        },
      },
      options,
    );
    // Electron wants the contents it made back whatever happened; a popup that was not built is closed already.
    if (popup === null) return (options as { webContents: Electron.WebContents }).webContents;
    this.popup = popup;
    this.watchSignIn();
    return popup.contents;
  }

  /**
   * Tell the steps whether the page is being signed in: on a provider's page, heading to one (`heading`,
   * a navigation allowed but not yet committed), or with its popup open. **The walk waits there and
   * does not read the missing target as a fault**, and goes on once the page is back on the shown origin.
   */
  private watchSignIn(heading?: string): void {
    if (this.stepwalk === null || this.page.webContents.isDestroyed()) return;
    this.stepwalk.signIn(this.popup !== null || onNamedOrigin(this.pin, heading ?? this.page.webContents.getURL()));
  }

  /**
   * Look at the loaded page for Vite's error overlay or nothing drawn. **Not while it is still
   * changing**: an overlay between two of an agent's edits is not news to the agent making them.
   */
  private async lookAtThePage(): Promise<void> {
    // A Studio's window has nobody to tell.
    if (this.subject === null || this.looking || this.signingIn || this.readiness === "working" || this.page.webContents.isDestroyed() || this.page.webContents.isLoading()) return;
    this.looking = true;
    try {
      const address = this.page.webContents.getURL();
      this.failures.look(address, looked(await this.inPage(lookSource())));
    } finally {
      this.looking = false;
    }
  }

  /** Tell the Session. A Studio's window has nobody to tell, and a Run that ended is already known. */
  private failed(address: string, why: string): void {
    if (!this.serving) return;
    if ("session" in this.landsOn) this.board.sessionFailed(this.landsOn.session.id, address, why);
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
        this.stoppedSteps.seen(null);
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

  /** Whether a field in the page has the cursor, asked in a world of the page's that its own scripts cannot see. A page that does not answer has one. */
  private async inAField(): Promise<boolean> {
    const contents = this.page.webContents;
    if (contents.isDestroyed()) return true;
    try {
      return (await contents.executeJavaScriptInIsolatedWorld(FIELD_WORLD, [{ code: FIELD_SOURCE }])) !== false;
    } catch {
      return true;
    }
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
      if (walk !== null) {
        const why = this.stoppedSteps.seen(walk);
        if (why !== null && this.serving && "session" in this.landsOn) this.board.sessionFailed(this.landsOn.session.id, this.pin.url, why);
        this.setWalk(walk);
      }
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
      guarded: () => loopbackOrigin(this.pin.origin) === null,
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
