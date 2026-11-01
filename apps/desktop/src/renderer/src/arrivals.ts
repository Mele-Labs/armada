// Which Jobs arrived while this window was open, so a Job just dispatched lifts off into the Cockpit's list and onto the stage once,
// and a Job that was already there when the window opened never does. Read on render: noting is idempotent, so a second
// render (StrictMode) notes nothing new.

const seen = new Set<string>();
const fresh = new Set<string>();
let primed = false;
const lifted = new Set<string>();

/** The list's rows by key, as drawn now. The first list drawn is what was already there. */
export function noteArrivals(keys: readonly string[]): void {
  if (!primed) {
    if (keys.length === 0) return;
    keys.forEach((key) => seen.add(key));
    primed = true;
    return;
  }
  for (const key of keys) {
    if (seen.has(key)) continue;
    seen.add(key);
    fresh.add(key);
  }
}

/** The row came in after the window opened. */
export const arrivedHere = (key: string): boolean => fresh.has(key);

/** The Job's card lifts off the first time it is opened after it arrived, and only then. */
export function liftOnce(key: string): boolean {
  if (!fresh.has(key) || lifted.has(key)) return false;
  lifted.add(key);
  return true;
}
