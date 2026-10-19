// Every capture window open — #1294. **A Studio's, one per Run**, and opening again raises the one that is open: two
// windows on one origin would be two things a person has to tell apart by their title. **Every walk a Job or a pull
// request shows is in one walk window** (`walk-window.ts`), its queue held to the Jobs' waiting items (`queue.ts`).

import tokens from "@armada/tokens/tokens.json";

import type { BridgeState } from "../../shared/bridge";
import type { CaptureOpened, SessionRecord, WalkStep, WindowTake } from "@armada/protocol";
import { originsOfWindow, readSteps, stepsOfWindow, takesOfWindow, titleOfWindow } from "../session-file";
import type { ShownReviewWalk, ShownReviewWalkOutcome } from "../../shared/review-walk";
import { stepsOfRequests } from "../../shared/review-walk";
import { isPinned, passThrough, pinned, webOrigin } from "./address";
import { sessionReadiness } from "./readiness";
import type { PanelSubject, ReviewWalkOwner } from "./layout";
import { jobName, questionOf, reviewWalkKey, sessionWalkKey, standingWalks } from "./queue";
import { startRunner } from "./runner";
import { WalkWindow, type Queued, type WalkWindowBoard } from "./walk-window";
import { CaptureWindow, type CaptureBoard, type LandsOn, type PanelKept, type WalkSizes } from "./window";

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

export class CaptureWindows {
  private readonly board: CaptureBoard;
  /** The Studio windows open, one per Run. */
  private readonly open = new Map<string, CaptureWindow>();
  /** The one walk window, holding every Job's walk and every pull request's. `null`: none is up. */
  private walks: WalkWindow | null = null;
  /** The sessions main last published, which each Job's walk reads its waiting items and its question off. */
  private sessions: readonly SessionRecord[] | null = null;
  /** Whether the panel is drawn, as last left on a window of each kind. Drawn until a press folds it. */
  private readonly panelOpen = { session: true };
  /** Where each `window_ask` was asked, by its waiting item, as Bridge heard it asked: the walk it marks. */
  private readonly askedIn = new Map<string, string>();
  /** The walks approved here, by when each was shown, so the publish before Fleet settles one does not queue it again. */
  private readonly approved = new Map<string, string>();

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
   * A Job showed a page: **it joins the walk window**, oldest shown first, and opens the window behind and without focus
   * where none is up. A walk already in the window stays where it is; shown again with other takes, other steps, other
   * origins or a sign-in to run, its page is made afresh in its place. **`pressed`**: the owner pressed it open in Bridge,
   * so the window comes forward on it.
   *
   * **Any `http` or `https` address**, unlike a server's window; the origin it is pinned to is the address's own, since the
   * Job named it. **`takes` are several pages of one change**, the first at `url`: the walk swaps among exactly those.
   * **`steps` are a scripted walk of the page**, which the window plays itself. **`origins` are the other origins it may
   * pass through**, each held to a bare `http` or `https` origin here. **`signIn` are addresses it loads, in order, before
   * the page**, in the repository's cookie jar; they are a secret and go no further than the page's own first load.
   */
  openForSession(
    sessionId: string,
    title: string,
    url: string,
    takes: readonly WindowTake[] = [],
    origins: readonly string[] = [],
    signIn: readonly string[] = [],
    steps: readonly WalkStep[] = [],
    pressed = false,
  ): CaptureOpened {
    const origin = webOrigin(url);
    if (origin === null || takes.some((one) => webOrigin(one.url) === null)) return { ok: false, why: "no_address" };
    const through = passThrough(origins);
    const key = sessionWalkKey(sessionId, url);
    const standing = this.walks?.open === true ? this.walks.find(key) : undefined;
    const walk = this.sessionWalk(sessionId, title, url, origin, through, takes, signIn, steps, standing?.since ?? this.sinceOf(sessionId, url));
    if (standing === undefined) return this.join(walk, pressed);
    // A page shown again with a sign-in is a sign-in to run, so its page is made afresh.
    if (standing.shape !== walk.shape || signIn.length > 0) this.walks?.replace(walk);
    if (pressed) this.toScreen(key);
    return { ok: true };
  }

