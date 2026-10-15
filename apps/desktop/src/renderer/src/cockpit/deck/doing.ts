// What a working drone is doing over its plot: reading it, writing it, or running tools against it. Read from the
// timeline's tool events, never from an agent's own report: Fleet records them from the hooks that observe the tools.
//
// **A burst is dated by its first call and written when it closes** (15 seconds quiet, or 2 minutes at the longest), so
// it can only be the present for 2 minutes and 15 seconds after its `at`. After that it says what the drone did, not what
// it is doing, and a stale burst reads as no action: the drone shows its form still. #2301.

import { PencilLine, Radar, ScanSearch, type LucideIcon } from "lucide-react";
import type { TimelineEvent, TimelineKind } from "@armada/protocol";

import type { DroneIn } from "./layout";
import { droneOfEvent } from "./plays";

/** Read, Grep and Glob are reading; Edit and Write are writing; Bash and tests are running. */
export type Doing = "read" | "write" | "run";
/** How a Check stands, for the ring a running drone circles on: running now, passed, failing. */
export type Check = "run" | "ok" | "bad";

/** The timeline kinds that say what a tool did. A Job's `job.files` names one file and is not a burst. */
const DOING: Partial<Record<TimelineKind, Doing>> = {
  "session.read": "read",
  "job.read": "read",
  "session.edited": "write",
  "session.ran": "run",
  "job.ran": "run",
};

/** How long after its `at` a burst can still be the present: the longest it stays open, and the quiet that closes it. */
export const FRESH_MS = 2 * 60_000 + 15_000;

export const doingOfEvent = (event: TimelineEvent): Doing | undefined => DOING[event.kind];
export const WORD: Record<Doing, string> = { read: "Reading", write: "Writing", run: "Running" };
export const GLYPH: Record<Doing, LucideIcon> = { read: ScanSearch, write: PencilLine, run: Radar };
/** The most action loops that run at once; the rest of the working drones show the same form still. */
export const MOST_LOOPS = 8;

const when = (event: TimelineEvent) => Date.parse(event.at);

/**
 * The drones as they are doing at `clock`: a working drone takes what its latest tool event up to then says, while that
 * event is fresh, with the state of its Check where that was a run. One that is idle, waiting or over does none of these.
 * `held` is a picked tool event, which is shown on its own drone whether or not it is working or fresh, so a feed row
 * and the map speak of the same action.
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
    const seen = latest.get(drone.key);
    const fresh = seen !== undefined && when(seen) + FRESH_MS > clock ? seen : undefined;
    const event = picked ?? (drone.working ? fresh : undefined);
    const doing = event === undefined ? undefined : doingOfEvent(event);
    if (event === undefined || doing === undefined) return drone;
    const check: Check = drone.flare || event.tone === "bad" ? "bad" : event.tone === "ok" ? "ok" : "run";
    return { ...drone, doing, check, ...(event.detail === undefined ? {} : { via: event.detail }) };
  });
}
