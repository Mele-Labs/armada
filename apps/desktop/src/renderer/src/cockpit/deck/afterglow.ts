// What a Job's own Field reads off its reads and edits: the glow each file keeps after it was touched, the route of the Job's stops,
// and where its drone was at a moment of the slider. **Read from the times of Fleet's `session.read` and `session.edited` bursts**,
// never from what an agent said. A glow fades linearly to nothing: ten minutes after a write, five after a read. Pure.

import type { TimelineEvent } from "@armada/protocol";

import { relativeTo } from "./areas";
import { inOrder } from "./replay";

export type Act = "read" | "write";

/** One burst of reads or writes: when it began, and the files it named, newest first, as the repository spells them. */
export type Burst = { at: number; act: Act; paths: readonly string[] };

/** How a file's bar glows: from what was done to it last that still shows, and how much of that is left, 0 to 1. */
export type Glow = { act: Act; v: number };

/** A stop of the route: its number from the first stop, the file the drone was at, and whether it read or wrote it. */
export type Stop = { n: number; path: string; act: Act };

/** How long a glow takes to fade, in milliseconds. */
export const GLOW_MS: Readonly<Record<Act, number>> = { write: 10 * 60_000, read: 5 * 60_000 };

/** A glow is held to fortieths, so a clock that moves while nothing new happens redraws the Field only when a glow has visibly moved. */
const STEPS = 40;

/** The most stops the route draws, the newest. */
export const MOST_STOPS = 12;

/** The Job's bursts, oldest first, with each path made relative to the checkouts it may be spelled under. */
export function burstsOf(events: readonly TimelineEvent[], session: string, roots: readonly string[]): Burst[] {
  const out: Burst[] = [];
  for (const event of inOrder(events)) {
    if (event.subject.kind !== "session" || event.subject.id !== session) continue;
    if (event.kind !== "session.read" && event.kind !== "session.edited") continue;
    const named = event.items ?? (event.detail === undefined ? [] : [event.detail]);
    const paths = named.map((path) => relativeTo(path, roots)).filter((path) => path !== "");
    if (paths.length > 0) out.push({ at: Date.parse(event.at), act: event.kind === "session.read" ? "read" : "write", paths });
  }
  return out;
}

/** The glow of every file at `at`: for each, the strongest of what is left of its reads and writes. A file nothing lights is absent. */
export function glowAt(bursts: readonly Burst[], at: number): Map<string, Glow> {
  const out = new Map<string, Glow>();
  for (const burst of bursts) {
    if (burst.at > at) break;
    const v = Math.round((1 - (at - burst.at) / GLOW_MS[burst.act]) * STEPS) / STEPS;
    if (v <= 0) continue;
    for (const path of burst.paths) {
      const was = out.get(path);
      if (was === undefined || was.v < v) out.set(path, { act: burst.act, v });
    }
  }
  return out;
}

/** The stops up to `at`: one for each burst, at the newest file it named, a repeat of the stop before folded into it. The newest `most`. */
export function routeAt(bursts: readonly Burst[], at: number, most: number = MOST_STOPS): Stop[] {
  const stops: Stop[] = [];
  for (const burst of bursts) {
    if (burst.at > at) break;
    const path = burst.paths[0]!;
    const last = stops.at(-1);
    if (last !== undefined && last.path === path && last.act === burst.act) continue;
    stops.push({ n: stops.length + 1, path, act: burst.act });
  }
  return stops.slice(-most);
}

/** Where the Job's drone was at `at`: the newest file of its last burst before then, and what it was doing there. None before its first. */
export function droneAt(bursts: readonly Burst[], at: number): { path: string; act: Act } | undefined {
  let last: Burst | undefined;
  for (const burst of bursts) {
    if (burst.at > at) break;
    last = burst;
  }
  return last === undefined ? undefined : { path: last.paths[0]!, act: last.act };
}
