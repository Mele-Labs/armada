// When a Job's Field asks Fleet for a file's change: as a drone starts writing it, again every two seconds while one still is, and on a
// `session.edited` entry naming it, never more than once in two seconds for one file, and always with the stamp already held so an
// unchanged file is answered `unchanged`. Nothing is asked while nothing is wanted. A refused file is asked again only after an edit
// names it; a Fleet without the route is never asked again.

import type { FileChangeRead } from "@armada/screens/src/draft/sessions";

/** The least time between two asks for one file. */
export const ASK_MS = 2_000;

export type Ask = (path: string, seen: string | undefined) => Promise<FileChangeRead>;

/** What an ask came to: a new change, or a refusal. An `unchanged` answer reports nothing. */
export type Answered = (path: string, read: Extract<FileChangeRead, { ok: true }> | undefined) => void;

type Timers = { set: (run: () => void, ms: number) => number; clear: (id: number) => void; now: () => number };

const WINDOW: Timers = { set: (run, ms) => window.setTimeout(run, ms), clear: (id) => window.clearTimeout(id), now: () => Date.now() };

export class LensAsks {
  private readonly wanted = new Set<string>();
  private readonly last = new Map<string, number>();
  private readonly timers = new Map<string, number>();
  private readonly flying = new Set<string>();
  private readonly refused = new Set<string>();
  private gone = false;
  private noRoute = false;

  /** `seen` is the stamp held for a file, read at each ask. */
  constructor(
    private readonly ask: Ask,
    private readonly seen: (path: string) => string | undefined,
    private readonly answered: Answered,
    private readonly clock: Timers = WINDOW,
  ) {}

  /** The files being written now. A new one is asked at once, as its two seconds allow; one no longer written is asked no more. */
  want(paths: Iterable<string>): void {
    const now = new Set(paths);
    for (const path of [...this.wanted]) {
      if (now.has(path)) continue;
      this.wanted.delete(path);
      this.unschedule(path);
    }
    for (const path of now) {
      if (this.wanted.has(path)) continue;
      this.wanted.add(path);
      this.poke(path);
    }
  }

  /** Fleet recorded an edit to `path`: asked now if it is wanted and its two seconds have passed, else as soon as they have. */
  edited(path: string): void {
    if (!this.wanted.has(path)) return;
    this.refused.delete(path);
    this.poke(path);
  }

  /** Asks nothing more, and forgets what it waited on. */
  stop(): void {
    this.want([]);
  }

  dispose(): void {
    this.gone = true;
    this.stop();
  }

  private poke(path: string): void {
    if (this.gone || this.noRoute || this.refused.has(path) || this.flying.has(path) || this.timers.has(path)) return;
    const wait = (this.last.get(path) ?? -Infinity) + ASK_MS - this.clock.now();
    if (wait <= 0) this.fire(path);
    else this.schedule(path, wait);
  }

  private schedule(path: string, ms: number): void {
    this.timers.set(
      path,
      this.clock.set(() => {
        this.timers.delete(path);
        if (this.wanted.has(path)) this.poke(path);
      }, ms),
    );
  }

  private unschedule(path: string): void {
    const timer = this.timers.get(path);
    if (timer === undefined) return;
    this.clock.clear(timer);
    this.timers.delete(path);
  }

  private fire(path: string): void {
    this.last.set(path, this.clock.now());
    this.flying.add(path);
    const settle = (read: FileChangeRead | undefined): void => {
      this.flying.delete(path);
      if (this.gone || !this.wanted.has(path)) return;
      if (read === undefined || !read.ok) {
        if (read?.ok === false && read.noRoute === true) this.noRoute = true;
        this.refused.add(path);
        this.answered(path, undefined);
        return;
      }
      if (read.unchanged !== true) this.answered(path, read);
      // Still written: asked again two seconds after this ask went.
      this.poke(path);
    };
    this.ask(path, this.seen(path)).then(settle, () => settle(undefined));
  }
}
