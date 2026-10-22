// The activity log as motion on the flight deck's battlefield: what happened, to which drone, and which motion tells it.
// Nothing here draws or moves anything.

import type { TimelineEvent, TimelineKind } from "@armada/protocol";

import type { Pr } from "./ship";

// ---- Pull requests as payloads -------------------------------------------------------------------------

/** How far along its route to the objective a pull request has got, by where it stands. */
export const ALONG: Record<Pr["stage"], number> = { checking: 0.22, red: 0.22, ready: 0.55, queued: 0.74, landed: 1 };

export const STAGE_SAID: Record<Pr["stage"], string> = { checking: "Checks running", red: "Checks failed", ready: "Ready", queued: "In the queue", landed: "Merged" };

// ---- The activity log as motion ------------------------------------------------------------------------

/**
 * What an event looks like on the field.
 *  launch   a drone lifts from its pad to its place, a contrail behind it
 *  sweep    a radar cone turns on the drone that is running a Check
 *  payload  a pull request leaves its drone for the objective
 *  capture  the objective flashes: a merge landed
 *  tracer   a short light runs between a Session and what it answered
 *  blip     one ring goes out from the drone: something was said or decided
 *  flare    the objective's beacon flares: main went red or green
 *  dock     a subagent that has finished flies back into the drone that sent it, and is gone
 */
export type PlayKind = "launch" | "sweep" | "payload" | "capture" | "tracer" | "blip" | "flare" | "dock";

export type Play = {
  id: string;
  kind: PlayKind;
  /** The drone it happens to, where it is one. */
  key?: string;
  root?: string;
  /** The pull request, where it is one's. */
  number?: number;
  at: number;
  delay: number;
};

export const PLAYS: Partial<Record<TimelineKind, PlayKind>> = {
  "session.started": "launch",
  "slot.leased": "launch",
  "pr.checks": "sweep",
  "pr.opened": "payload",
  "pr.queued": "payload",
  "pr.merged": "capture",
  "session.turn": "tracer",
  "session.answered": "tracer",
  "session.asked": "blip",
  "main.red": "flare",
  "main.green": "flare",
};

/** Events this new play when the field is drawn, the rest only leave their trail. */
export const RECENT_MS = 6 * 60_000;
/** How long a launch leaves its contrail on the ground. */
export const TRAIL_MS = 45 * 60_000;
/** The most that move at once. */
export const MOST_PLAYING = 12;

/** The key of the drone an event happened to, through the pull request's owner where it is one's. */
export function droneOfEvent(event: TimelineEvent, ownerOf: (number: number) => string | undefined): string | undefined {
  const { subject } = event;
  if (subject.kind === "session") return `session:${subject.id}`;
  if (subject.kind === "pull_request") return ownerOf(subject.number);
  return undefined;
}

/** One event as the play it is, or nothing where it only leaves a trail. A replay and a focused row both play events this way. */
export function playOfEvent(event: TimelineEvent, id: string, ownerOf: (number: number) => string | undefined, rootOf: (manifest: string | undefined) => string | undefined): Play | undefined {
  const kind = PLAYS[event.kind];
  if (kind === undefined) return undefined;
  const key = droneOfEvent(event, ownerOf);
  const root = rootOf(event.manifest_id);
  const number = event.subject.kind === "pull_request" ? event.subject.number : undefined;
  return { id, kind, at: Date.now(), delay: 0, ...(key === undefined ? {} : { key }), ...(root === undefined ? {} : { root }), ...(number === undefined ? {} : { number }) };
}

/** The events recent enough to move, newest first, as plays. */
export function playsOf(events: readonly TimelineEvent[], now: number, ownerOf: (number: number) => string | undefined, rootOf: (manifest: string | undefined) => string | undefined): Play[] {
  return events
    .flatMap((event): Play[] => {
      const kind = PLAYS[event.kind];
      const at = Date.parse(event.at);
      if (kind === undefined || now - at > RECENT_MS || at - now > 60_000) return [];
      const key = droneOfEvent(event, ownerOf);
      const root = rootOf(event.manifest_id);
      const number = event.subject.kind === "pull_request" ? event.subject.number : undefined;
      return [{ id: `event:${event.id}`, kind, at, delay: 0, ...(key === undefined ? {} : { key }), ...(root === undefined ? {} : { root }), ...(number === undefined ? {} : { number }) }];
    })
    .sort((a, b) => b.at - a.at)
    .slice(0, MOST_PLAYING)
    .map((play, i) => ({ ...play, delay: i * 220 }));
}

/** When a drone was last dispatched, from the events, for the contrail it left. Absent where there is none or it is long gone. */
export function trailsOf(events: readonly TimelineEvent[], now: number, ownerOf: (number: number) => string | undefined): Map<string, number> {
  const trails = new Map<string, number>();
  for (const event of events) {
    if (event.kind !== "session.started" && event.kind !== "slot.leased") continue;
    const key = droneOfEvent(event, ownerOf);
    const age = now - Date.parse(event.at);
    if (key === undefined || age < 0 || age > TRAIL_MS) continue;
    trails.set(key, Math.min(trails.get(key) ?? Infinity, age));
  }
  return trails;
}

