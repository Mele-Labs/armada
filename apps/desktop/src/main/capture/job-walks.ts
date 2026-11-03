// The walks Jobs showed, each drawn as a tab of the Job in Bridge's own window — `docs/practices/capture-window.md`. The owner approved the mock walk `walkInJob`.
//
// **No window of its own, and nothing raised or focused.** A walk's page and its bar are views main lays inside the frame its pane reports (`CaptureWindow`
// in a pane's seat), the bar above the page. The frame is the pane's to move: with none reported, both are parked hidden and keep their page, step
// and take, so jumping to another Job and back finds the walk as it was. Keyed by the Job and the address it showed, as `queue.ts` names it.

import type { BaseWindow } from "electron";

import { CHANNELS } from "../../shared/bridge";
import type { JobWalkFrozen } from "../../shared/api/job-walk";
import { boundsOf } from "../session-page-bounds";
import { sessionWalkKey } from "./queue";
import type { Rect } from "./layout";
import type { Queued } from "./walk-window";

/** A Job's walk: the walk window's entry with its Job, and without what only a queue's bar or a held pull request's walk has. */
export type JobWalk = Omit<Queued, "describe" | "held" | "sessionId"> & {
  sessionId: string;
};
import { barView } from "./views";
import type { CaptureWindow, WalkSeat } from "./window";

/** What the walks tell whoever holds them. */
export type JobWalksBoard = {
  /** A walk was made, or put in front of a pane: it is told what it should be showing, a question among it. */
  shown: (walk: JobWalk) => void;
  /** The owner approved a walk, as it leaves. */
  approved: (walk: JobWalk) => void;
};

export class JobWalks {
  private walks: JobWalk[] = [];
  /** Where each walk's pane has it, by key. A frame outlives the walk it was reported for, which a Job showing the page again makes afresh. */
  private readonly frames = new Map<string, Rect>();
  private readonly host: () => BaseWindow | null;
  private readonly forward: (input: Electron.Input) => boolean;
  private readonly board: JobWalksBoard;

  /**
   * `host`: Bridge's own window, or `null` while none is up. `forward`: a press a view of a walk heard that is Bridge's own window's, handed to it;
   * `true` where it was, so the view does not hear it too.
   */
  constructor(
    host: () => BaseWindow | null,
    forward: (input: Electron.Input) => boolean,
    board: JobWalksBoard,
  ) {
    this.host = host;
    this.forward = forward;
    this.board = board;
  }

  /** The walks, in the order they were shown. */
  get queued(): readonly JobWalk[] {
    return this.walks;
  }

  find(key: string): JobWalk | undefined {
    return this.walks.find((one) => one.key === key);
  }

  /** A walk the Job showed, or one Fleet still holds open. **Its page is made when its pane first reports a frame**, so a walk nobody has opened loads nothing. */
  add(walk: JobWalk): void {
    if (this.find(walk.key) !== undefined) return;
    this.walks = [
      ...this.walks,
      walk,
    ];
    if (this.frames.has(walk.key)) this.make(walk);
  }

  /** A walk shown again with other takes, other steps, other origins or a sign-in to run: it keeps its place and its frame, and its page is made afresh. */
  replace(walk: JobWalk): void {
    const standing = this.find(walk.key);
    if (standing === undefined) return this.add(walk);
    const fresh: JobWalk = {
      ...walk,
      since: standing.since,
      seen: standing.seen || walk.seen,
    };
    this.walks = this.walks.map((one) => (one === standing ? fresh : one));
    standing.walk?.dispose();
    if (this.frames.has(fresh.key)) this.make(fresh);
  }

  /** A walk leaves: approved, or its waiting item gone while it reads ready. **Its page goes**, and its owner is told once. */
  remove(key: string): void {
    const leaving = this.find(key);
    if (leaving === undefined) return;
    this.walks = this.walks.filter((one) => one !== leaving);
    leaving.walk?.dispose();
    leaving.left();
  }

  /** The pane of a Job's walk reports where it is, in the window it was drawn in. The page and the bar follow it, and nothing is raised or focused. */
  place(host: BaseWindow, sessionId: string, walkId: string, bounds: unknown): void {
    if (host.isDestroyed()) return;
    const key = sessionWalkKey(sessionId, walkId);
    const at = boundsOf(bounds, host.getContentBounds());
    const was = this.frames.get(key);
    this.frames.set(key, at);
    const walk = this.find(key);
    if (walk === undefined) return;
    if (walk.walk === null) this.make(walk);
    else if (was === undefined) walk.walk.show();
    else walk.walk.resized();
  }

  /** The pane left: the walk's views are parked hidden. */
  unplace(sessionId: string, walkId: string): void {
    const key = sessionWalkKey(sessionId, walkId);
    if (!this.frames.delete(key)) return;
    this.find(key)?.walk?.hide();
  }

  /** Both views as pictures, held still under a layer of Bridge's own that lies over the frame. */
  async freeze(sessionId: string, walkId: string): Promise<JobWalkFrozen | null> {
    return (await this.find(sessionWalkKey(sessionId, walkId))?.walk?.freeze()) ?? null;
  }

  thaw(sessionId: string, walkId: string): void {
    this.find(sessionWalkKey(sessionId, walkId))?.walk?.thaw();
  }

  /** The walk whose bar an IPC call came from, which is what every bar call is about. */
  from(contents: Electron.WebContents): CaptureWindow | null {
    for (const one of this.walks) if (one.walk?.owns(contents) === true) return one.walk;
    return null;
  }

  /** Every walk made. */
  get made(): CaptureWindow[] {
    return this.walks.flatMap((one) => (one.walk === null ? [] : [one.walk]));
  }

  /** Every walk in it, with its page taken out of the window. */
  closeAll(): void {
    const left = this.walks;
    this.walks = [];
    this.frames.clear();
    for (const one of left) {
      one.walk?.dispose();
      one.left();
    }
  }

  /** The walk's page and bar made, with a seat that is the pane's: its own bar, the frame it reports, no panel and no queue. */
  private make(walk: JobWalk): void {
    const bar = barView(true);
    const seat: WalkSeat = {
      window: null,
      bar,
      panel: null,
      onScreen: () => this.frames.has(walk.key),
      queue: () => ({
        entries: [],
        at: 0,
      }),
      // The keys the page heard that the bar acts on. A queue's own (the walks, the takes) are not a pane's.
      key: (key) => {
        if (key !== "approve" && key !== "reload") return;
        if (!bar.webContents.isDestroyed()) bar.webContents.send(CHANNELS.captureWindowKey, key);
      },
      settled: () => {
        this.board.approved(walk);
        this.remove(walk.key);
      },
      pane: {
        host: this.host,
        frame: () => this.frames.get(walk.key) ?? null,
        forward: this.forward,
      },
    };
    walk.walk = walk.make(seat);
    walk.walk.show();
    this.board.shown(walk);
  }
}
