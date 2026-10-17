// What the map says about the thing under the pointer, kept steady. Where drones, pads, tiles and builds overlap in the tilted scene, one
// pointer position is on and off several of them within a frame, and a card that follows each flip remounts at once, many times a second. A
// card is kept until the pointer has been off its thing for `LEAVE_MS`, and is given to another thing only once that has held the pointer for
// `SWITCH_MS`.

import type { Anchor } from "./Callout";
import type { Tip } from "./tip";

export const LEAVE_MS = 110;
export const SWITCH_MS = 90;

/** What is shown: the card, what it hangs from, and the drone or tile it lights while it stands. */
export type Shown = { tip: Tip; anchor: Anchor; hot?: string; tile?: string };

export type Tips = {
  /** The pointer or focus came to something. */
  show: (shown: Shown) => void;
  /** It left what it was on. The card stays a moment, in case it is only crossing to the next thing. */
  leave: () => void;
  /** Away at once: Esc, a press, the camera taken by hand. */
  clear: () => void;
  get: () => Shown | undefined;
  subscribe: (on: () => void) => () => void;
  /** Stop a wait still pending, for the map going away. */
  cancel: () => void;
  /** Runs when what is shown settles. */
  onSettle: (on: (shown: Shown | undefined) => void) => void;
};

/** How long a card waits before it first opens, and how long after one closed the next opens at once. The inspect card's, so the map keeps time with every other card. */
export type Waits = { delay: () => number; grace: () => number };

export function steadyTips(
  timers: { set: (run: () => void, ms: number) => number; clear: (id: number) => void } = { set: (run, ms) => window.setTimeout(run, ms), clear: (id) => window.clearTimeout(id) },
  waits?: Waits,
): Tips {
  let current: Shown | undefined;
  let closedAt = 0;
  let pending: number | undefined;
  let settled: (shown: Shown | undefined) => void = () => undefined;
  const listeners = new Set<() => void>();
  const cancel = () => {
    if (pending !== undefined) timers.clear(pending);
    pending = undefined;
  };
  const settle = (next: Shown | undefined) => {
    cancel();
    if (next === current) return;
    if (next === undefined) closedAt = Date.now();
    current = next;
    settled(next);
    listeners.forEach((on) => on());
  };
  const queue = (next: Shown | undefined, ms: number) => {
    cancel();
    pending = timers.set(() => {
      pending = undefined;
      settle(next);
    }, ms);
  };
  return {
    show: (shown) => {
      if (current !== undefined) return current.anchor.target === shown.anchor.target ? settle(shown) : queue(shown, SWITCH_MS);
      // The first card of a run waits the shared delay; one that follows a card just closed does not.
      if (waits === undefined || Date.now() - closedAt < waits.grace()) return settle(shown);
      queue(shown, waits.delay());
    },
    leave: () => (current === undefined ? cancel() : queue(undefined, LEAVE_MS)),
    clear: () => settle(undefined),
    get: () => current,
    subscribe: (on) => (listeners.add(on), () => void listeners.delete(on)),
    cancel,
    onSettle: (on) => {
      settled = on;
    },
  };
}
