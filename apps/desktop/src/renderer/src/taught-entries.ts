// Which mods a Logbook entry made, as `save_teaching` answered when it was kept: its `entry` and the `mods` it wrote. **Fleet's mod list
// does not say which entry a mod came from** (`ModSummary` carries `taught_on` and no entry), so the window keeps what the answer said,
// in `localStorage` across restarts. A mod Fleet no longer lists is no mod of the entry, so forgetting the entry, which has Fleet remove
// its mods, empties the line without this being told.

import { useSyncExternalStore } from "react";

const KEY = "armada.bridge.taught-entries";

type Kept = Readonly<Record<string, readonly string[]>>;

function read(): Kept {
  try {
    const stored: unknown = JSON.parse(window.localStorage.getItem(KEY) ?? "{}");
    if (typeof stored !== "object" || stored === null || Array.isArray(stored)) return {};
    return Object.fromEntries(Object.entries(stored).flatMap(([id, mods]) => (Array.isArray(mods) ? [[id, mods.filter((one): one is string => typeof one === "string")]] : [])));
  } catch {
    return {};
  }
}

let held: Kept | undefined;
const listeners = new Set<() => void>();
const now = (): Kept => (held ??= read());

function keep(next: Kept): void {
  held = next;
  try {
    window.localStorage.setItem(KEY, JSON.stringify(next));
  } catch {
    // Unwritten, the answer is held for this window, which is all a preference can promise.
  }
  listeners.forEach((on) => on());
}

/** A teaching was kept: the entry it made, if it made one, and the mods it wrote. */
export function rememberTaught(entry: string | undefined, mods: readonly string[] | undefined): void {
  if (entry === undefined || mods === undefined || mods.length === 0) return;
  keep({ ...now(), [entry]: [...new Set([...(now()[entry] ?? []), ...mods])] });
}

/** Forgets every entry, for a window that starts from nothing taught. */
export function forgetTaughtEntries(): void {
  keep({});
}

/** The mods an entry made, by name. */
export function useTaughtMods(entry: string): readonly string[] {
  return useSyncExternalStore(
    (on) => (listeners.add(on), () => void listeners.delete(on)),
    () => now()[entry] ?? NONE,
  );
}

const NONE: readonly string[] = [];
