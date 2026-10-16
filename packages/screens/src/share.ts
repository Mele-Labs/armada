// A value that crossed the window's IPC arrives as new objects every time, equal or not. `share` hands back
// what the window already holds wherever the two are equal, so a record that did not change is the same
// object it was and everything keyed on identity (a memo, React's bail-out) can tell.

const plain = (one: unknown): one is Record<string, unknown> => typeof one === "object" && one !== null && Object.getPrototypeOf(one) === Object.prototype;
const idOf = (one: unknown): string | undefined => (plain(one) && typeof one["id"] === "string" ? one["id"] : undefined);

/**
 * `now`, with every part that equals the same part of `was` replaced by `was`'s. Plain objects and arrays are
 * walked; an array of things with an `id` is matched by it, so a row that moved down one place is still found.
 * Anything else (a Date, a Blob, a function) is kept as `now`.
 */
export function share<T>(was: unknown, now: T): T {
  if (was === now) return now;
  if (Array.isArray(now)) {
    if (!Array.isArray(was)) return now;
    const byId = new Map<string, unknown>();
    for (const one of was) {
      const id = idOf(one);
      if (id !== undefined) byId.set(id, one);
    }
    let same = was.length === now.length;
    const out = now.map((one: unknown, at) => {
      const id = idOf(one);
      const before = id === undefined ? was[at] : byId.get(id);
      const kept = share(before, one);
      if (kept !== was[at]) same = false;
      return kept;
    });
    return (same ? was : out) as T;
  }
  if (plain(now)) {
    if (!plain(was)) return now;
    const keys = Object.keys(now);
    let same = keys.length === Object.keys(was).length;
    const out: Record<string, unknown> = {};
    for (const key of keys) {
      const kept = share(was[key], now[key]);
      if (kept !== was[key] || !(key in was)) same = false;
      out[key] = kept;
    }
    return (same ? was : out) as T;
  }
  return now;
}
