// What a Job's drones are doing now, for its Field and the strip over it: the Job's own drone from its thread's rows and its timeline
// entries (`cockpit/deck/off-floor.ts`), each subagent from the ledger and Fleet's touches. Also the lift-off of a subagent that starts
// and the return of one that ends, which play once each and are gone. A message to or from another Job is a word on the strip and an arc
// between the two drones. Read from what Fleet recorded, never from what an agent said.

import { useEffect, useMemo, useRef, useState } from "react";
import type { TimelineEvent } from "@armada/protocol";
import { owesOf, standingOf, type Session, type WorkDrone } from "@armada/screens/src/draft/sessions";

import { relativeTo } from "../cockpit/deck/areas";
import type { DroneIn } from "../cockpit/deck/layout";
import { THINK_MS, burstOf, quietAt, callingOf, lastToolAt, lightsOf, mainNow, messageWord, subagentNow, wordOf, type CheckRecord, type Light, type LinkIn, type Message, type Now } from "../cockpit/deck/off-floor";
import type { Play } from "../cockpit/deck/plays";

/** One button of the strip: the drone it frames, its name, what it is doing in words, and whether it has finished. */
export type StripEntry = { key: string; name: string; word: string; done: boolean };

/** How long a lift-off or a return is kept on the Field: past the longest the play moves. */
const PLAY_MS = 4_000;

/** Whether the person asked for no motion: a subagent then appears and goes at once. */
const still = (): boolean => typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/** Whether `ms` have passed since `since`, drawn again once when they have: one timer, and none while there is nothing to wait for. */
function useQuiet(since: number | undefined, ms: number, armed: boolean): boolean {
  const [, bump] = useState(0);
  const quiet = armed && quietAt(since, Date.now(), ms);
  useEffect(() => {
    if (!armed || since === undefined || quiet) return;
    const timer = window.setTimeout(() => bump((n) => n + 1), Math.max(0, since + ms - Date.now()) + 20);
    return () => window.clearTimeout(timer);
  }, [armed, since, ms, quiet]);
  return quiet;
}

/** The Job's own drone now, its pad's lights and its walk. `clock` moves the reading on as a fresh call or burst goes stale. */
export function useMainNow(session: Session, events: readonly TimelineEvent[], roots: readonly string[], clock: number, file: string | undefined): { now: Now; lights: Light[]; walk: string | undefined } {
  const rows = session.rows;
  const working = session.turn.state === "working";
  const checks = useMemo(() => rows.flatMap((one): CheckRecord[] => (one.kind === "check" ? [{ name: one.name, at: one.at, state: one.state, ...(one.blame === undefined ? {} : { blame: one.blame }) }] : [])), [rows]);
  const stamps = useMemo(() => rows.flatMap((one) => (one.kind === "tool" || one.kind === "check" ? [one.at] : [])), [rows]);
  const last = lastToolAt(stamps, events);
  const quiet = useQuiet(last, THINK_MS, working);
  const editedAt = useMemo(() => {
    let at: number | undefined;
    for (const one of events) if (one.kind === "session.edited") at = Math.max(at ?? -Infinity, Date.parse(one.at));
    return at;
  }, [events]);
  const lights = useMemo(() => lightsOf(checks, editedAt), [checks, editedAt]);
  // A walk is open from the window row Fleet wrote until it records the walk's end, and never on a Job that is over.
  const shown = [...rows].reverse().find((one) => one.kind === "window");
  const walk = shown?.kind === "window" && shown.ended === undefined && session.dead === undefined ? shown.title : undefined;
  // The present, not the clock's last step: a call or a burst that has just arrived is read at once, and the clock moves it on later.
  const at = Math.max(clock, Date.now());
  const burst = burstOf(events, at);
  const now = mainNow({
    working,
    walk,
    checking: checks.some((one) => one.state === "running"),
    quiet,
    call: callingOf(events, at),
    burst: burst === undefined ? undefined : { act: burst.act, ...(burst.detail === undefined ? {} : { file: relativeTo(burst.detail, roots) }) },
    file,
    owed: session.asked !== undefined || owesOf(session).length > 0,
    done: standingOf(session) === "done",
  });
  return { now, lights, walk };
}

/**
 * The Job's drone, then each subagent the ledger holds in the order they started: finished ones too, which the strip keeps listed. While
 * a message to or from another Job is fresh the Job's drone says that (`talking`), unless it is waiting on you.
 */
export function stripOf(session: Session, main: Now, field: readonly WorkDrone[] | undefined, talking?: string): StripEntry[] {
  const own = `session:${session.id}`;
  const placed = new Map((field ?? []).map((one) => [one.id, one] as const));
  const subagents = session.attachments.flatMap((one) => (one.kind === "subagent" ? [one] : []));
  return [
    { key: own, name: session.title ?? session.id, word: talking !== undefined && main.kind !== "ask" ? talking : wordOf(main), done: false },
    ...subagents.map((one) => {
      const file = placed.get(one.id)?.file ?? (one.call === undefined ? undefined : placed.get(one.call)?.file);
      return { key: `${own}/${one.id}`, name: one.task, word: wordOf(subagentNow(one.state === "done", file)), done: one.state === "done" };
    }),
  ];
}

