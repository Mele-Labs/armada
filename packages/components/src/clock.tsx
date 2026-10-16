// The window's one clock. A figure that reads the time subscribes where it draws, so a tick draws
// those leaves and not the screen above them. One interval serves every reader, and it runs only
// while somebody is reading.

import { useSyncExternalStore } from "react";
import type { ReactNode } from "react";

/** How often the elapsed figures are redrawn. They are read, so they must move. */
const TICK_MS = 1000;

const listeners = new Set<() => void>();
let current = Date.now();
let timer: ReturnType<typeof setInterval> | undefined;

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  timer ??= setInterval(() => {
    current = Date.now();
    for (const one of [...listeners]) one();
  }, TICK_MS);
  return () => {
    listeners.delete(listener);
    if (listeners.size === 0 && timer !== undefined) {
      clearInterval(timer);
      timer = undefined;
    }
  };
}

/** While nobody listens `current` is not moving, so a reader that mounts after a pause is not handed the time it last saw. */
function read(): number {
  if (timer === undefined && Date.now() - current >= TICK_MS) current = Date.now();
  return current;
}

/**
 * The time, redrawn at each `step` and not between: a figure that reads whole minutes passes a
 * minute and is drawn once in it, where one that reads seconds takes the default.
 */
export function useNow(step: number = TICK_MS): number {
  return useSyncExternalStore(subscribe, () => Math.floor(read() / step) * step);
}

/** The time while `active`, and a fixed 0 otherwise: a reader that only needs it in one state draws on no tick outside it. */
export function useNowWhile(active: boolean): number {
  return useSyncExternalStore(
    active ? subscribe : QUIET,
    () => (active ? read() : 0),
  );
}

const QUIET = (): (() => void) => () => undefined;

/** Draws `children` on the clock, so a screen that takes `now` as a prop can be given it without its parent reading it. */
export function Ticking({ children, step }: { children: (now: number) => ReactNode; step?: number }) {
  return <>{children(useNow(step))}</>;
}
