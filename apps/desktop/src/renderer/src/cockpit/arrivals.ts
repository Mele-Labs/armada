// Which calls just arrived, and the one toast that tells somebody so. Pure.
//
// **Entering the set is the event; being in it is not.** The same rule `waiting.ts` in `@armada/screens`
// applies to the system notification, applied to the calls the cockpit deals: what is held is the keys
// in the last reading, and a reading is a diff against it. A call already standing when Bridge opened is
// not news, which is why the first reading seeds and tells nobody.

import type { Item } from "../Dashboard";

/** What the last reading left: the keys of every call standing. `null` until one has been taken. */
export type Held = ReadonlySet<string> | null;

/**
 * The calls in `calls` that `held` did not have. **Nothing while `held` is `null`**: that reading seeds.
 * A call that leaves and comes back is news again, as it is for the system notification.
 */
export function arrived(held: Held, calls: readonly Item[]): Item[] {
  if (held === null) return [];
  return calls.filter((one) => !held.has(one.key));
}

/** One toast: what it says, the Job state its dot wears (none for a Session), and the call it opens. */
export type Alert = {
  /** The call a press brings to the front: the one that has waited longest of those that arrived. */
  key: string;
  /** Every call it speaks for, so it can go when none of them stands. */
  keys: readonly string[];
  sentence: string;
  /** The Job state stem, hyphenated as the status tokens are; omitted where the call is not a Job's. */
  status?: string;
};

/** Oldest first; a call with no readable date last, and the key as the tiebreak. */
function oldestFirst(a: Item, b: Item): number {
  const left = Date.parse(a.at ?? "");
  const right = Date.parse(b.at ?? "");
  if (Number.isNaN(left) || Number.isNaN(right)) return Number.isNaN(left) === Number.isNaN(right) ? a.key.localeCompare(b.key) : Number.isNaN(left) ? 1 : -1;
  return left === right ? a.key.localeCompare(b.key) : left - right;
}

/**
 * What to say about a batch that arrived together, or `null` for none.
 *
 * **Five at once is one toast.** The reasoning is the system notification's: a dispatch whose Jobs reach
 * their gates together is one event to a person and five to the machine. One call names itself; several
 * say how many, in the cockpit's word for them (a Session's question is a call, and not a job), and a press lands on the first.
 */
export function alertOf(fresh: readonly Item[]): Alert | null {
  if (fresh.length === 0) return null;
  const ordered = [...fresh].sort(oldestFirst);
  const first = ordered[0]!;
  const status = ordered.length === 1 ? first.job?.status.replaceAll("_", "-") : undefined;
  return {
    key: first.key,
    keys: ordered.map((one) => one.key),
    sentence: ordered.length === 1 ? `“${first.title}” needs you.` : `${ordered.length} calls need you.`,
    ...(status === undefined ? {} : { status }),
  };
}
