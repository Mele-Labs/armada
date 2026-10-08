import { useRef, useState, useSyncExternalStore } from "react";

/**
 * What a person typed and has not sent, kept by what it is about. The map is
 * the copy the window holds across leaving a panel and coming back; the
 * browser's storage is the copy a reload finds. Every storage access is in a
 * try, so a blocked store leaves the map and nothing else.
 */
const PREFIX = "armada:kept:";
const held = new Map<string, unknown>();
const listeners = new Set<() => void>();

const emit = () => listeners.forEach((one) => one());

/** What was kept under `key`. `revive` turns the stored form back into the value, where the two differ. */
export function readKept<T>(key: string, revive?: (stored: unknown) => T): T | undefined {
  if (held.has(key)) return held.get(key) as T;
  try {
    const raw = window.localStorage.getItem(PREFIX + key);
    if (raw === null) return undefined;
    const value = revive === undefined ? (JSON.parse(raw) as T) : revive(JSON.parse(raw));
    held.set(key, value);
    return value;
  } catch {
    return undefined;
  }
}

/** Keeps `value`; `stored` is the part of it storage can hold (a File cannot be). */
export function writeKept<T>(key: string, value: T, stored: unknown = value): void {
  held.set(key, value);
  try {
    window.localStorage.setItem(PREFIX + key, JSON.stringify(stored));
  } catch {
    // The in-memory copy is the one that counts.
  }
  emit();
}

/** The message was sent, or the thing it was about is gone. */
export function forgetKept(key: string): void {
  held.delete(key);
  try {
    window.localStorage.removeItem(PREFIX + key);
  } catch {
    // Nothing was stored.
  }
  emit();
}

/** Everything kept, gone: a story or a test starts from a window that kept nothing. */
export function forgetAllKept(): void {
  held.clear();
  try {
    Object.keys(window.localStorage)
      .filter((one) => one.startsWith(PREFIX))
      .forEach((one) => window.localStorage.removeItem(one));
  } catch {
    // Nothing was stored.
  }
  emit();
}

const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => void listeners.delete(listener);
};

/**
 * A value that outlives the panel holding it: `[value, set, clear]`. With no
 * `key` it is plain component state, so a story or a surface with nothing to
 * be about keeps nothing.
 */
export function useKept<T>(
  key: string | undefined,
  initial: T,
  options: { stored?: (value: T) => unknown; revive?: (stored: unknown) => T } = {},
): [T, (next: T | ((was: T) => T)) => void, () => void] {
  const first = useRef(initial);
  const [local, setLocal] = useState(initial);
  const kept = useSyncExternalStore(subscribe, () => (key === undefined ? first.current : (readKept<T>(key, options.revive) ?? first.current)));
  const value = key === undefined ? local : kept;
  const set = (next: T | ((was: T) => T)) => {
    const was = key === undefined ? value : (readKept<T>(key, options.revive) ?? first.current);
    const resolved = typeof next === "function" ? (next as (was: T) => T)(was) : next;
    if (key === undefined) setLocal(resolved);
    else writeKept(key, resolved, options.stored === undefined ? resolved : options.stored(resolved));
  };
  const clear = () => (key === undefined ? setLocal(first.current) : forgetKept(key));
  return [value, set, clear];
}
