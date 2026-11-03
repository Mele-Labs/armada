// Every capture window open — #1294. **A Studio's, one per Run**, and opening again raises the one that is open: two
// windows on one origin would be two things a person has to tell apart by their title. **A pull request's walk is in the
// one walk window** (`walk-window.ts`). **A Job's walk is a tab of the Job in Bridge's own window** (`job-walks.ts`): no window,
// nothing raised, and its set held to the Jobs' waiting items (`queue.ts`).

import tokens from "@armada/tokens/tokens.json";

import type { BaseWindow, BrowserWindow } from "electron";

import type { BridgeState } from "../../shared/bridge";
import type { CaptureOpened, SessionRecord, WalkStep, WindowTake } from "@armada/protocol";
import {
  briefOfWindow,
  originsOfWindow,
  readSteps,
  stepsOfWindow,
  takesOfWindow,
  titleOfWindow,
} from "../session-file";
import type { ShownReviewWalk, ShownReviewWalkOutcome } from "../../shared/review-walk";
import { stepsOfRequests } from "../../shared/review-walk";
import { isPinned, passThrough, pinned, webOrigin } from "./address";
import { sessionReadiness } from "./readiness";
import type { PanelSubject, ReviewWalkOwner } from "./layout";
import {
  jobName,
  questionOf,
  reviewWalkKey,
  sessionWalkKey,
  standingWalks,
  type StandingWalk,
} from "./queue";
import { startRunner } from "./runner";
import { JobWalks, type JobWalk } from "./job-walks";
import { forwardTo } from "./pane-keys";
import { WalkWindow, type Queued, type WalkWindowBoard } from "./walk-window";
import {
  CaptureWindow,
  type CaptureBoard,
  type LandsOn,
  type WalkSizes,
} from "./window";

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
  /** The one walk window, holding every pull request's walk. `null`: none is up. */
  private walks: WalkWindow | null = null;
  /** Every Job's walk, drawn as a tab of the Job in Bridge's own window. */
  private readonly jobWalks: JobWalks;
  /** The sessions main last published, which each Job's walk reads its waiting items and its question off. */
  private sessions: readonly SessionRecord[] | null = null;
  /** Where each `window_ask` was asked, by its waiting item, as Bridge heard it asked: the walk it marks. */
  private readonly askedIn = new Map<string, string>();
  /** The walks approved here, by when each was shown, so the publish before Fleet settles one does not queue it again. */
  private readonly approved = new Map<string, string>();

  /** `host`: Bridge's own window, or `null` while none is up, which the Jobs' walks outlive. */
  constructor(board: CaptureBoard, host: () => BrowserWindow | null = () => null) {
    this.board = board;
    this.jobWalks = new JobWalks(host, (input) => forwardTo(host()?.webContents ?? null, input), {
      shown: (walk) => this.refresh(walk),
      approved: (walk) => void this.approved.set(walk.key, walk.since),
    });
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
    studio: {
      id: string;
      name: string | null;
    } | null,
  ): CaptureOpened {
    // A server's window needs its Studio, for what is pointed at there to land on.
    const landsOn: LandsOn | null =
      studio !== null
        ? {
            studio,
          }
        : null;
    if (landsOn === null)
      return {
        ok: false,
        why: "no_studio",
      };
    const standing = this.open.get(serverId);
    if (standing !== undefined && standing.open) {
      standing.raise();
      return {
        ok: true,
      };
    }
    const pin = pinned(state, serverId, url);
    if (!isPinned(pin)) return pin;
    const opened = new CaptureWindow(pin, landsOn, this.board, sizes(), null);
    this.open.set(serverId, opened);
    return {
      ok: true,
    };
  }

  /**
   * A Job showed a page: **its walk is a tab of the Job in Bridge's own window**, with no window of its own and nothing raised or focused.
   * A walk already shown stays where it is; shown again with other takes, other steps, other origins, another brief or a sign-in to run, its
   * page is made afresh in its place. Its page is made when its pane first reports a frame (`job-walks.ts`).
   *
   * **Any `http` or `https` address**, unlike a server's window; the origin it is pinned to is the address's own, since the
   * Job named it. **`takes` are several pages of one change**, the first at `url`: the walk swaps among exactly those.
   * **`steps` are a scripted walk of the page**, which Bridge plays itself. **`origins` are the other origins it may
   * pass through**, each held to a bare `http` or `https` origin here. **`signIn` are addresses it loads, in order, before
   * the page**, in the repository's cookie jar; they are a secret and go no further than the page's own first load.
   * **`brief` is what the Job changed and what to look at**, which the bar draws behind its `?`.
   */
  openForSession(
    sessionId: string,
    title: string,
    url: string,
    takes: readonly WindowTake[] = [],
    origins: readonly string[] = [],
    signIn: readonly string[] = [],
    steps: readonly WalkStep[] = [],
    brief?: string,
  ): CaptureOpened {
    const origin = webOrigin(url);
    if (origin === null || takes.some((one) => webOrigin(one.url) === null))
      return {
        ok: false,
        why: "no_address",
      };
    const through = passThrough(origins);
    const key = sessionWalkKey(sessionId, url);
    const standing = this.jobWalks.find(key);
    const walk = this.sessionWalk(
      sessionId,
      title,
      url,
      origin,
      through,
      takes,
      signIn,
      steps,
      standing?.since ?? this.sinceOf(sessionId, url),
      brief,
    );
    if (standing === undefined) this.jobWalks.add(walk);
    // A page shown again with a sign-in is a sign-in to run, so its page is made afresh.
    else if (standing.shape !== walk.shape || signIn.length > 0) this.jobWalks.replace(walk);
    return {
      ok: true,
    };
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
    if (standing !== undefined && standing.held?.walkId === shown.walkId)
      return {
        ok: true,
      };
    const requests = shown.requests;
    const steps = requests !== undefined ? stepsOfRequests(requests) : readSteps(shown.steps);
    if (steps.length === 0)
      return {
        ok: false,
        why: "no_steps",
      };
    if (webOrigin(shown.address) === null)
      return {
        ok: false,
        why: "no_address",
      };
    let runner: Awaited<ReturnType<typeof startRunner>> | null = null;
    if (requests !== undefined) {
      try {
        runner = await startRunner(shown.address, requests);
      } catch {
        return {
          ok: false,
          why: "runner_failed",
        };
      }
    }
    const url = runner === null ? shown.address : runner.url;
    const origin = webOrigin(url);
    if (origin === null) {
      await runner?.stop();
      return {
        ok: false,
        why: "no_address",
      };
    }
    // One cookie jar per repository, as a Job's: a sign-in on a preview holds for the next walk of that repository.
    const pin = {
      run: key,
      name: shown.title,
      origin,
      url,
      manifestId: shown.manifestId,
      through: [],
    };
    const review: ReviewWalkOwner = {
      walkId: shown.walkId,
      manifestId: shown.manifestId,
      number: shown.number,
      title: shown.title,
      address: shown.address,
      ...(shown.holder === undefined
        ? {}
        : {
            holder: shown.holder,
          }),
    };
    const stopping = runner;
    const name = shown.title.replace(/^#\d+\s*/, "");
    const walk: Queued = {
      key,
      since: standing?.since ?? new Date().toISOString(),
      url,
      make: (seat) =>
        new CaptureWindow(
          pin,
          {
            review,
          },
          this.board,
          sizes(),
          null,
          [],
          [],
          steps,
          seat,
        ),
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
      held: {
        manifestId: shown.manifestId,
        walkId: shown.walkId,
      },
    };
    if (standing === undefined) {
      this.join(walk);
      return {
        ok: true,
      };
    }
    // The earlier walk of this pull request is over: its held walk closes, and its runner stops.
    standing.left();
    this.walks?.replace(walk);
    return {
      ok: true,
    };
  }

  /** The held walks in the walk window now, so Bridge can close them at Fleet as it quits. */
  heldWalks(): {
    manifestId: string;
    walkId: string;
  }[] {
    return this.walks?.open === true
      ? this.walks.queued.flatMap((one) =>
          one.held === undefined
            ? []
            : [
                one.held,
              ],
        )
      : [];
  }

  /**
   * A Job asked about a place on its walk. **The walk shows it on its bar**, wherever its tab is; a walk this Bridge has not heard of is made
   * first, from what Fleet published. `window` is the address the Job showed, and `itemId` the waiting item the question is, where it is
   * known; the question itself is read off what Fleet published.
   */
  askedFor(
    sessionId: string,
    title: string,
    window: string,
    _place: string,
    itemId?: string,
  ): CaptureOpened {
    if (itemId !== undefined) this.askedIn.set(itemId, window);
    const record = this.record(sessionId);
    const opened =
      this.jobWalks.find(sessionWalkKey(sessionId, window)) !== undefined
        ? ({
            ok: true,
          } as const)
        : this.openForSession(
            sessionId,
            title,
            window,
            takesOfWindow(record, window),
            originsOfWindow(record, window),
            [],
            stepsOfWindow(record, window),
            briefOfWindow(record, window),
          );
    this.ask();
    return opened;
  }

  /** The owner chose another walk on the bar, by its place: only the walk window's own bar is answered. */
  walkTo(contents: Electron.WebContents, at: number): void {
    if (this.walks?.open === true && this.walks.fromBar(contents) !== null) this.walks.jump(at);
  }

  /** Every walk of a Job's shows its Job's oldest open question asked about it, or none. */
  private ask(): void {
    if (this.sessions === null) return;
    for (const one of this.jobWalks.queued) this.asking(one);
  }

  /** What a walk is told of its Job: whether the page is ready to walk, and the question asked about it. Never with focus, and nothing is raised. */
  private refresh(walk: JobWalk): void {
    if (walk.walk === null) return;
    walk.walk.setReadiness(sessionReadiness(this.record(walk.sessionId), walk.url));
    this.asking(walk);
  }

  private asking(walk: JobWalk): void {
    if (walk.walk === null) return;
    walk.walk.question(
      questionOf(this.record(walk.sessionId), walk.walk.origins, {
        url: walk.url,
        askedIn: this.askedIn,
      }),
      false,
    );
  }

  /**
   * Fold what main published: a Run that has stopped serving ends capture in its own window, and the bar says the run
   * ended. **The Jobs' walks follow their waiting items**: a walk that stands is held, and one whose item went leaves,
   * unless its page reads working.
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
    for (const one of this.jobWalks.queued) this.refresh(one);
    this.walks?.refresh();
  }

  /**
   * The Jobs' walks held to their waiting items. **A walk whose item went while its page reads working stays**: Fleet drops the item while
   * the Job works on notes sent, and brings it back when the Job shows the page again. It leaves once its page reads ready with the item
   * still gone. Back with a newer `since`, it is made afresh in its place, as the page and what it was shown with may have moved. A walk
   * that stands with no tab held (Bridge was restarted) is held here too, and its page is made when its tab reports a frame.
   */
  private follow(): void {
    const walks = this.jobWalks;
    if (this.sessions === null) return;
    const standing = standingWalks(this.sessions);
    const items = new Map(
      standing.map((one) => [
        one.key,
        one,
      ]),
    );
    for (const [key, at] of this.approved)
      if (items.get(key)?.since !== at) this.approved.delete(key);
    for (const one of [
      ...walks.queued,
    ]) {
      const item = items.get(one.key);
      if (item !== undefined) {
        one.seen = true;
        const fresh =
          one.kept === true && item.since !== one.since ? this.standingWalk(item) : null;
        if (fresh !== null) walks.replace(fresh);
      } else if (!one.seen) continue;
      else if (sessionReadiness(this.record(one.sessionId ?? ""), one.url) === "working")
        one.kept = true;
      else walks.remove(one.key);
    }
    for (const one of standing) {
      if (walks.find(one.key) !== undefined || this.approved.has(one.key)) continue;
      const walk = this.standingWalk(one);
      if (walk !== null) walks.add(walk);
    }
  }

  /** A Job's walk as its waiting item and its record say it, seen. `null`: its address is not a web one. */
  private standingWalk(one: StandingWalk): JobWalk | null {
    const record = this.record(one.sessionId);
    const origin = webOrigin(one.url);
    if (origin === null) return null;
    const walk = this.sessionWalk(
      one.sessionId,
      titleOfWindow(record, one.url) ?? one.title,
      one.url,
      origin,
      passThrough(originsOfWindow(record, one.url)),
      takesOfWindow(record, one.url),
      [],
      stepsOfWindow(record, one.url),
      one.since,
      briefOfWindow(record, one.url),
    );
    return {
      ...walk,
      seen: true,
    };
  }

  /** A pull request's walk into the walk window, opening it behind on that walk where none is up. */
  private join(walk: Queued): CaptureOpened {
    if (this.walks === null || !this.walks.open) {
      const walks = new WalkWindow(sizes(), this.walkBoard(), null);
      this.walks = walks;
      walks.add(walk, true);
      walks.reveal(false);
      return {
        ok: true,
      };
    }
    this.walks.add(walk, false);
    return {
      ok: true,
    };
  }

  /** When Fleet says a Job's walk at `url` was shown, or now where it says nothing yet. */
  private sinceOf(sessionId: string, url: string): string {
    const record = this.record(sessionId);
    const item =
      record === undefined
        ? undefined
        : standingWalks([
            record,
          ]).find((one) => one.url === url);
    return item?.since ?? new Date().toISOString();
  }

  private record(sessionId: string): SessionRecord | undefined {
    return this.sessions?.find((one) => one.id === sessionId);
  }

  /**
   * A Job's walk, its page made the first time its pane reports a frame. **The sign-in is held until then and let go of once the
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
    brief: string | undefined,
  ): JobWalk {
    const key = sessionWalkKey(sessionId, url);
    let held = signIn;
    return {
      key,
      since,
      sessionId,
      url,
      make: (seat) => {
        const record = this.record(sessionId);
        const pin = {
          run: key,
          name: title,
          origin,
          url,
          manifestId: record?.manifest_id ?? null,
          through,
        };
        // The Job by its own name, as the Jobs list calls it: `title` is the window's.
        const landsOn = {
          session: {
            id: sessionId,
            title: jobName(record, sessionId),
          },
        };
        const walk = new CaptureWindow(
          pin,
          landsOn,
          this.board,
          sizes(),
          null,
          takes,
          held,
          steps,
          seat,
          brief,
        );
        held = [];
        walk.setReadiness(sessionReadiness(record, url));
        return walk;
      },
      walk: null,
      seen: false,
      left: () => this.board.sessionClosed(sessionId, url),
      shape: JSON.stringify({
        takes,
        steps,
        through,
        brief,
      }),
    };
  }

  /** What the walk window tells. */
  private walkBoard(): WalkWindowBoard {
    return {
      // Taking focus dims Bridge behind and moves it to no Job: the owner, 19 Oct 2026, asked when it should, said Never.
      focused: (on) => this.board.focused(WALKS, on),
    };
  }

  /** The Job an IPC call's panel holds: the walk window's walk on screen. `null` where the call came from anything but its panel, or the walk has no Job. */
  panelFrom(contents: Electron.WebContents): PanelSubject | null {
    if (this.walks?.open === true && this.walks.isPanel(contents)) {
      const sessionId = this.walks.onScreen?.sessionId;
      return sessionId === undefined
        ? null
        : {
            kind: "session",
            id: sessionId,
          };
    }
    return null;
  }

  /** A Job's held notes changed: the bar of each walk of that Job is told again, where its notes are held on that Job. */
  heldChanged(sessionId: string): void {
    for (const walk of this.jobWalks.made) walk.heldChanged(sessionId);
  }

  /** Every panel's document, to be told the state Bridge's own windows are told. */
  panels(): Electron.WebContents[] {
    const panel = this.walks?.open === true ? this.walks.panelContents : null;
    return panel === null
      ? []
      : [
          panel,
        ];
  }

  /** The window an IPC call came from, or `null` where it came from anything else. A Job's walk by its own bar; in the walk window, the walk on screen. */
  from(contents: Electron.WebContents): CaptureWindow | null {
    const job = this.jobWalks.from(contents);
    if (job !== null) return job;
    const walk = this.walks?.open === true ? this.walks.fromBar(contents) : null;
    if (walk !== null) return walk;
    for (const window of this.open.values()) {
      if (window.open && window.owns(contents)) return window;
    }
    return null;
  }

  /** A Job's walk placed in a pane of Bridge's window, a frame at a time: the page and the bar are laid over it. */
  place(host: BaseWindow, sessionId: string, walkId: string, bounds: unknown): void {
    this.jobWalks.place(host, sessionId, walkId, bounds);
  }

  /** The pane left: the walk is parked hidden and keeps its page. */
  unplace(sessionId: string, walkId: string): void {
    this.jobWalks.unplace(sessionId, walkId);
  }

  freeze(sessionId: string, walkId: string) {
    return this.jobWalks.freeze(sessionId, walkId);
  }

  thaw(sessionId: string, walkId: string): void {
    this.jobWalks.thaw(sessionId, walkId);
  }

  closeAll(): void {
    this.jobWalks.closeAll();
    this.walks?.close();
    this.walks = null;
    for (const window of this.open.values()) window.close();
    this.open.clear();
  }
}

/** What the walk window is known by when it takes focus, beside the server Runs' windows. */
const WALKS = "walks";

export { questionOf };
