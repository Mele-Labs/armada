// armada-allow-off-contract: the one colour here is Electron's own spelling for a view with no background of its own, not a design value
// docs/practices/capture-window.md
//
// The capture window — #1294, `docs/practices/capture-window.md`.
//
// **Two views in one frame.** The page runs in a view with no preload, in a
// persistent session of its own; Bridge's bar runs above it in a view with
// Bridge's preload and Bridge's CSP. While capture is armed the bar's view
// grows over the whole frame, so the outline is drawn where the page cannot
// read it, restyle it or forge it, and a press never reaches the app.
//
// Bridge's own window is untouched: it still loads one file and refuses every
// navigation — `bridge.md`, *Security posture*.

import { BaseWindow, session, shell, WebContentsView, type Session } from "electron";
import { join } from "node:path";

import type { Outcome, StagedFrame, StudioCapture, WindowPlace } from "@armada/protocol";
import type {
  CaptureAimed,
  CaptureAnswer,
  CaptureRect,
  CaptureHeld,
  CaptureRefusedWhat,
  CaptureWheel,
  CaptureWindowState,
} from "../../shared/capture-window";
import { CHANNELS } from "../../shared/bridge";
import { isPinned, offerable, onOrigin, partitionFor, type Pinned } from "./address";
import { aimed, bounded, chainOf, rectOf } from "./bounds";
import { askSource } from "./layer";
import { locateSource } from "./locate";

