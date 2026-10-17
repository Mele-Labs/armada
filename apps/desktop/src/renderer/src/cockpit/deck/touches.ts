// What each Session is changing, read from what Fleet observed and never from a table.
//
// **Four feeds, each real and each partial.** A Session's `session.edited` bursts carry the paths it wrote, newest first
// and at most 25 of them.
// Fleet's `list_slot_touches` is its own git read of
// what each held slot changed since it met the base, and is the last resort for a drone no burst places. Write clashes come
// from Fleet's read of the held slots too. A drone with none of these touches nothing, and the battlefield leaves it at its
// pad. #2301.

import type { SlotTouches, TimelineEvent, WriteClash } from "@armada/protocol";

import { areaOf, relativeTo } from "./areas";

/** An area, and how many observed edits there are in it. */
export type Touch = { path: string; w: number };

/** Two or more drones changing the same area, or one changing what main has also changed. */
export type Clash = { path: string; root: string; keys: string[] };

/** How many items a burst carries: Fleet keeps the newest 25. */
const MOST_ITEMS = 25;

const byWeight = (a: Touch, b: Touch) => b.w - a.w || a.path.localeCompare(b.path);

/** Paths counted to the areas they are in, biggest share first. */
function rolled(paths: readonly string[]): Touch[] {
  const counts = new Map<string, number>();
  for (const path of paths) {
    const area = areaOf(path);
    if (area !== undefined) counts.set(area, (counts.get(area) ?? 0) + 1);
  }
  return [...counts].map(([path, w]) => ({ path, w })).sort(byWeight);
}

/** The paths a burst wrote: a Session's items. */
function writtenBy(event: TimelineEvent): readonly string[] {
  if (event.kind === "session.edited") return event.items ?? [];
  return [];
}

/**
 * The areas each drone has edited, by its key. A drone with no observed edit is not in the map: nothing observed is
 * nothing touched. `rootsOf` gives the checkouts a drone's paths may be spelled under.
 */
export function touchesFromEvents(
  events: readonly TimelineEvent[],
  of: { keyOf: (event: TimelineEvent) => string | undefined; rootsOf: (key: string) => readonly string[] },
): Map<string, Touch[]> {
  const paths = new Map<string, string[]>();
  for (const event of events) {
    const written = writtenBy(event);
    const key = written.length === 0 ? undefined : of.keyOf(event);
    if (key === undefined) continue;
    const roots = of.rootsOf(key);
    paths.set(key, [...(paths.get(key) ?? []), ...written.slice(0, MOST_ITEMS).map((path) => relativeTo(path, roots))]);
  }
  const touched = new Map<string, Touch[]>();
  for (const [key, list] of paths) {
    const areas = rolled(list);
    if (areas.length > 0) touched.set(key, areas);
  }
  return touched;
}

/**
 * The areas each holder of a slot has changed in it, by its key, from Fleet's read of the slots against the base. The
 * paths are repository-relative already, and a holder whose changes are all at the top of a repository is not in the map.
 */
export function touchesOfSlots(read: SlotTouches): Map<string, Touch[]> {
  const touched = new Map<string, Touch[]>();
  for (const repository of read.repositories) {
    for (const holder of repository.holders) {
      const areas = rolled(holder.paths);
      if (areas.length > 0) touched.set(`${holder.kind}:${holder.id}`, areas);
    }
  }
  return touched;
}

/** What a drone touches: what its tool bursts placed, else what Fleet read of its slot, else nothing, so it rests at its pad. */
export const touchesOf = (key: string, observed: ReadonlyMap<string, readonly Touch[]>, slotted: ReadonlyMap<string, readonly Touch[]>): readonly Touch[] => observed.get(key) ?? slotted.get(key) ?? [];

/**
 * The areas two holders are both writing, from the write-clash read: one mark per area, naming the drones that hold it.
 * Main is not a drone, and a holder the ship does not hold has no key, so a clash against main marks one drone's areas and
 * a clash between two strangers marks none.
 */
export function clashesOf(clashes: readonly WriteClash[], of: { keyOf: (kind: string, id: string) => string | undefined; rootOf: (key: string) => string | undefined }): Clash[] {
  const found = new Map<string, Clash>();
  for (const clash of clashes) {
    const keys = [clash.one, clash.other].flatMap((holder) => {
      const key = of.keyOf(holder.kind, holder.id);
      return key === undefined ? [] : [key];
    });
    const root = keys[0] === undefined ? undefined : of.rootOf(keys[0]);
    if (root === undefined) continue;
    for (const area of new Set(clash.paths.flatMap((path) => areaOf(path) ?? []))) {
      const at = `${root} ${area}`;
      const have = found.get(at);
      if (have === undefined) found.set(at, { path: area, root, keys });
      else have.keys = [...new Set([...have.keys, ...keys])];
    }
  }
  return [...found.values()];
}
