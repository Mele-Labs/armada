// Mock only: the construction on the battlefield with no Fleet behind it. A scenario hands a script, the views of each moment, and a
// walk's `later` step takes the next moment, so a twelve-hour Session is played in a few presses. `?build=tower|pad|ring` is how
// literally it is drawn and `?after=stand|clear|monument` what a landed structure does a day on; the window's bar swaps them.

import type { After, ProgressFeed, Take } from "../cockpit/deck/progress";
import type { ProgressSource } from "../cockpit/deck/progress-source";

/** The views of one moment of the story. */
export type ProgressScript = (moment: number) => ProgressFeed["views"];

const TAKES: readonly Take[] = ["tower", "pad", "ring"];
const AFTERS: readonly After[] = ["stand", "clear", "monument"];
const named = <T extends string>(all: readonly T[], value: string | null, fallback: T): T => all.find((one) => one === value) ?? fallback;

export function createMockProgress(): ProgressSource & { reset(script?: ProgressScript): void; next(): void } {
  let script: ProgressScript | undefined;
  let moment = 0;
  let feed: ProgressFeed | undefined;
  const listeners = new Set<() => void>();
  const read = () => {
    const query = new URLSearchParams(window.location.search);
    feed = script === undefined ? undefined : { take: named(TAKES, query.get("build"), "tower"), after: named(AFTERS, query.get("after"), "clear"), views: script(moment) };
    listeners.forEach((on) => on());
  };
  return {
    get: () => feed,
    subscribe: (on) => (listeners.add(on), () => void listeners.delete(on)),
    reset: (to) => {
      script = to;
      moment = 0;
      read();
    },
    next: () => {
      moment += 1;
      read();
    },
  };
}

export const mockProgress = createMockProgress();
