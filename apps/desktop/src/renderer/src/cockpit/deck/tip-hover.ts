// What the map says about the thing under the pointer, kept steady. Where drones, pads, tiles and builds overlap in the tilted scene, one
// pointer position is on and off several of them within a frame, and a card that follows each flip remounts at once, many times a second. A
// card is given to another thing only once that has held the pointer for `SWITCH_MS`. Once the pointer has left the card's own thing, the
// card waits the inspect card's grace before it closes or is handed on, so the pointer can cross the gap to the card; on the card it holds,
// as the inspect card does.

import type { Anchor } from "./Callout";
import type { Tip } from "./tip";

export const SWITCH_MS = 90;
/** How long a card stands once the pointer has left its thing, where no shared grace is given. */
export const CROSS_MS = 300;

/** What is shown: the card, what it hangs from, and the drone, tile or part (a pad, flag or crate) it lights while it stands. */
export type Shown = { tip: Tip; anchor: Anchor; hot?: string; tile?: string; part?: string };

export type Tips = {
  /** The pointer or focus came to something. */
  show: (shown: Shown) => void;
  /** It left what it was on. The card stays a moment, in case it is only crossing to the next thing or to the card. */
  leave: () => void;
  /** The pointer is on the card: it stays until the pointer leaves the card. */
  hold: () => void;
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
  // The pointer has left the card's own thing and may be on its way to the card.
  let crossing = false;
  const cross = () => waits?.grace() ?? CROSS_MS;
  let settled: (shown: Shown | undefined) => void = () => undefined;
  const listeners = new Set<() => void>();
  const cancel = () => {
    if (pending !== undefined) timers.clear(pending);
    pending = undefined;
  };
  const settle = (next: Shown | undefined) => {
    cancel();
    crossing = false;
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
      if (current !== undefined) return current.anchor.target === shown.anchor.target ? settle(shown) : queue(shown, crossing ? cross() : SWITCH_MS);
      // The first card of a run waits the shared delay; one that follows a card just closed does not.
      if (waits === undefined || Date.now() - closedAt < waits.grace()) return settle(shown);
      queue(shown, waits.delay());
    },
    leave: () => {
      if (current === undefined) return cancel();
      crossing = true;
      queue(undefined, cross());
    },
    hold: () => {
      cancel();
      crossing = false;
    },
    clear: () => settle(undefined),
    get: () => current,
    subscribe: (on) => (listeners.add(on), () => void listeners.delete(on)),
    cancel,
    onSettle: (on) => {
      settled = on;
    },
  };
}
