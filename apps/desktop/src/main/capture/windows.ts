// Every capture window open, one per Run — #1294.
//
// **One per Run, and opening again raises the one that is open.** Two windows
// on one origin would be two things a person has to tell apart by their title.

import tokens from "@armada/tokens/tokens.json";

import type { BridgeState } from "../../shared/bridge";
import type { CaptureOpened, SessionRecord, WalkStep, WindowTake } from "@armada/protocol";
import { addressOf, cleanTitle } from "@armada/screens/src/sessions-wire";
import { readSteps } from "../session-file";
import type { ShownReviewWalk, ShownReviewWalkOutcome } from "../../shared/review-walk";
import { stepsOfRequests } from "../../shared/review-walk";
import { isPinned, passThrough, pinned, webOrigin } from "./address";
import { sessionReadiness } from "./readiness";
import type { PanelSubject } from "./layout";
import { startRunner } from "./runner";
import { CaptureWindow, type CaptureBoard, type LandsOn, type PanelKept, type WalkSizes, type WindowQuestion } from "./window";

/** One spacing token, in CSS pixels. */
function token(name: string): number {
  const found = tokens.tokens.find((one) => one.name === name);
  return found === undefined ? 0 : Number.parseInt(found.value, 10);
}

/**
 * How the window is sized. **The bar is the title row's own composition**, which `TitleBar.css` spells
 * as the same three tokens, so the bar and Bridge's own title row are one height. **The panel is a
 * sheet's width**, and the page beside it keeps what a dock leaves of the work, `--w-work-min`.
 */
function sizes(): WalkSizes {
  return {
    bar: token("--space-8") + token("--space-2") + token("--space-1"),
    panel: token("--w-sheet"),
    floor: token("--w-work-min"),
  };
}

/** What a Session's window is keyed by, so it cannot be mistaken for a server Run's id. */
const SESSION_KEY = "session:";
/** What a pull request's walk is keyed by, so it is neither a server Run's nor a Session's. */
const REVIEW_KEY = "review:";

export class CaptureWindows {
  private readonly board: CaptureBoard;
  private readonly open = new Map<string, CaptureWindow>();
  /** The sessions main last published, which each Session's window reads its open question off. */
  private sessions: readonly SessionRecord[] | null = null;
  /** Whether the panel is drawn, as last left on a window of each kind. Drawn until a press folds it. */
  private readonly panelOpen = { session: true };

  constructor(board: CaptureBoard) {
    this.board = board;
  }

  /**
   * Open the capture window on one server Run, or raise the one already on it.
   *
   * **The address is resolved off the live holder Fleet published**, never off
   * the string the renderer sent — `servers.ts`'s rule, which loading a link in
   * a window is strictly more than.
   */
  openOn(
    state: BridgeState,
    serverId: string,
    url: string,
    studio: { id: string; name: string | null } | null,
  ): CaptureOpened {
    // A server's window needs its Studio, for what is pointed at there to land on.
    const landsOn: LandsOn | null = studio !== null ? { studio } : null;
    if (landsOn === null) return { ok: false, why: "no_studio" };
    const standing = this.open.get(serverId);
    if (standing !== undefined && standing.open) {
      standing.raise();
      return { ok: true };
    }
    const pin = pinned(state, serverId, url);
    if (!isPinned(pin)) return pin;
    const opened = new CaptureWindow(pin, landsOn, this.board, sizes(), this.kept(landsOn));
    this.open.set(serverId, opened);
    return { ok: true };
  }

