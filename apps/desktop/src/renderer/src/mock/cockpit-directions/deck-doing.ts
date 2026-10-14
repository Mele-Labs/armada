// What a working drone is doing over its plot: reading it, writing it, or running tools and Checks against it. Read from the
// timeline's tool events, never from an agent's own report: Fleet records them from the hooks that observe the tools, as
// `session.read`, `session.edited`, `session.ran`, `job.read`, `job.files` and `job.ran` (the fleet/timeline-tool-events work).
// Mock only. proto/cockpit-flight-deck.

import { PencilLine, Radar, ScanSearch, type LucideIcon } from "lucide-react";
import type { TimelineEvent } from "@armada/protocol";

import type { DroneIn } from "./deck-field";
import { droneOfEvent } from "./deck-field";

/** Read, Grep and Glob are reading; Edit, Write and a commit are writing; Bash, tests and Checks are running. */
export type Doing = "read" | "write" | "run";
/** How a Check stands, for the ring a running drone circles on: running now, passed, failing. */
export type Check = "run" | "ok" | "bad";

/** The timeline kinds that say what a tool did. The protocol's union does not carry the tool kinds until Fleet's event work lands. */
const DOING: Readonly<Record<string, Doing>> = {
  "session.read": "read",
  "job.read": "read",
  "session.edited": "write",
  "job.files": "write",
  "session.ran": "run",
  "job.ran": "run",
  "job.check": "run",
  "job.judged": "run",
};

export const doingOfEvent = (event: TimelineEvent): Doing | undefined => DOING[event.kind];
export const WORD: Record<Doing, string> = { read: "Reading", write: "Writing", run: "Running" };
export const GLYPH: Record<Doing, LucideIcon> = { read: ScanSearch, write: PencilLine, run: Radar };
/** The most action loops that run at once; the rest of the working drones show the same form still. */
export const MOST_LOOPS = 8;

const when = (event: TimelineEvent) => Date.parse(event.at);

/**
 * The drones as they are doing at `clock`: a working drone takes what its latest tool event up to then says, with the state of
 * its Check where that was a run. One that is idle, waiting or over does none of these. `held` is a picked tool event, which
 * is shown on its own drone whether or not it is working, so a feed row and the map speak of the same action.
 */
export function withDoing(
  drones: readonly DroneIn[],
  events: readonly TimelineEvent[],
  clock: number,
  ownerOf: (number: number) => string | undefined,
  held?: { key: string; event: TimelineEvent },
): DroneIn[] {
  const latest = new Map<string, TimelineEvent>();
  for (const event of events) {
    if (doingOfEvent(event) === undefined || when(event) > clock) continue;
    const key = droneOfEvent(event, ownerOf);
    const was = key === undefined ? undefined : latest.get(key);
    if (key !== undefined && (was === undefined || when(event) >= when(was))) latest.set(key, event);
  }
  return drones.map((drone) => {
    const picked = held?.key === drone.key ? held.event : undefined;
    const event = picked ?? (drone.working ? latest.get(drone.key) : undefined);
    const doing = event === undefined ? undefined : doingOfEvent(event);
    if (event === undefined || doing === undefined) return drone;
    const check: Check = drone.flare || event.tone === "bad" ? "bad" : event.tone === "ok" ? "ok" : "run";
    return { ...drone, doing, check, ...(event.detail === undefined ? {} : { via: event.detail }) };
  });
}