/**
 * The Job's fresh messages as the Field draws them: an arc for each other Job whose drone is on it, from the sender's drone to the
 * receiver's, and the strip's word for the newest. Another Job with no drone here draws no arc, and its word still shows.
 */
export function talkOf(self: string, messages: readonly Message[], ghosts: readonly DroneIn[], titleOf: (id: string) => string | undefined): { links: LinkIn[]; word: string | undefined } {
  const own = `session:${self}`;
  const here = new Set(ghosts.map((one) => one.key));
  const title = (one: Message): string => titleOf(one.other) ?? one.title ?? "another Job";
  const links = messages.flatMap((one): LinkIn[] => {
    const other = `session:${one.other}`;
    if (!here.has(other)) return [];
    return [{ key: `message:${one.other}`, from: one.sent ? own : other, to: one.sent ? other : own, other, title: title(one), quote: one.quote, sent: one.sent }];
  });
  const newest = messages[0];
  return { links, word: newest === undefined ? undefined : messageWord(newest, title(newest)) };
}

/** The Job's own drone as its state draws it: off the floor over the pad or risen over its file, calling, or at the file it is reading or writing. */
export function placedNow(drone: DroneIn, now: Now): DroneIn {
  const { doing: _doing, via: _via, ...rest } = drone;
  switch (now.kind) {
    case "checks":
    case "walk":
      return { ...rest, off: now.kind === "checks" ? "pad" : "walk", touches: [] };
    case "think":
      return { ...rest, off: "think" };
    case "call":
      return { ...rest, calling: now.service };
    case "read":
    case "write":
      return { ...rest, doing: now.kind, via: now.file, file: now.file, touches: [{ path: now.file, w: 4 }] };
    case "run":
      return { ...rest, doing: "run" };
    case "ask":
      return { ...rest, hue: "ask", ping: true };
    default:
      return rest;
  }
}

/** Between two readings of the subagents on the Field: a lift-off for each that is new, and a return for each that has gone, kept still. */
export function liftsAndReturns(was: ReadonlyMap<string, DroneIn>, now: ReadonlyMap<string, DroneIn>, at: number): { added: Play[]; gone: DroneIn[] } {
  const lifts = [...now.keys()].filter((key) => !was.has(key)).map((key): Play => ({ id: `lift:${key}:${at}`, kind: "launch", key, at, delay: 0 }));
  const gone = [...was.values()].filter((one) => !now.has(one.key)).map(({ doing: _doing, via: _via, ...one }): DroneIn => ({ ...one, spin: false, working: false, touches: [] }));
  const docks = gone.map((one): Play => ({ id: `dock:${one.key}:${at}`, kind: "dock", key: one.key, at, delay: 0 }));
  return { added: [...lifts, ...docks], gone };
}

/**
 * A subagent that starts lifts off the drone that sent it, and one that ends flies back into it and is gone: a play each, and the one
 * that ended kept on the Field until its return has played. None on the first draw, while scrubbed back, or under reduced motion.
 */
export function useLiftsAndReturns(drones: readonly DroneIn[], live: boolean): { drones: readonly DroneIn[]; plays: readonly Play[] } {
  const seen = useRef<Map<string, DroneIn> | undefined>(undefined);
  const [plays, setPlays] = useState<readonly Play[]>([]);
  const [leaving, setLeaving] = useState<readonly DroneIn[]>([]);
  const timers = useRef(new Set<number>());
  useEffect(() => () => timers.current.forEach((one) => window.clearTimeout(one)), []);
  useEffect(() => {
    const now = new Map(drones.filter((one) => one.parent !== undefined).map((one) => [one.key, one] as const));
    const was = seen.current;
    seen.current = now;
    if (was === undefined || !live || still()) return;
    const { added, gone } = liftsAndReturns(was, now, Date.now());
    if (added.length === 0) return;
    setPlays((list) => [...list, ...added]);
    setLeaving((list) => [...list, ...gone]);
    const timer = window.setTimeout(() => {
      timers.current.delete(timer);
      setPlays((list) => list.filter((one) => !added.includes(one)));
      setLeaving((list) => list.filter((one) => !gone.includes(one)));
    }, PLAY_MS);
    timers.current.add(timer);
  }, [drones, live]);
  const all = useMemo(() => (leaving.length === 0 ? drones : [...drones, ...leaving.filter((one) => !drones.some((it) => it.key === one.key))]), [drones, leaving]);
  return { drones: all, plays };
}