  /**
   * A pull request's walk joins the walk window, as a Job's does. **No waiting item stands for it**: Bridge keeps it from
   * when it is held until it is approved or the window closes, either of which closes its held walk. A web walk loads
   * the place it was written against; **an API walk loads a runner of Armada's own on loopback** (`runner.ts`) whose
   * buttons its steps press. The page is pinned to its own origin and no other, and a note goes to the Session holding the
   * pull request, else stays in Armada. A later walk of the same pull request takes the earlier one's place.
   */
  async openForReview(shown: ShownReviewWalk): Promise<ShownReviewWalkOutcome> {
    const key = reviewWalkKey(shown.manifestId, shown.number);
    const standing = this.walks?.open === true ? this.walks.find(key) : undefined;
    if (standing !== undefined && standing.held?.walkId === shown.walkId) return { ok: true };
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
    // One cookie jar per repository, as a Job's: a sign-in on a preview holds for the next walk of that repository.
    const pin = { run: key, name: shown.title, origin, url, manifestId: shown.manifestId, through: [] };
    const review: ReviewWalkOwner = {
      walkId: shown.walkId,
      manifestId: shown.manifestId,
      number: shown.number,
      title: shown.title,
      address: shown.address,
      ...(shown.holder === undefined ? {} : { holder: shown.holder }),
    };
    const stopping = runner;
    const name = shown.title.replace(/^#\d+\s*/, "");
    const walk: Queued = {
      key,
      since: standing?.since ?? new Date().toISOString(),
      url,
      make: (seat) => new CaptureWindow(pin, { review }, this.board, sizes(), null, [], [], steps, seat),
      walk: null,
      seen: false,
      left: () => {
        void stopping?.stop();
        this.board.reviewClosed(review);
      },
      describe: () => ({
        id: key,
        name: name === "" ? shown.title : name,
        pull: shown.number,
        title: shown.title,
        address: shown.address,
        aim: `Notes go to ${shown.holder?.title ?? "Armada"}`,
      }),
      shape: shown.walkId,
      held: { manifestId: shown.manifestId, walkId: shown.walkId },
    };
    if (standing === undefined) {
      this.join(walk, false);
      return { ok: true };
    }
    // The earlier walk of this pull request is over: its held walk closes, and its runner stops.
    standing.left();
    this.walks?.replace(walk);
    return { ok: true };
  }

  /** The held walks in the walk window now, so Bridge can close them at Fleet as it quits. */
  heldWalks(): { manifestId: string; walkId: string }[] {
    return this.walks?.open === true ? this.walks.queued.flatMap((one) => (one.held === undefined ? [] : [one.held])) : [];
  }

  /**
   * A Job asked about a place on its walk. **The walk on screen comes forward and shows it**; a walk of the window's that
   * is not on screen takes the ask mark on its entry and the page stays. A walk not in the window joins it, and one with
   * no window up opens it on that walk. `window` is the address the Job showed, and `itemId` the waiting item the question
   * is, where it is known; the question itself, and the page it moves to, are read off what Fleet published.
   */
  askedFor(sessionId: string, title: string, window: string, _place: string, itemId?: string): CaptureOpened {
    if (itemId !== undefined) this.askedIn.set(itemId, window);
    const queued = this.walks?.open === true && this.walks.find(sessionWalkKey(sessionId, window)) !== undefined;
    const record = this.record(sessionId);
    const opened = queued
      ? ({ ok: true } as const)
      : this.openForSession(sessionId, title, window, takesOfWindow(record, window), originsOfWindow(record, window), [], stepsOfWindow(record, window));
    this.ask();
    this.walks?.refresh();
    return opened;
  }

  /** The addresses of the Job walks in the walk window now. */
  sessionAddresses(): string[] {
    return this.walks?.open === true ? this.walks.queued.flatMap((one) => (one.sessionId === undefined ? [] : [one.url])) : [];
  }

  /** The owner chose another walk on the bar, by its place: only the walk window's own bar is answered. */
  walkTo(contents: Electron.WebContents, at: number): void {
    if (this.walks?.open === true && this.walks.fromBar(contents) !== null) this.walks.jump(at);
  }

  /** The walk on screen shows its Job's oldest open question asked about it, or none. */
  private ask(): void {
    const on = this.walks?.open === true ? this.walks.onScreen : null;
    if (on === null || on.walk === null || on.sessionId === undefined || this.sessions === null) return;
    on.walk.question(questionOf(this.record(on.sessionId), on.walk.origins, { url: on.url, askedIn: this.askedIn }));
  }

  /**
   * Fold what main published: a Run that has stopped serving ends capture in its own window, and the bar says the run
   * ended. **The walk window follows the Jobs' waiting items**: a walk that stands joins it, and one whose item went,
   * settled elsewhere, leaves it.
   */
  changed(state: BridgeState): void {
    if (state.sessions.state === "read") this.sessions = state.sessions.sessions;
    for (const [serverId, window] of this.open) {
      if (!window.open) {
        this.open.delete(serverId);
        continue;
      }
      const server = state.servers.servers.find((one) => one.id === serverId);
      if (server === undefined || server.phase === "exited") window.ended();
    }
    this.follow();
    const on = this.walks?.open === true ? this.walks.onScreen : null;
    if (on?.walk != null && on.sessionId !== undefined) on.walk.setReadiness(sessionReadiness(this.record(on.sessionId), on.url));
    this.ask();
    this.walks?.refresh();
  }

  /** The walk window's queue held to the Jobs' waiting items. Nothing while no walk window is up: a walk that stands opens none. */
  private follow(): void {
    const walks = this.walks;
    if (walks === null || !walks.open || this.sessions === null) return;
    const standing = standingWalks(this.sessions);
    const since = new Map(standing.map((one) => [one.key, one.since]));
    for (const [key, at] of this.approved) if (since.get(key) !== at) this.approved.delete(key);
    for (const one of [...walks.queued]) {
      if (one.sessionId === undefined) continue;
      if (since.has(one.key)) one.seen = true;
      else if (one.seen && walks.open) walks.remove(one.key);
    }
    for (const one of standing) {
      if (!walks.open || walks.find(one.key) !== undefined || this.approved.has(one.key)) continue;
      const record = this.record(one.sessionId);
      const origin = webOrigin(one.url);
      if (origin === null) continue;
      const walk = this.sessionWalk(one.sessionId, titleOfWindow(record, one.url) ?? one.title, one.url, origin, passThrough(originsOfWindow(record, one.url)), takesOfWindow(record, one.url), [], stepsOfWindow(record, one.url), one.since);
      walks.add({ ...walk, seen: true }, false);
    }
  }

  /** A walk into the walk window, opening it behind on that walk where none is up, with every walk standing beside it. */
  private join(walk: Queued, pressed: boolean): CaptureOpened {
    if (this.walks === null || !this.walks.open) {
      const walks = new WalkWindow(sizes(), this.walkBoard(), walk.sessionId === undefined ? null : { kind: "session", id: walk.sessionId });
      this.walks = walks;
      walks.add(walk, true);
      this.follow();
      walks.reveal(pressed);
      return { ok: true };
    }
    this.walks.add(walk, false);
    if (pressed) this.toScreen(walk.key);
    return { ok: true };
  }

  /** The owner pressed a walk open: the walk window comes forward on it. */
  private toScreen(key: string): void {
    const walks = this.walks;
    if (walks === null || !walks.open) return;
    walks.jump(walks.queued.findIndex((one) => one.key === key));
    walks.raise();
  }

  /** When Fleet says a Job's walk at `url` was shown, or now where it says nothing yet. */
  private sinceOf(sessionId: string, url: string): string {
    const record = this.record(sessionId);
    const item = record === undefined ? undefined : standingWalks([record]).find((one) => one.url === url);
    return item?.since ?? new Date().toISOString();
  }

  private record(sessionId: string): SessionRecord | undefined {
    return this.sessions?.find((one) => one.id === sessionId);
  }

  /**
   * A Job's walk, its page made the first time it comes on screen. **The sign-in is held until then and let go of once the
   * page has it**: one jar per repository, so a Job's sign-in, or the owner's, holds for its later walks.
   */
  private sessionWalk(
    sessionId: string,
    title: string,
    url: string,
    origin: string,
    through: readonly string[],
    takes: readonly WindowTake[],
    signIn: readonly string[],
    steps: readonly WalkStep[],
    since: string,
  ): Queued {
    const key = sessionWalkKey(sessionId, url);
    const origins = [origin, ...takes.flatMap((one) => webOrigin(one.url) ?? [])];
    let held = signIn;
    return {
      key,
      since,
      sessionId,
      url,
      make: (seat) => {
        const record = this.record(sessionId);
        const pin = { run: key, name: title, origin, url, manifestId: record?.manifest_id ?? null, through };
        // The Job by its own name, as the Jobs list calls it: `title` is the window's.
        const landsOn = { session: { id: sessionId, title: jobName(record, sessionId) } };
        const walk = new CaptureWindow(pin, landsOn, this.board, sizes(), this.kept(landsOn), takes, held, steps, seat);
        held = [];
        walk.setReadiness(sessionReadiness(record, url));
        return walk;
      },
      walk: null,
      seen: false,
      left: () => this.board.sessionClosed(sessionId, url),
      describe: (onScreen) => {
        const record = this.record(sessionId);
        const job = jobName(record, sessionId);
        const readiness = sessionReadiness(record, url);
        const asked = onScreen ? null : questionOf(record, origins, { url, askedIn: this.askedIn });
        return {
          id: key,
          name: job,
          title,
          address: url,
          aim: `Notes go to ${job}`,
          ...(readiness === undefined ? {} : { readiness }),
          ...(asked === null ? {} : { asking: asked.question }),
        };
      },
      shape: JSON.stringify({ takes, steps, through }),
    };
  }

  /** What the walk window tells, and what it is told. */
  private walkBoard(): WalkWindowBoard {
    return {
      // A walk that comes on screen shows its question without taking focus: the owner moved to it, or nobody did.
      shown: (walk) => {
        if (walk.walk === null || walk.sessionId === undefined) return;
        walk.walk.setReadiness(sessionReadiness(this.record(walk.sessionId), walk.url));
        walk.walk.question(questionOf(this.record(walk.sessionId), walk.walk.origins, { url: walk.url, askedIn: this.askedIn }), false);
      },
      focused: (on) => this.board.focused(WALKS, on),
      changed: () => this.board.windowsChanged?.(),
      closed: () => this.board.windowsChanged?.(),
      approved: (walk) => {
        if (walk.sessionId !== undefined) this.approved.set(walk.key, walk.since);
      },
    };
  }

  /** Where a window of this kind keeps whether its panel is drawn. `null`: a Studio's, which has none. */
  private kept(landsOn: LandsOn): PanelKept | null {
    const kind = "session" in landsOn ? "session" : null;
    if (kind === null) return null;
    return { open: this.panelOpen[kind], keep: (open) => void (this.panelOpen[kind] = open) };
  }

  /** The Job an IPC call's panel holds: the walk window's walk on screen. `null` where the call came from anything but its panel, or the walk has no Job. */
  panelFrom(contents: Electron.WebContents): PanelSubject | null {
    if (this.walks?.open === true && this.walks.isPanel(contents)) {
      const sessionId = this.walks.onScreen?.sessionId;
      return sessionId === undefined ? null : { kind: "session", id: sessionId };
    }
    return null;
  }

  /** Every panel's document, to be told the state Bridge's own windows are told. */
  panels(): Electron.WebContents[] {
    const panel = this.walks?.open === true ? this.walks.panelContents : null;
    return panel === null ? [] : [panel];
  }

  /** The window an IPC call came from, or `null` where it came from anything else. In the walk window, the walk on screen. */
  from(contents: Electron.WebContents): CaptureWindow | null {
    const walk = this.walks?.open === true ? this.walks.fromBar(contents) : null;
    if (walk !== null) return walk;
    for (const window of this.open.values()) {
      if (window.open && window.owns(contents)) return window;
    }
    return null;
  }

  closeAll(): void {
    this.walks?.close();
    this.walks = null;
    for (const window of this.open.values()) window.close();
    this.open.clear();
  }
}

/** What the walk window is known by when it takes focus, beside the server Runs' windows. */
const WALKS = "walks";

export { questionOf };

