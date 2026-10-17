// What the battlefield is handed, as pure readings: which drones carry a call marker, which action animations loop, what a callout
// says of a drone besides the drone, what arrives while the field is on screen, and the two things that keep a busy window quiet,
// the clock quantized and the timeline's appends held to four a second. **Every fact here is read off what Fleet recorded.** Nothing
// draws. Ported from proto/cockpit-flight-deck. #2301.

import type { TimelineEvent } from "@armada/protocol";

import { MOST_LOOPS } from "./doing";
import type { DroneIn, FileAct, FileIn } from "./layout";
import { MOST_PLAYING, type Play } from "./plays";

// ---- A call marker ------------------------------------------------------------------------------------

export type CallTone = "ask" | "bad";

/**
 * The drones that hold a call he has not opened, amber while it is owed and red while its Check is failing. `owedOf` is the call a
 * drone's subject holds, already past the ones answered. **In a replay** the marks are the drones that were calling at the playhead,
 * from how the events left them, and what has been opened since is no part of that moment. A drone the filters dim carries none.
 */
export function callMarkersOf(drones: readonly DroneIn[], owedOf: (key: string) => string | undefined, visited: ReadonlySet<string>, live: boolean): Map<string, CallTone> {
  const out = new Map<string, CallTone>();
  for (const drone of drones) {
    if (!drone.shown) continue;
    if (!live) {
      if (drone.ping) out.set(drone.key, drone.flare ? "bad" : "ask");
      continue;
    }
    const owed = owedOf(drone.key);
    if (owed !== undefined && !visited.has(owed)) out.set(drone.key, drone.flare ? "bad" : "ask");
  }
  return out;
}

// ---- The loops --------------------------------------------------------------------------------------------

/** Which of the drones that are doing something loop their action: at most `MOST_LOOPS`, the focused first and then the selected and the one under the pointer. The rest wear the same form still. */
export function loopsOf(doers: readonly DroneIn[], first: { focus?: string | undefined; selected?: string | undefined; hot?: string | undefined }, most: number = MOST_LOOPS): Set<string> {
  const rank = (key: string) => (key === first.focus ? 0 : key === first.selected ? 1 : key === first.hot ? 2 : 3);
  return new Set(
    [...doers]
      .sort((a, b) => rank(a.key) - rank(b.key))
      .slice(0, most)
      .map((one) => one.key),
  );
}

// ---- What the callout says besides the drone ---------------------------------------------------------------

const HOUR = 60 * 60_000;
/** One bar for each five minutes of the last hour. */
export const BUCKETS = 12;

const when = (event: TimelineEvent) => Date.parse(event.at);

/** A drone's last event up to the clock, and its last hour as bars, newest on the right. An event after the clock (a minute of slack) is not yet. */
export function eventFactsOf(events: readonly TimelineEvent[], key: string, keyOf: (event: TimelineEvent) => string | undefined, clock: number): { last?: { says: string; at: string }; ticks: number[] } {
  const ticks = Array.from({ length: BUCKETS }, () => 0);
  let last: TimelineEvent | undefined;
  for (const event of events) {
    if (keyOf(event) !== key || when(event) > clock + 60_000) continue;
    if (last === undefined || when(event) > when(last)) last = event;
    const ago = clock - when(event);
    if (ago >= 0 && ago < HOUR) ticks[BUCKETS - 1 - Math.floor(ago / (HOUR / BUCKETS))]! += 1;
  }
  return { ...(last === undefined ? {} : { last: { says: last.says, at: last.at } }), ticks };
}

/** The areas a drone touches, the biggest two, each shortened from the left. Nothing observed is no pill. */
export const pillsOf = (drone: Pick<DroneIn, "touches">): string[] =>
  [...drone.touches]
    .sort((a, b) => b.w - a.w || a.path.localeCompare(b.path))
    .slice(0, 2)
    .map((one) => (one.path.length > 26 ? `…${one.path.slice(-25)}` : one.path));

// ---- What arrives while the field is drawn ------------------------------------------------------------------