  /**
   * Open the window on a page a Session showed, or raise the one it has open on that address.
   *
   * **One per Session**, keyed beside the server Runs'. A Session that shows another address
   * replaces its window rather than opening a second. **Any `http` or `https` address**, unlike a
   * server's window; the origin it is pinned to is the address's own, since the
   * Session named it and there is no live holder to resolve it against as a server's link has.
   * **`takes` are several pages of one change**, the first at `url`: the window swaps among exactly those.
   * **`steps` are a scripted walk of the page**, which the window plays itself.
   * **`origins` are the other origins it may pass through**, each held to a bare `http` or `https` origin
   * here. **`signIn` are addresses it loads, in order, before the page**, in the repository's cookie jar;
   * they are a secret and go no further than the window's own load.
   */
  openForSession(
    sessionId: string,
    title: string,
    url: string,
    takes: readonly WindowTake[] = [],
    origins: readonly string[] = [],
    signIn: readonly string[] = [],
    steps: readonly WalkStep[] = [],
  ): CaptureOpened {
    const origin = webOrigin(url);
    if (origin === null || takes.some((one) => webOrigin(one.url) === null)) return { ok: false, why: "no_address" };
    const through = passThrough(origins);
    const key = `${SESSION_KEY}${sessionId}`;
    const standing = this.open.get(key);
    if (standing !== undefined && standing.open) {
      // A page shown again with a sign-in is a sign-in to run, so it opens afresh.
      if (standing.address === url && standing.showing(takes, steps) && standing.passes(through) && signIn.length === 0) {
        standing.raise();
        return { ok: true };
      }
      standing.close();
    }
    // The Session by its own name, as the Sessions list calls it: `title` is the window's, and a
    // window shown with no title is titled by its address.
    const record = this.sessions?.find((one) => one.id === sessionId);
    // One cookie jar per repository: a Session's sign-in, or the person's, holds for its later walks.
    const pin = { run: key, name: title, origin, url, manifestId: record?.manifest_id ?? null, through };
    const landsOn = { session: { id: sessionId, title: cleanTitle(record?.title) ?? addressOf(sessionId) } };
    const window = new CaptureWindow(pin, landsOn, this.board, sizes(), this.kept(landsOn), takes, signIn, steps);
    this.open.set(key, window);
    window.setReadiness(sessionReadiness(record, url));
    this.board.windowsChanged?.();
    // A window opened by a press on a waiting question shows it as one opened by its arrival does.
    if (this.sessions !== null) window.question(questionOf(this.sessions.find((one) => one.id === sessionId), window.origins));
    return { ok: true };
  }

  /**
   * Open a pull request's walk, or raise the one already open for it. **One a pull request**: a walk held again replaces the earlier
   * (closing it closes its held place). A web walk loads the place it was written against; **an API walk loads a runner of Armada's
   * own on loopback** (`runner.ts`) whose buttons its steps press, so both play in the one window. The page is pinned to its own
   * origin and no other, and a note goes to the Session holding the pull request, else stays in Armada.
   */
  async openForReview(shown: ShownReviewWalk): Promise<ShownReviewWalkOutcome> {
    const key = `${REVIEW_KEY}${shown.manifestId}#${shown.number}`;
    const standing = this.open.get(key);
    if (standing !== undefined && standing.open) {
      if (standing.walkId === shown.walkId) {
        standing.raise();
        return { ok: true };
      }
      standing.close();
    }
    const requests = shown.requests;
    const steps = requests !== undefined ? stepsOfRequests(requests) : readSteps(shown.steps);
    if (steps.length === 0) return { ok: false, why: "no_steps" };
    if (webOrigin(shown.address) === null) return { ok: false, why: "no_address" };
    let runner: Awaited<ReturnType<typeof startRunner>> | null = null;
    if (requests !== undefined) {
      try {
        runner = await startRunner(shown.address, requests);
      } catch {
        return { ok: false, why: "runner_failed" };
      }
    }
    const url = runner === null ? shown.address : runner.url;
    const origin = webOrigin(url);
    if (origin === null) {
      await runner?.stop();
      return { ok: false, why: "no_address" };
    }
    // One cookie jar per repository, as a Session's: a sign-in on a preview holds for the next walk of that repository.
    const pin = { run: key, name: shown.title, origin, url, manifestId: shown.manifestId, through: [] };
    const review = {
      walkId: shown.walkId,
      manifestId: shown.manifestId,
      number: shown.number,
      title: shown.title,
      address: shown.address,
      ...(shown.holder === undefined ? {} : { holder: shown.holder }),
    };
    const stopping = runner;
    const board: CaptureBoard =
      stopping === null
        ? this.board
        : { ...this.board, reviewClosed: (closed) => { void stopping.stop(); this.board.reviewClosed(closed); } };
    this.open.set(key, new CaptureWindow(pin, { review }, board, sizes(), null, [], [], steps));
    return { ok: true };
  }

