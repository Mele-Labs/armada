// The walk window as `CaptureWindows` drives it, with no Electron: the queue, the walk on screen, and a page made the
// first time a walk comes on screen. For the tests of `windows.ts`, which mock `./walk-window` with this.

import type { Queued, WalkWindowBoard } from "./walk-window";
import { placeOf } from "./queue";
import type { WalkSeat } from "./window";

/** Every fake window made, the last one last. */
export const made: { windows: FakeWalkWindow[] } = { windows: [] };

export class FakeWalkWindow {
  open = true;
  raised = 0;
  revealed: boolean[] = [];
  private walks: Queued[] = [];
  private current: Queued | null = null;
  private readonly board: WalkWindowBoard;
  private readonly seat = { onScreen: () => true } as unknown as WalkSeat;

  constructor(_sizes: unknown, board: WalkWindowBoard) {
    this.board = board;
    made.windows.push(this);
  }

  get queued(): readonly Queued[] {
    return this.walks;
  }

  get onScreen(): Queued | null {
    return this.current;
  }

  find(key: string): Queued | undefined {
    return this.walks.find((one) => one.key === key);
  }

  add(walk: Queued, show: boolean): void {
    if (this.find(walk.key) !== undefined) return;
    const at = placeOf(this.walks, walk.since);
    this.walks = [...this.walks.slice(0, at), walk, ...this.walks.slice(at)];
    if (show || this.current === null) this.bring(walk);
  }

  replace(walk: Queued): void {
    const standing = this.find(walk.key);
    if (standing === undefined) return this.add(walk, false);
    const fresh = { ...walk, since: standing.since };
    this.walks = this.walks.map((one) => (one === standing ? fresh : one));
    standing.walk?.dispose();
    if (this.current === standing) this.bring(fresh);
  }

  remove(key: string): void {
    const leaving = this.find(key);
    if (leaving === undefined) return;
    this.walks = this.walks.filter((one) => one !== leaving);
    leaving.walk?.dispose();
    leaving.left();
    if (this.current === leaving) this.current = null;
    if (this.walks.length === 0) this.close();
    else if (this.current === null) this.bring(this.walks[0]!);
  }

  jump(at: number): void {
    const walk = this.walks[at];
    if (walk !== undefined && walk !== this.current) this.bring(walk);
  }

  reveal(focus: boolean): void {
    this.revealed.push(focus);
  }

  raise(): void {
    this.raised += 1;
  }

  close(): void {
    if (!this.open) return;
    this.open = false;
    const left = this.walks;
    this.walks = [];
    this.current = null;
    for (const one of left) {
      one.walk?.dispose();
      one.left();
    }
    this.board.closed();
  }

  refresh(): void {}

  fromBar(): null {
    return null;
  }

  isPanel(): boolean {
    return false;
  }

  get panelContents(): null {
    return null;
  }

  private bring(walk: Queued): void {
    this.current = walk;
    if (walk.walk === null) walk.walk = walk.make(this.seat);
    this.board.shown(walk);
  }
}