/** What the window needs from main: a Note lands, and a frame is staged. */
export type CaptureBoard = {
  capture: (studioId: string, said: string, capture: StudioCapture, frame: StagedFrame | null) => Promise<Outcome>;
  /** A note kept on the Job whose server this window walks — protocol 23.18. */
  walkNote: (jobId: string, said: string, capture: StudioCapture, frame: StagedFrame | null) => Promise<Outcome>;
  /** A note taken on a page a Session showed: a message from the person, which wakes the Session. */
  sessionNote: (sessionId: string, said: string, capture: StudioCapture, address: string) => Promise<Outcome>;
  /** Approve on the bar, for a Job: the act JobDetail's own Approve sends. */
  walkApproved: (jobId: string) => Promise<Outcome>;
  /** Approve on the bar, for a Session: a message from the person naming the address, which wakes it. */
  sessionApproved: (sessionId: string, address: string) => Promise<Outcome>;
  /** The Session's question, answered from the bar: `answer_waiting` on its item, which wakes it. */
  answerAsk: (sessionId: string, itemId: string, answer: CaptureAnswer) => Promise<Outcome>;
  /** A Session's window closed, whoever closed it: its walk is over, so the Session may ask again. */
  sessionClosed: (sessionId: string, address: string) => void;
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

/** Where a note made in this window lands: the Run's Studio, the Job whose server it walks, or the Session that showed the page. */
/** A question the Session asked about a place on this window's page — `ask_in_window`. */
export type WindowQuestion = {
  id: string;
  question: string;
  options: { label: string; description?: string }[];
  place: WindowPlace;
};

export type LandsOn =
  | { studio: { id: string; name: string | null } }
  | { job: { id: string; handle: string } }
  | { session: { id: string; title: string } };

/**
 * Every partition already given its handlers. **Kept because a partition is
 * per repository and two windows share one**, so the download refusal would
 * otherwise be added twice and the second window's close would take both.
 */
const held = new Set<string>();

/**
 * The session this window's page runs in: persistent, one per repository, and
 * nothing of Bridge's in it.
 *
 * **Every permission is denied without asking.** A page that wants the camera,
 * the clipboard, a notification or the screen gets no dialog and no capability
 * — this window exists to look at a page, not to host one.
 */
function sessionFor(manifestId: string | null): Session {
  const name = partitionFor(manifestId);
  const part = session.fromPartition(name);
  if (held.has(name)) return part;
  held.add(name);
  part.setPermissionRequestHandler((_contents, _permission, decide) => decide(false));
  part.setPermissionCheckHandler(() => false);
  part.setDisplayMediaRequestHandler(() => {}, { useSystemPicker: false });
  part.on("will-download", (event) => event.preventDefault());
  return part;
}

export class CaptureWindow {
  private readonly window: BaseWindow;
  private readonly page: WebContentsView;
  private readonly bar: WebContentsView;
  private readonly board: CaptureBoard;
  private readonly pin: Pinned;
  private readonly landsOn: LandsOn;
  /** How tall the bar is drawn, in CSS pixels, from the tokens the title row is built of. */
  private readonly barHeight: number;

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

  constructor(pin: Pinned, landsOn: LandsOn, board: CaptureBoard, barHeight: number) {
    this.pin = pin;
    this.landsOn = landsOn;
    this.board = board;
    this.barHeight = barHeight;

    this.window = new BaseWindow({
      width: 1280,
      height: 900,
      // The OS frame and its title, where Bridge's own window is frameless and
      // draws its own: two windows that are not the same kind of thing do not
      // have the same chrome.
      title: `${pin.name} — ${pin.origin}`,
      show: false,
    });

    this.page = new WebContentsView({
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

    this.bar = new WebContentsView({
      webPreferences: {
        preload: join(__dirname, "../preload/index.js"),
        sandbox: true,
        contextIsolation: true,
        nodeIntegration: false,
      },
    });
    // Fully transparent: the bar paints its own strip and nothing else, so the
    // page shows through this view wherever the bar does not draw.
    this.bar.setBackgroundColor("#00000000");

    this.window.contentView.addChildView(this.page);
    this.window.contentView.addChildView(this.bar);
    this.window.on("resize", () => {
      this.layout();
      this.relocateSoon();
    });
    this.window.on("focus", () => this.board.focused(pin.run, true));
    this.window.on("blur", () => this.board.focused(pin.run, false));
    this.window.on("closed", () => {
      this.board.focused(pin.run, false);
      if ("session" in landsOn) this.board.sessionClosed(landsOn.session.id, pin.url);
    });
    this.layout();

    this.holdThePage();
    void this.bar.webContents.loadFile(join(__dirname, "../renderer/capture.html"));
    void this.page.webContents.loadURL(pin.url);
    this.window.show();
  }

  /** Whether this window is still on screen. */
  get open(): boolean {
    return !this.window.isDestroyed();
  }

  /** The address the window was opened on. */
  get address(): string {
    return this.pin.url;
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
      studio: "studio" in this.landsOn ? this.landsOn.studio : null,
      // A Session's title stands where a Job's handle does: the bar says "Notes go to" either.
      ...("job" in this.landsOn ? { job: this.landsOn.job } : {}),
      ...("session" in this.landsOn ? { job: { id: this.landsOn.session.id, handle: this.landsOn.session.title } } : {}),
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
          ? await this.board.sessionNote(this.landsOn.session.id, said, capture, this.pin.url)
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
    if ("session" in this.landsOn) return this.board.sessionApproved(this.landsOn.session.id, this.pin.url);
    return { ok: false, why: "no_owner" };
  }

  /** Reload the pinned origin. There is no Back, no Forward and no history. */
  reload(): void {
    if (!this.serving || this.page.webContents.isDestroyed()) return;
    void this.page.webContents.loadURL(this.pin.url);
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

  /** Whether the bar covers the whole frame: capture is armed, or a question is drawn over the page. */
  private covered(): boolean {
    return this.armed || (this.asked !== null && this.askShown);
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
   * The bar over the whole frame while armed, and a strip at the top while not.
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
    const strip = this.barHeight * (said ? 2 : 1);
    this.page.setBounds({ x: 0, y: strip, width, height: Math.max(0, height - strip) });
    this.bar.setBounds({ x: 0, y: 0, width, height: this.covered() ? height : strip });
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
    contents.on("before-input-event", (event, input) => {
      if (!isCaptureBinding(input)) return;
      event.preventDefault();
      this.arm(!this.armed);
    });
    this.bar.webContents.on("before-input-event", (event, input) => {
      if (!isCaptureBinding(input)) return;
      event.preventDefault();
      this.arm(!this.armed);
    });
    // The bar goes nowhere either: it is a Bridge document and Bridge's own
    // window's rule holds over every one of them.
    this.bar.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
    this.bar.webContents.on("will-navigate", (event) => event.preventDefault());
    this.bar.webContents.on("did-finish-load", () => this.tell());
    // A page the question moved to, or one that moved under it: its place is looked for again.
    contents.on("did-finish-load", () => void this.locate());
    contents.on("did-navigate-in-page", () => void this.locate());
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
