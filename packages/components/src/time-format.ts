// How Bridge draws a time. One format the person sets once in Settings, read wherever a stamp is
// drawn, and the relative age a press on a stamp flips to. The store is `keymap.ts`'s: a module
// value, a saver Bridge hands in, and a hook that redraws its readers when the value moves.

import { useSyncExternalStore } from "react";

/** The three switches. Each one adds to the stamp, so the sample beside them is always the whole answer. */
export type TimeFormat = { hour12: boolean; seconds: boolean; date: boolean };

/** What ships: the thread's own look, a 24-hour clock to the second with no date. */
export const DEFAULT_TIME_FORMAT: TimeFormat = { hour12: false, seconds: true, date: false };

let current: TimeFormat = DEFAULT_TIME_FORMAT;
let saver: ((text: string) => void) | null = null;
const listeners = new Set<() => void>();

/** What the person has set. */
export function timeFormat(): TimeFormat {
  return current;
}

/** Take Fleet's answer, on a read or after a save. Identical values draw nothing. */
export function setTimeFormat(next: TimeFormat): void {
  if (next.hour12 === current.hour12 && next.seconds === current.seconds && next.date === current.date) return;
  current = next;
  listeners.forEach((on) => on());
}

/** Where a change is sent to be kept. Bridge hands Fleet's save in; a test or the mock keeps none. */
export function setTimeFormatSaver(save: ((text: string) => void) | null): void {
  saver = save;
}

/** A change the person made: drawn now, and sent to be kept. */
export function saveTimeFormat(next: TimeFormat): void {
  setTimeFormat(next);
  saver?.(JSON.stringify(next));
}

/** Fleet's text back into a format. Text that is not one gives the shipped default, never half of another. */
export function parseTimeFormat(text: string): TimeFormat {
  try {
    const raw: unknown = JSON.parse(text);
    if (typeof raw !== "object" || raw === null) return DEFAULT_TIME_FORMAT;
    const { hour12, seconds, date } = raw as Record<string, unknown>;
    if (typeof hour12 !== "boolean" || typeof seconds !== "boolean" || typeof date !== "boolean") return DEFAULT_TIME_FORMAT;
    return { hour12, seconds, date };
  } catch {
    return DEFAULT_TIME_FORMAT;
  }
}

/** Re-renders a component when the format moves, so every stamp it draws is in the one now set. */
export function useTimeFormat(): TimeFormat {
  return useSyncExternalStore(
    (on) => {
      listeners.add(on);
      return () => void listeners.delete(on);
    },
    () => current,
    () => current,
  );
}

// One formatter per combination, built once: `toLocaleTimeString` builds a fresh Intl formatter on each
// call, which was most of the time a Sessions message took to read (see `duration.ts`'s CLOCK).
const TIMES = new Map<string, Intl.DateTimeFormat>();
const DATE = new Intl.DateTimeFormat([], { month: "short", day: "numeric" });

function timeOf(format: TimeFormat): Intl.DateTimeFormat {
  const key = `${format.hour12}:${format.seconds}`;
  let one = TIMES.get(key);
  if (one === undefined) {
    one = new Intl.DateTimeFormat([], {
      hour: format.hour12 ? "numeric" : "2-digit",
      minute: "2-digit",
      ...(format.seconds ? { second: "2-digit" } : {}),
      hour12: format.hour12,
    });
    TIMES.set(key, one);
  }
  return one;
}

/** The instant as the person set it to be read: "14:02:09", or "Oct 19 2:02 PM". The host's locale spells the clock. */
export function stampOf(ms: number, format: TimeFormat): string {
  const time = timeOf(format).format(ms);
  return format.date ? `${DATE.format(ms)} ${time}` : time;
}

/** How long ago, to one unit: "<1m ago", "4m ago", "2h ago", "3d ago". An instant ahead of `now` reads as the first. */
export function ageOf(ms: number, now: number): string {
  const minutes = Math.floor(Math.max(0, now - ms) / 60_000);
  if (minutes < 1) return "<1m ago";
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}
