// What main tells a window when its state moved: the whole state once, then only what differs from the last one it sent.
//
// **Why a diff and not the state.** Electron structured-clones what main sends, and the preload then copies it again
// into the page's world. Both costs scale with the whole state however little moved, and a busy Fleet publishes about
// once a second. Main keeps the last state it sent each window and sends the difference; the window applies it, and what
// did not change keeps the object it was, so nothing keyed on identity has to compare it again.

/** One change to a value the window already holds. */
export type Patch =
  | { set: unknown }
  | { keys: Record<string, Patch>; gone?: string[] }
  | { rows: Row[] };

/** One place in an array whose rows all carry a unique string `id`, in the new order: a row kept as it was, changed, or new. */
export type Row = string | { id: string; patch: Patch } | { add: unknown };

/**
 * `full` replaces what the window holds and says which send it is, `seq`. A `patch` applies only to the state of send
 * `from`, and makes the state of send `to`: a window that holds anything else asks for the whole state again.
 */
export type StateChange<S = unknown> = { full: S; seq: number } | { patch: Patch; from: number; to: number };

const plain = (one: unknown): one is Record<string, unknown> => typeof one === "object" && one !== null && Object.getPrototypeOf(one) === Object.prototype;

/** The ids of an array whose every row is a plain object with a distinct string `id`, else `undefined`. */
function idsOf(list: readonly unknown[]): string[] | undefined {
  const ids: string[] = [];
  const seen = new Set<string>();
  for (const one of list) {
    const id = plain(one) ? one["id"] : undefined;
    if (typeof id !== "string" || seen.has(id)) return undefined;
    seen.add(id);
    ids.push(id);
  }
  return ids;
}

/** What turns `was` into `now`, or `undefined` where they are the same. */
export function diff(was: unknown, now: unknown): Patch | undefined {
  if (Object.is(was, now)) return undefined;
  if (plain(was) && plain(now)) {
    const keys: Record<string, Patch> = {};
    let moved = false;
    for (const key of Object.keys(now)) {
      const patch = key in was ? diff(was[key], now[key]) : { set: now[key] };
      if (patch !== undefined) {
        keys[key] = patch;
        moved = true;
      }
    }
    const gone = Object.keys(was).filter((key) => !(key in now));
    if (!moved && gone.length === 0) return undefined;
    return gone.length === 0 ? { keys } : { keys, gone };
  }
  if (Array.isArray(was) && Array.isArray(now)) {
    const before = idsOf(was);
    const after = idsOf(now);
    if (before !== undefined && after !== undefined) {
      const held = new Map(before.map((id, at) => [id, was[at]]));
      let same = was.length === now.length;
      const rows = now.map((one, at): Row => {
        const id = after[at]!;
        if (!held.has(id)) {
          same = false;
          return { add: one };
        }
        const patch = diff(held.get(id), one);
        if (before[at] !== id || patch !== undefined) same = false;
        return patch === undefined ? id : { id, patch };
      });
      return same ? undefined : { rows };
    }
    if (was.length === now.length && was.every((one, at) => diff(one, now[at]) === undefined)) return undefined;
  }
  return { set: now };
}

/** `held` with `patch` applied. Whatever the patch leaves alone is the same object. Throws where `held` is not what the patch was made against. */
export function apply(held: unknown, patch: Patch): unknown {
  if ("set" in patch) return patch.set;
  if ("keys" in patch) {
    if (!plain(held)) throw new Error("a patch for an object met something else");
    const out = { ...held };
    for (const [key, one] of Object.entries(patch.keys)) out[key] = apply(held[key], one);
    for (const key of patch.gone ?? []) delete out[key];
    return out;
  }
  if (!Array.isArray(held)) throw new Error("a patch for a list met something else");
  const byId = new Map<string, unknown>();
  for (const one of held) if (plain(one) && typeof one["id"] === "string") byId.set(one["id"], one);
  return patch.rows.map((row) => {
    if (typeof row === "string") {
      if (!byId.has(row)) throw new Error("a patch kept a row the window does not hold");
      return byId.get(row);
    }
    if ("add" in row) return row.add;
    if (!byId.has(row.id)) throw new Error("a patch changed a row the window does not hold");
    return apply(byId.get(row.id), row.patch);
  });
}
