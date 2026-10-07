// Back and forward through places, as a fold. A place is where `App`'s flags
// say a person is; the fold knows nothing of how they got there.

import { useEffect, useRef } from "react";
import { useHistoryKeys } from "@armada/shell";
import type { OpenStudio } from "@armada/studios";

export type Place = {
  surface: string;
  job: string | null;
  session: string | null;
  studio: OpenStudio | null;
  studioNode: string | null;
};

export type History = { back: Place[]; at: Place; forward: Place[] };

export type Event = { kind: "visited"; place: Place } | { kind: "back" } | { kind: "forward" };

export const CAP = 50;

export function start(at: Place): History {
  return { back: [], at, forward: [] };
}

export function same(a: Place, b: Place): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

export function fold(history: History, event: Event): History {
  switch (event.kind) {
    case "visited":
      if (same(event.place, history.at)) return history;
      return { back: [...history.back, history.at].slice(-CAP), at: event.place, forward: [] };
    case "back": {
      const to = history.back[history.back.length - 1];
      if (to === undefined) return history;
      return { back: history.back.slice(0, -1), at: to, forward: [history.at, ...history.forward] };
    }
    case "forward": {
      const [to, ...rest] = history.forward;
      if (to === undefined) return history;
      return { back: [...history.back, history.at].slice(-CAP), at: to, forward: rest };
    }
  }
}

/**
 * Record `current` as it changes — by observation, so every way of getting
 * somewhere is a visit — and bind `⌘[` / `⌘]` to `restore`. A place `valid`
 * refuses (its Job is gone) is stepped over.
 */
export function useHistory(current: Place, restore: (place: Place) => void, valid: (place: Place) => boolean): void {
  const held = useRef<History>(start(current));
  const latest = useRef({ restore, valid });
  latest.current = { restore, valid };

  // Restoring sets `held.at` first, so the render it causes finds `current` equal to it.
  useEffect(() => {
    held.current = fold(held.current, { kind: "visited", place: current });
  }, [current.surface, current.job, current.session, current.studio, current.studioNode]); // eslint-disable-line react-hooks/exhaustive-deps

  function go(kind: "back" | "forward"): void {
    let next = fold(held.current, { kind });
    while (next !== held.current && !latest.current.valid(next.at)) {
      held.current = next;
      next = fold(held.current, { kind });
    }
    if (next === held.current && !latest.current.valid(next.at)) return;
    held.current = next;
    latest.current.restore(next.at);
  }
  useHistoryKeys(() => go("back"), () => go("forward"));
}