  /** The walks held open by windows now, so Bridge can close them at Fleet as it quits. */
  heldWalks(): { manifestId: string; walkId: string }[] {
    return [...this.open.values()].flatMap((window) => (window.open && window.walkId !== null && window.walkManifestId !== null ? [{ manifestId: window.walkManifestId, walkId: window.walkId }] : []));
  }

  /**
   * A Session asked about a place on its walk: **its window comes forward, and opens where it is
   * closed** or is on another origin. `window` is the address the Session showed, which the window
   * opens on; the question itself, and the page it moves to, are read off what Fleet published.
   */
  askedFor(sessionId: string, title: string, window: string, place: string): CaptureOpened {
    const standing = this.open.get(`${SESSION_KEY}${sessionId}`);
    const here = standing !== undefined && standing.open && standing.origins.includes(webOrigin(place) ?? "");
    const opened = here ? ({ ok: true } as const) : this.openForSession(sessionId, title, window);
    this.ask();
    return opened;
  }

  /** The addresses of the Session windows open now. */
  sessionAddresses(): string[] {
    return [...this.open].filter(([key, window]) => key.startsWith(SESSION_KEY) && window.open).map(([, window]) => window.address);
  }

  /** Every Session window shows its Session's oldest open question on its own origin, or none. */
  private ask(): void {
    if (this.sessions === null) return;
    for (const [key, window] of this.open) {
      if (!key.startsWith(SESSION_KEY) || !window.open) continue;
      const record = this.sessions.find((one) => one.id === key.slice(SESSION_KEY.length));
      window.question(questionOf(record, window.origins));
    }
  }

  /**
   * Fold what main published: a Run that has stopped serving ends capture in
   * its window, and the bar says the run ended.
   */
  changed(state: BridgeState): void {
    if (state.sessions.state === "read") this.sessions = state.sessions.sessions;
    for (const [serverId, window] of this.open) {
      if (!window.open) {
        this.open.delete(serverId);
        continue;
      }
      // A pull request's walk is no Run either, and nothing Fleet publishes about servers ends it.
      if (serverId.startsWith(REVIEW_KEY)) continue;
      // A Session's page is no Run, so nothing Fleet publishes about servers ends it.
      if (serverId.startsWith(SESSION_KEY)) {
        window.setReadiness(sessionReadiness(this.sessions?.find((one) => one.id === serverId.slice(SESSION_KEY.length)), window.address));
        continue;
      }
      const server = state.servers.servers.find((one) => one.id === serverId);
      if (server === undefined || server.phase === "exited") window.ended();
    }
    this.ask();
  }

  /** Where a window of this kind keeps whether its panel is drawn. `null`: a Studio's, which has none. */
  private kept(landsOn: LandsOn): PanelKept | null {
    const kind = "session" in landsOn ? "session" : null;
    if (kind === null) return null;
    return { open: this.panelOpen[kind], keep: (open) => void (this.panelOpen[kind] = open) };
  }

  /** The Session an IPC call's panel holds, or `null` where the call came from anything but a walk window's panel. */
  panelFrom(contents: Electron.WebContents): PanelSubject | null {
    for (const window of this.open.values()) {
      const subject = window.open ? window.panelOf(contents) : null;
      if (subject !== null) return subject;
    }
    return null;
  }

  /** Every panel's document, to be told the state Bridge's own windows are told. */
  panels(): Electron.WebContents[] {
    return [...this.open.values()].flatMap((window) => (window.open && window.panelContents !== null ? [window.panelContents] : []));
  }

  /** The window an IPC call came from, or `null` where it came from anything else. */
  from(contents: Electron.WebContents): CaptureWindow | null {
    for (const window of this.open.values()) {
      if (window.open && window.owns(contents)) return window;
    }
    return null;
  }

  closeAll(): void {
    for (const window of this.open.values()) window.close();
    this.open.clear();
  }
}

/**
 * The question a Session's window shows: its oldest open `window_ask` item on the origin of its page or of any of its takes.
 * Pure, so the rule is tested without a window.
 */
export function questionOf(record: SessionRecord | undefined, origins: readonly string[]): WindowQuestion | null {
  for (const item of record?.waiting_for ?? []) {
    if (item.source !== "window_ask" || item.place === undefined) continue;
    if (!origins.includes(webOrigin(item.place.url) ?? "")) continue;
    return { id: item.id, question: item.text, options: item.options ?? [], place: item.place };
  }
  return null;
}
