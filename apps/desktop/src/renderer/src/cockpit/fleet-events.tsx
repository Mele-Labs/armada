// The Fleet's events as the Cockpit's timeline draws them: one row per thing that happened, naming what it
// happened to. Fleet serves them as `get_fleet_timeline` and `timeline.appended` (`@armada/protocol`,
// `TimelineEvent`); `fleet-timeline.ts` reads them in the app, and a mock scenario carries its own list.
// Both reach the window through `FleetEventsFrom` or `FleetEventsOver`, and `fromWire` is where an entry
// becomes what the takes draw.

import { createContext, useContext, useMemo, useSyncExternalStore, type ReactNode } from "react";

import type { SessionState } from "@armada/components";
import type { DashboardTab } from "@armada/overview";
import type { TimelineEvent, TimelineKind, TimelineTone } from "@armada/protocol";

import type { TimelineSource } from "../fleet-timeline";

/** What an event is a state of, as the colour reads it: working, needs him, failing, passed, waiting. */
export type EventTone = TimelineTone;

export type Subject =
  | { kind: "job"; id: string; title: string }
  | { kind: "session"; id: string; title: string }
  | { kind: "pull_request"; number: number; title: string; url?: string }
  | { kind: "main" };

export type EventKind = TimelineKind;

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
  /** The repository it belongs to, by manifest id. Empty where Fleet could not place it. */
  repo: string;
  /** The subject is still working after this. */
  live?: boolean;
  /** The subject is over after this: landed, failed, merged, ended. */
  over?: boolean;
};

const NOTHING: readonly TimelineEvent[] = [];
const NO_SOURCE: TimelineSource = { get: () => NOTHING, subscribe: () => () => undefined };
const Held = createContext<TimelineSource>(NO_SOURCE);

/** Hand a moment's events to the window, fixed. The mock's own mount is the one caller. */
export function FleetEventsFrom({ events, children }: { events: readonly TimelineEvent[] | undefined; children: ReactNode }) {
  const source = useMemo<TimelineSource>(() => ({ get: () => events ?? NOTHING, subscribe: () => () => undefined }), [events]);
  return <Held.Provider value={source}>{children}</Held.Provider>;
}

/** Hand the window what Fleet serves. Nothing is read until a timeline is on screen to subscribe. */
export function FleetEventsOver({ source, children }: { source: TimelineSource; children: ReactNode }) {
  return <Held.Provider value={source}>{children}</Held.Provider>;
}

/** The entries this window holds, oldest first, current while the caller is mounted. Empty where nothing serves them. */
export function useFleetEvents(): readonly TimelineEvent[] {
  const source = useContext(Held);
  return useSyncExternalStore(source.subscribe, source.get);
}

/** What a subject is called where the entry itself carries no title: the Jobs and Sessions the window holds. */
export type Names = { job: (id: string) => string | undefined; session: (id: string) => string | undefined };

/** An entry as the takes draw it. A subject Fleet gave no title takes the window's own name for it. */
export function fromWire(event: TimelineEvent, names: Names): FleetEvent {
  const { subject } = event;
  const drawn: Subject =
    subject.kind === "job"
      ? { kind: "job", id: subject.id, title: event.title ?? names.job(subject.id) ?? "Job" }
      : subject.kind === "session"
        ? { kind: "session", id: subject.id, title: event.title ?? names.session(subject.id) ?? "Session" }
        : subject.kind === "pull_request"
          ? { kind: "pull_request", number: subject.number, title: event.title ?? "", ...(subject.url === undefined ? {} : { url: subject.url }) }
          : { kind: "main" };
  return {
    id: String(event.id),
    at: event.at,
    kind: event.kind,
    subject: drawn,
    tone: event.tone,
    says: event.says,
    ...(event.detail === undefined ? {} : { detail: event.detail }),
    repo: event.manifest_id ?? "",
    ...(event.live === true ? { live: true } : {}),
    ...(event.over === true ? { over: true } : {}),
  };
}

export const keyOf = (subject: Subject): string =>
  subject.kind === "job" ? `job:${subject.id}` : subject.kind === "session" ? `session:${subject.id}` : subject.kind === "pull_request" ? `pr:${subject.number}` : "main";

export const titleOf = (subject: Subject): string =>
  subject.kind === "pull_request" ? (subject.title === "" ? `#${subject.number}` : `#${subject.number} ${subject.title}`) : subject.kind === "main" ? "main" : subject.title;

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

/** A Job working and silent this long is stuck. A Session is not read this way: Fleet says whether it is (`session-state.ts`). */
export const STUCK_MS = 10 * 60_000;

export type Lane = {
  key: string;
  subject: Subject;
  repo: string;
  /** Newest first. */
  events: FleetEvent[];
  standing: Standing;
  /** A Session's state as the Sessions list reads it, where the window holds the Session. */
  session?: SessionState;
  /** When its newest event happened, in ms. */
  last: number;
};

const when = (event: FleetEvent): number => Date.parse(event.at);

/**
 * Where a Session's lane goes, from its state alone. **Never an event's `live` flag or its age**: a
 * Session that finished a turn is not moving, and one that is quiet is stuck only when Fleet says so.
 */
function sessionStanding(state: SessionState): Standing {
  switch (state) {
    case "waiting":
    case "replied":
    case "done":
      return "ask";
    case "stuck":
      return "stuck";
    case "working":
      return "run";
    case "ended":
      return "done";
    default:
      return "idle";
  }
}

/**
 * One lane per subject, grouped by standing then newest first. `reading` is the state of a Session the
 * window holds, as the Sessions list reads it; a Session it does not hold stands by its newest event's
 * tone and is never moving or stuck.
 */
export function lanesOf(events: readonly FleetEvent[], now: number, reading: (sessionId: string) => SessionState | undefined = () => undefined): Lane[] {
  const by = new Map<string, FleetEvent[]>();
  for (const event of events) {
    const key = keyOf(event.subject);
    by.set(key, [...(by.get(key) ?? []), event]);
  }
  const lanes = [...by].map(([key, own]): Lane => {
    const newest = own.sort((a, b) => when(b) - when(a));
    const head = newest[0]!;
    const read = head.subject.kind === "session" ? reading(head.subject.id) : undefined;
    const standing: Standing =
      head.over === true
        ? "done"
        : read !== undefined
          ? sessionStanding(read)
          : head.tone === "ask"
            ? "ask"
            : head.tone === "bad"
              ? "bad"
              : head.subject.kind !== "session" && head.live === true
                ? now - when(head) > STUCK_MS
                  ? "stuck"
                  : "run"
                : "idle";
    return { key, subject: head.subject, repo: head.repo, events: newest, standing, ...(read === undefined ? {} : { session: read }), last: when(head) };
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
