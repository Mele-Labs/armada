// The services Sessions reach outside the repository, read from the timeline's `session.called` entries. One outpost for each service any
// Session has called, at the far edge of the field; it appears with the first call seen. A Session whose latest call is fresh, and which is
// working, is calling that service now: a beam runs from its drone to the outpost and the outpost is lit. Read from the entries Fleet
// recorded, never from an agent's own report.

import type { TimelineEvent } from "@armada/protocol";

import { age } from "../../Dashboard";
import { FRESH_MS } from "./doing";
import type { DroneIn } from "./layout";
import { droneOfEvent } from "./plays";

/** The glyph an outpost wears: the registry's names (`packages/icons`), and the lucide components are chosen from them where it is drawn. */
export type OutpostGlyph = "forge" | "ticket" | "globe" | "plug";

/** The most calls an outpost's card lists. */
export const MOST_CALLS = 5;

export type Outpost = {
  /** The service: `forge`, `tracker`, `web`, or an MCP server by its own name. */
  service: string;
  /** What the card and a screen reader call it: the newest call's name, as a label. */
  name: string;
  glyph: OutpostGlyph;
  /** The recent calls to it, newest first, each as the card says it. */
  lines: readonly string[];
};

const FIXED: Record<string, { glyph: OutpostGlyph; rank: number }> = {
  forge: { glyph: "forge", rank: 0 },
  tracker: { glyph: "ticket", rank: 1 },
  web: { glyph: "globe", rank: 2 },
};

/** A service's name as a label: `the web` is `Web`, and a name already capitalised is left alone. */
const labelOf = (name: string): string => {
  const bare = name.replace(/^the /, "");
  return `${bare.charAt(0).toUpperCase()}${bare.slice(1)}`;
};

const when = (event: TimelineEvent): number => Date.parse(event.at);

/** The calls up to `clock`, newest first. */
const calledOf = (events: readonly TimelineEvent[], clock: number): TimelineEvent[] =>
  events.filter((one) => one.kind === "session.called" && one.call !== undefined && when(one) <= clock).sort((a, b) => when(b) - when(a) || b.id - a.id);

/**
 * The outposts as they stand at `clock`: the forge, the issue tracker and the web in that order when they have been called, then the other
 * services alphabetically. `keep` drops the calls the map does not draw.
 */
export function outpostsOf(events: readonly TimelineEvent[], clock: number, keep: (event: TimelineEvent) => boolean = () => true): Outpost[] {
  const by = new Map<string, TimelineEvent[]>();
  for (const event of calledOf(events, clock)) {
    if (!keep(event)) continue;
    const service = event.call!.service;
    by.set(service, [...(by.get(service) ?? []), event]);
  }
  const nameOf = (service: string): string => labelOf(by.get(service)![0]!.call!.name);
  return [...by.keys()]
    .sort((a, b) => (FIXED[a]?.rank ?? 3) - (FIXED[b]?.rank ?? 3) || nameOf(a).localeCompare(nameOf(b)))
    .map((service) => ({
      service,
      name: nameOf(service),
      glyph: FIXED[service]?.glyph ?? "plug",
      lines: by.get(service)!.slice(0, MOST_CALLS).map((one) => `${one.title ?? "Session"} · ${one.call!.line} · ${age(one.at, clock)}`),
    }));
}

/**
 * The drones as they are calling at `clock`: a working drone whose latest call is still fresh is calling that service. The same freshness
 * a tool burst has (`doing.ts`), and the same rule that an idle, waiting or finished drone does not.
 */
export function withCalling(
  drones: readonly DroneIn[],
  events: readonly TimelineEvent[],
  clock: number,
  ownerOf: (number: number) => string | undefined,
): DroneIn[] {
  const latest = new Map<string, TimelineEvent>();
  for (const event of calledOf(events, clock)) {
    const key = droneOfEvent(event, ownerOf);
    if (key !== undefined && !latest.has(key)) latest.set(key, event);
  }
  return drones.map((drone) => {
    const seen = latest.get(drone.key);
    return seen !== undefined && drone.working && when(seen) + FRESH_MS > clock ? { ...drone, calling: seen.call!.service } : drone;
  });
}
