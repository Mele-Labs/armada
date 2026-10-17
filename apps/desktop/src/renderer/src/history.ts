// Back and forward through places, as a fold. A place is where `App`'s flags
// say a person is; the fold knows nothing of how they got there.

import { useEffect, useRef } from "react";
import { useHistoryKeys } from "@armada/shell";
import type { OpenStudio } from "@armada/studios";

export type Place = {
  surface: string;
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
 * The place replacing the layers over the Cockpit: what is behind the last Cockpit visit stays, the layers after it
 * go, and `place` stands where they were, so Back from a sibling place returns to the Cockpit and not to the last one.
 * The place being left counts as a visit, so a press made on the Cockpit itself keeps it behind. With no Cockpit
 * anywhere in the trail it is an ordinary visit.
 */
export function collapse(history: History, place: Place): History {
  const cockpit = (one: Place) => one.surface === "overview";
  const trail = [...history.back, history.at];
  let at = trail.length - 1;
  while (at >= 0 && !cockpit(trail[at]!)) at -= 1;
  if (at < 0) return fold(history, { kind: "visited", place });
  return { back: trail.slice(0, cockpit(place) ? at : at + 1), at: place, forward: [] };
}

/**
 * Record `current` as it changes — by observation, so every way of getting
 * somewhere is a visit — and bind `⌘[`, `⌘]`, the mouse buttons and the OS swipe to `restore`. It hands back
 * `back`, and `replaceNext`, which makes the next place visited replace the layers over the Cockpit (`collapse`)
 * instead of stacking on them.
 */
export function useHistory(current: Place, restore: (place: Place) => void): { back: () => void; replaceNext: () => void } {
  const held = useRef<History>(start(current));
  const latest = useRef({ restore });
  latest.current = { restore };

  // Restoring sets `held.at` first, so the render it causes finds `current` equal to it.
  const replacing = useRef(false);
  useEffect(() => {
    if (replacing.current) {
      replacing.current = false;
      held.current = collapse(held.current, current);
      return;
    }
    held.current = fold(held.current, { kind: "visited", place: current });
  }, [current.surface, current.studio, current.studioNode]); // eslint-disable-line react-hooks/exhaustive-deps

  function go(kind: "back" | "forward"): void {
    // With nowhere to go the current place is restored all the same: it lets go of a layer over it, as the reports are.
    const next = fold(held.current, { kind });
    held.current = next;
    latest.current.restore(next.at);
  }
  useHistoryKeys(() => go("back"), () => go("forward"));
  // A swipe or browser key only main hears.
  useEffect(() => window.armada.onHistory((step) => go(step)), []); // eslint-disable-line react-hooks/exhaustive-deps
  return {
    back: () => go("back"),
    // A press that goes nowhere lets go of it, so the next visit that is not this press's stacks as it always did.
    replaceNext: () => {
      replacing.current = true;
      window.setTimeout(() => (replacing.current = false), 200);
    },
  };
}