/**
 * What happened to the field since it was last drawn: a drone that was not there lifts from its base, and a pull request that has
 * landed since is captured. Each is played once (`seen`), none on the first draw, and at most a dozen at a time.
 */
export function arrivalsOf(input: {
  was: ReadonlySet<string> | undefined;
  landedWas: ReadonlySet<string> | undefined;
  drones: readonly string[];
  landed: ReadonlyMap<number, string>;
  seen: ReadonlySet<string>;
}): Play[] {
  const { was, landedWas, drones, landed, seen } = input;
  const out: Play[] = [];
  if (was !== undefined) for (const key of drones) if (!was.has(key) && !seen.has(`arrived:${key}`)) out.push({ id: `arrived:${key}`, kind: "launch", key, at: Date.now(), delay: 0 });
  if (landedWas !== undefined) for (const [number, root] of landed) if (!landedWas.has(String(number)) && !seen.has(`merged:${number}`)) out.push({ id: `merged:${number}`, kind: "capture", root, number, at: Date.now(), delay: 0 });
  return out.slice(-MOST_PLAYING);
}

// ---- A busy window stays quiet --------------------------------------------------------------------------------

/** How often the field reads a new clock: the moment that only moves a figure by a minute is not worth a redraw of the plane. */
export const CLOCK_MS = 5000;
export const quantized = (now: number, step: number = CLOCK_MS): number => Math.floor(now / step) * step;

/** The field takes timeline appends no faster than this: four a second. */
export const TICK_MS = 250;

type Timers = { now: () => number; later: (run: () => void, ms: number) => void };
const REAL: Timers = { now: () => Date.now(), later: (run, ms) => void window.setTimeout(run, ms) };

/**
 * A value delivered at once when it has been quiet, and otherwise held to the end of the tick, only the latest of those that
 * arrived in it. A burst of appends is one redraw, and the last word is never lost.
 */
export function throttled<T>(deliver: (value: T) => void, ms: number = TICK_MS, timers: Timers = REAL): { push: (value: T) => void } {
  let last = -Infinity;
  let waiting: { value: T } | undefined;
  let scheduled = false;
  const send = (value: T) => {
    last = timers.now();
    deliver(value);
  };
  return {
    push: (value) => {
      const quiet = timers.now() - last;
      if (quiet >= ms && !scheduled) return send(value);
      waiting = { value };
      if (scheduled) return;
      scheduled = true;
      timers.later(() => {
        scheduled = false;
        const held = waiting;
        waiting = undefined;
        if (held !== undefined) send(held.value);
      }, Math.max(0, ms - quiet));
    },
  };
}

// ---- The files Sessions name -----------------------------------------------------------------------------

/** Which act stands where two Sessions name one file. */
const ACT_RANK: Record<FileAct | "rest", number> = { remove: 3, write: 2, edit: 1, rest: 0 };

/**
 * The files the live Sessions name, for each repository, one entry for each path. Where two name one path the stronger act stands: a file
 * taken away over one made, one made over one changed, one changed over one at rest. `key` says what the lists hold, so a message that moves
 * no file hands the layout the same list.
 */
export function filesNamed(named: Iterable<{ root: string; file: FileIn }>): { files: Map<string, FileIn[]>; key: string } {
  const by = new Map<string, Map<string, FileAct | undefined>>();
  for (const { root, file } of named) {
    const into = by.get(root) ?? new Map<string, FileAct | undefined>();
    by.set(root, into);
    if (!into.has(file.path) || ACT_RANK[file.act ?? "rest"] > ACT_RANK[into.get(file.path) ?? "rest"]) into.set(file.path, file.act);
  }
  const key = [...by].map(([root, paths]) => `${root}:${[...paths].map(([path, act]) => `${path}=${act ?? ""}`).sort().join(",")}`).sort().join("|");
  const files = new Map([...by].map(([root, paths]) => [root, [...paths].map(([path, act]) => (act === undefined ? { path } : { path, act })).sort((a, b) => a.path.localeCompare(b.path))] as [string, FileIn[]]));
  return { files, key };
}
