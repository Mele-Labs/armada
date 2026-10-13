// The Fleet's events as the Cockpit's timeline reads them: one row per thing that happened, naming
// what it happened to. Mock only. Fleet publishes the facts on `/events` and a Job's own moves in
// `get_job_history`, but serves no fleet-wide, timestamped window of them, and a pull request,
// a Session or a slot lease moves as a whole row replaced rather than as an event. A scenario
// carries these and hands them down through `FleetEventsFrom`; a real Fleet hands down none.

import { createContext, useContext, type ReactNode } from "react";

import type { DashboardTab } from "@armada/overview";

/** What an event is a state of, as the colour reads it: working, needs him, failing, passed, waiting. */
export type EventTone = "run" | "ask" | "bad" | "ok" | "idle";

export type Subject =
  | { kind: "job"; id: string; number: number; title: string }
  | { kind: "session"; id: string; title: string }
  | { kind: "pull_request"; number: number; title: string; url: string }
  | { kind: "main" };

/** What happened. Each names the wire fact it would be read from, or says none is. */
export type EventKind =
  | "job.created" // job.created
  | "job.queued" // job.state_changed to queued
  | "job.started" // job.state_changed to running
  | "job.step" // job.step_advanced
  | "job.check" // job.checking
  | "job.judged" // job.judging
  | "job.files" // job.files_changed
  | "job.asked" // job.asking
  | "job.answered" // none: the answer is a command, and the row it clears is replaced
  | "job.landed" // job.landed
  | "job.failed" // job.state_changed to a failed end
  | "session.started" // session.changed, first row
  | "session.turn" // session.row
  | "session.asked" // session.changed, waiting_for appears
  | "session.answered" // none: as job.answered
  | "session.ended" // session.changed, dead
  | "slot.leased" // none: worktrees are read whole, never published
  | "pr.opened" // none: merge_lines.changed carries the list whole
  | "pr.checks" // none: as pr.opened
  | "pr.queued" // none: as pr.opened
  | "pr.merged" // none: as pr.opened
  | "main.red"; // none: as pr.opened

export type FleetEvent = {
  id: string;
  /** When it happened. */
  at: string;
  kind: EventKind;
  subject: Subject;
  tone: EventTone;
  /** The fact, bare. */
  says: string;
  /** What it opens to, in mono: a command, a branch, a Check's name. */
  detail?: string;
  /** The repository it belongs to, by name. */
  repo: string;
  /** The subject is still working after this. */
  live?: boolean;
  /** The subject is over after this: landed, failed, merged, ended. */
  over?: boolean;
};

const Held = createContext<readonly FleetEvent[]>([]);

/** Hand a moment's events to the window. The mock's own mount is the one caller. */
export function FleetEventsFrom({ events, children }: { events: readonly FleetEvent[] | undefined; children: ReactNode }) {
  return <Held.Provider value={events ?? []}>{children}</Held.Provider>;
}

/** The events this window was mounted holding, in no order. Empty where a Fleet serves none. */
export function useFleetEvents(): readonly FleetEvent[] {
  return useContext(Held);
}

export const keyOf = (subject: Subject): string =>
  subject.kind === "job" ? `job:${subject.id}` : subject.kind === "session" ? `session:${subject.id}` : subject.kind === "pull_request" ? `pr:${subject.number}` : "main";

export const titleOf = (subject: Subject): string =>
  subject.kind === "pull_request" ? `#${subject.number} ${subject.title}` : subject.kind === "main" ? "main" : subject.title;

/** How a subject stands, from its newest event: the order the lanes group in. */
export type Standing = "ask" | "bad" | "stuck" | "run" | "idle" | "done";

export const STANDINGS: readonly { id: Standing; said: string }[] = [
  { id: "ask", said: "Needs you" },
  { id: "bad", said: "Failing" },
  { id: "stuck", said: "Stuck" },
  { id: "run", said: "Moving" },
  { id: "idle", said: "Waiting" },
  { id: "done", said: "Finished" },
];

/** A subject working and silent this long is stuck. */
export const STUCK_MS = 10 * 60_000;

export type Lane = {
  key: string;
  subject: Subject;
  repo: string;
  /** Newest first. */
  events: FleetEvent[];
  standing: Standing;
  /** When its newest event happened, in ms. */
  last: number;
};

const when = (event: FleetEvent): number => Date.parse(event.at);

/** One lane per subject, grouped by standing then newest first. */
export function lanesOf(events: readonly FleetEvent[], now: number): Lane[] {
  const by = new Map<string, FleetEvent[]>();
  for (const event of events) {
    const key = keyOf(event.subject);
    by.set(key, [...(by.get(key) ?? []), event]);
  }
  const lanes = [...by].map(([key, own]): Lane => {
    const newest = own.sort((a, b) => when(b) - when(a));
    const head = newest[0]!;
    const standing: Standing = head.over === true ? "done" : head.tone === "ask" ? "ask" : head.tone === "bad" ? "bad" : head.live === true ? (now - when(head) > STUCK_MS ? "stuck" : "run") : "idle";
    return { key, subject: head.subject, repo: head.repo, events: newest, standing, last: when(head) };
  });
  const rank = (lane: Lane) => STANDINGS.findIndex((one) => one.id === lane.standing);
  return lanes.sort((a, b) => rank(a) - rank(b) || b.last - a.last);
}

/** The lanes the Cockpit's filter shows: what needs him, what is not over, what is. */
export function lanesFor(lanes: readonly Lane[], filter: DashboardTab): Lane[] {
  if (filter === "command-central") return lanes.filter((one) => one.standing === "ask" || one.standing === "bad" || one.standing === "stuck");
  if (filter === "done") return lanes.filter((one) => one.standing === "done");
  return lanes.filter((one) => one.standing !== "done");
}

/** Whole minutes since `at`, as the list rows say it: nothing under a minute, then `4m`, `2h`. */
export function ageOf(at: string, now: number): string {
  const minutes = Math.max(0, Math.floor((now - Date.parse(at)) / 60_000));
  if (minutes < 1) return "now";
  return minutes < 60 ? `${minutes}m` : `${Math.floor(minutes / 60)}h`;
}
