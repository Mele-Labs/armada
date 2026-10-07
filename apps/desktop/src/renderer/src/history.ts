// Back and forward through places, as a fold. A place is where `App`'s flags
// say a person is; the fold knows nothing of how they got there.

import { useEffect, useRef, useState } from "react";
import type { DetailTab } from "@armada/jobs";
import { useHistoryKeys } from "@armada/shell";
import type { OpenStudio } from "@armada/studios";

export type Place = {
  surface: string;
  job: string | null;
  /** The Job's open tab; null when no Job is open, or before its first report. */
  tab: DetailTab | null;
  /** What is open inside that tab — a task, step, Drone or Record row, by id — or null. */
  item: string | null;
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
      // A restore renders once before the tab reports back; that blank is not a place.
      if (event.place.job !== null && event.place.tab === null && history.at.job === event.place.job && history.at.tab !== null && same({ ...history.at, tab: null, item: null }, event.place)) return history;
      // A Job's first tab, and what is open in it, are reported a render after the Job opens: the same visit, completed.
      if (history.at.job !== null && history.at.tab === null && event.place.job === history.at.job && same({ ...event.place, tab: null, item: null }, history.at)) {
        return { ...history, at: event.place };
      }
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

/** The tab and item the open Job reports, read as null for any other Job and once none is open. */
export function useJobTab(openJob: string | null): [{ tab: DetailTab | null; item: string | null }, (job: string, tab: DetailTab, item: string | null) => void] {
  const [held, set] = useState<{ job: string; tab: DetailTab; item: string | null } | null>(null);
  useEffect(() => {
    if (openJob === null) set(null);
  }, [openJob]);
  return [held !== null && held.job === openJob ? held : { tab: null, item: null }, (job, tab, item) => set({ job, tab, item })];
}

/** The opening that puts a Job on `place`'s tab with its item open — the id's kind is the tab's. */
export function openingOf(place: Place): { tab?: DetailTab; task?: string; step?: string; drone?: string; row?: string } {
  if (place.tab === null) return {};
  const item = place.item === null ? {} : place.tab === "workflow" ? { step: place.item } : place.tab === "drones" ? { drone: place.item } : place.tab === "record" ? { row: place.item } : { task: place.item };
  return { tab: place.tab, ...item };
}

/**
 * Record `current` as it changes — by observation, so every way of getting
 * somewhere is a visit — and bind `⌘[`, `⌘]`, the mouse buttons and the OS swipe to `restore`. A place `valid`
 * refuses (its Job is gone) is stepped over.
 */
export function useHistory(current: Place, restore: (place: Place) => void, valid: (place: Place) => boolean): void {
  const held = useRef<History>(start(current));
  const latest = useRef({ restore, valid });
  latest.current = { restore, valid };

  // Restoring sets `held.at` first, so the render it causes finds `current` equal to it.
  useEffect(() => {
    held.current = fold(held.current, { kind: "visited", place: current });
  }, [current.surface, current.job, current.tab, current.item, current.session, current.studio, current.studioNode]); // eslint-disable-line react-hooks/exhaustive-deps

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
  // A swipe or browser key only main hears.
  useEffect(() => window.armada.onHistory((step) => go(step)), []); // eslint-disable-line react-hooks/exhaustive-deps
}
