// Replay: the field as it was at a time, derived from the events up to it, and the clock that plays it forward.
//
// **Its source is the events Bridge holds, at their real times, and nothing earlier.** The window is the last two hours and
// at most 2000 events, the newest, so a replay is offered only from the first of them and only where there is one. Before
// that first event nothing is known: a bay is `unknown` (`bays.ts`), and a replay cannot be started there. Fleet records no
// touched paths over time, so a drone replayed stands where its edits observed now put it. #2301.

import { useEffect, useRef } from "react";
import type { TimelineEvent, TimelineKind } from "@armada/protocol";
import { holdsText } from "@armada/screens/src/keys";

import type { DroneIn, PrIn } from "./layout";
import { MOST_PLAYING, droneOfEvent, playOfEvent } from "./plays";
import { changeReplay, deckNow, stopReplay, useDeck } from "./store";

/** The longest a replay waits between two events, wall time. A silent gap longer than that is skipped. */
const LONGEST_WAIT = 2000;
const TICK = 120;
/** How long an effect of a passed event stays drawn. */
const STAYS = 8000;
export const SPEEDS = [1, 4, 16] as const;

/** How far back a replay reaches, and the most events it holds. Fleet cuts a timeline read to 2000. */
export const WINDOW_MS = 2 * 60 * 60_000;
export const MOST_EVENTS = 2000;

const when = (event: TimelineEvent) => Date.parse(event.at);
/** The events in the order they happened. */
export const inOrder = (events: readonly TimelineEvent[]): TimelineEvent[] => [...events].sort((a, b) => when(a) - when(b) || a.id - b.id);

/** The events a replay can play, oldest first, and the instant the first of them happened. */
export type ReplayWindow = { events: TimelineEvent[]; from: number };

/** The window of the events held at `now`: the last two hours, and of those the newest 2000. None where there is no event in it. */
export function windowOf(events: readonly TimelineEvent[], now: number): ReplayWindow | undefined {
  const held = inOrder(events.filter((event) => now - when(event) <= WINDOW_MS && when(event) <= now)).slice(-MOST_EVENTS);
  const first = held[0];
  return first === undefined ? undefined : { events: held, from: when(first) };
}

/** Where a replay starts: where he asked, but never before the first event. With no ask, at the first. */
export const startAt = (window: ReplayWindow, wanted: number | undefined): number => Math.max(window.from, wanted ?? window.from);

/** What an event says about where its subject stands: the kinds that make a drone or a pull request. A later event does not. */
const STARTS: ReadonlySet<TimelineKind> = new Set(["session.started", "pr.opened"]);

/** A drone as its last event before the playhead left it: working through a Check, idle after, calling between an ask and its answer. */
function asOf(base: DroneIn, last: TimelineEvent): DroneIn {
  const ask = last.tone === "ask";
  const bad = last.tone === "bad";
  const working = last.tone === "run" || (last.live === true && !ask && !bad);
  return { ...base, said: last.says, hue: bad ? "bad" : ask ? "ask" : "calm", spin: working, hover: working || ask, ping: ask || bad, stuck: false, flare: bad, working: last.live === true && last.over !== true };
}

const stageOf = (last: TimelineEvent): PrIn["stage"] =>
  last.kind === "pr.merged" ? "landed" : last.kind === "pr.queued" ? "queued" : last.kind === "pr.checks" ? (last.tone === "bad" ? "red" : last.tone === "ok" ? "ready" : "checking") : "checking";

/**
 * The drones and the pull requests as they were at `at`. A drone whose first event is its start is absent until then, and then
 * stands as its last event left it. One whose first event is not a start was running before the window, and one the events say
 * nothing about is not known to be otherwise: each stands as it does now. A pull request appears at its opening and is moved along
 * its route by the rest.
 */
export function fieldAt(events: readonly TimelineEvent[], at: number, drones: readonly DroneIn[], prs: readonly PrIn[], ownerOf: (number: number) => string | undefined): { drones: DroneIn[]; prs: PrIn[] } {
  const byDrone = new Map<string, TimelineEvent[]>();
  const byPr = new Map<number, TimelineEvent[]>();
  for (const event of events) {
    const key = droneOfEvent(event, ownerOf);
    if (key !== undefined && event.subject.kind !== "pull_request") byDrone.set(key, [...(byDrone.get(key) ?? []), event]);
    if (event.subject.kind === "pull_request") byPr.set(event.subject.number, [...(byPr.get(event.subject.number) ?? []), event]);
  }
  const lastBefore = (list: readonly TimelineEvent[]) => list.filter((one) => when(one) <= at).at(-1);
  return {
    drones: drones.flatMap((drone) => {
      const own = byDrone.get(drone.key);
      if (own === undefined) return [drone];
      const last = lastBefore(own);
      if (last !== undefined) return [asOf(drone, last)];
      return STARTS.has(own[0]!.kind) ? [] : [drone];
    }),
    prs: prs.flatMap((pr) => {
      const own = byPr.get(pr.number);
      if (own === undefined) return [pr];
      const last = lastBefore(own);
      if (last !== undefined) return [{ ...pr, stage: stageOf(last) }];
      return STARTS.has(own[0]!.kind) ? [] : [pr];
    }),
  };
}

/** The instant of the event before `at` and the one after it, where there is one. */
export const neighbours = (events: readonly TimelineEvent[], at: number): { before: number | undefined; after: number | undefined } => {
  const times = inOrder(events).map(when);
  return { before: times.filter((t) => t < at).at(-1), after: times.find((t) => t > at) };
};

/** Moves the playhead to `to` and plays the effects of the events it passed. */
export function seek(events: readonly TimelineEvent[], to: number, ownerOf: (number: number) => string | undefined, rootOf: (manifest: string | undefined) => string | undefined): void {
  const was = deckNow().replay;
  if (was === undefined) return;
  const passed = inOrder(events).filter((event) => when(event) > was.at && when(event) <= to);
  const plays = passed.flatMap((event) => {
    const play = playOfEvent(event, `r${was.run}:${event.id}`, ownerOf, rootOf);
    return play === undefined ? [] : [play];
  });
  changeReplay({ at: to, plays: [...was.plays.filter((one) => Date.now() - one.at < STAYS), ...plays].slice(-MOST_PLAYING) });
}

/** Plays the replay forward while it is playing; arriving at now is arriving at Live. **Now is the clock's own**, read each step: the deck is handed the minute. */
export function useReplayDriver(events: readonly TimelineEvent[], ownerOf: (number: number) => string | undefined, rootOf: (manifest: string | undefined) => string | undefined): void {
  const latest = useRef({ events, ownerOf, rootOf });
  latest.current = { events, ownerOf, rootOf };
  const replay = useDeck((deck) => deck.replay);
  const playing = replay?.playing === true;
  const speed = replay?.speed ?? 1;
  useEffect(() => {
    if (!playing) return;
    // Reduced motion steps from event to event, slowly, and does not ease between them.
    const reduced = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true;
    let last = performance.now();
    const timer = window.setInterval(
      () => {
        const clock = performance.now();
        const waited = clock - last;
        last = clock;
        const here = deckNow().replay;
        const { events: all } = latest.current;
        const end = Date.now();
        if (here === undefined || !here.playing) return;
        const next = neighbours(all, here.at).after;
        let to = here.at;
        if (next === undefined || next >= end) to = end;
        else if (reduced) to = next;
        else {
          if (next - to > LONGEST_WAIT * speed) to = next - LONGEST_WAIT * speed;
          to += waited * speed;
        }
        to = Math.min(to, end);
        seek(all, to, latest.current.ownerOf, latest.current.rootOf);
        if (to >= end) stopReplay();
      },
      reduced ? 700 : TICK,
    );
    return () => window.clearInterval(timer);
  }, [playing, speed]);
}

/**
 * The keys a replay takes while it is up: Space plays and pauses, the arrows step an event, Home starts over, Esc or Shift+L is
 * back to Live. Esc and L are the replay's wherever focus is; the rest only where the arrows would otherwise pan the map or
 * move nothing, so the list and the panels keep theirs.
 */
export function claimsReplay(event: KeyboardEvent, panel: string | undefined): boolean {
  if (event.metaKey || event.ctrlKey || event.altKey || holdsText(event.target)) return false;
  if (event.key === "Escape" || event.key === "L") return true;
  if (panel !== undefined && panel !== "map") return false;
  if (event.key === " ") return !(event.target instanceof HTMLElement && event.target.tagName === "BUTTON");
  return event.key === "ArrowLeft" || event.key === "ArrowRight" || event.key === "Home";
}

export function useReplayKeys(active: boolean, events: readonly TimelineEvent[], ownerOf: (number: number) => string | undefined, rootOf: (manifest: string | undefined) => string | undefined, panelOf: (target: EventTarget | null) => string | undefined): void {
  const latest = useRef({ events, ownerOf, rootOf, panelOf });
  latest.current = { events, ownerOf, rootOf, panelOf };
  useEffect(() => {
    if (!active) return;
    const press = (event: KeyboardEvent) => {
      if (document.querySelector('[role="dialog"], [role="alertdialog"]') !== null) return;
      const here = deckNow().replay;
      const { events: all, ownerOf: owner, rootOf: root, panelOf: panel } = latest.current;
      if (here === undefined || !claimsReplay(event, panel(event.target))) return;
      event.preventDefault();
      event.stopPropagation();
      const { before, after } = neighbours(all, here.at);
      if (event.key === "Escape" || event.key === "L") stopReplay();
      else if (event.key === " ") changeReplay({ playing: !here.playing });
      else if (event.key === "Home") changeReplay({ at: here.from, plays: [] });
      else if (event.key === "ArrowLeft") changeReplay({ at: before === undefined ? here.from : Math.max(here.from, before), plays: [] });
      else if (after === undefined) stopReplay();
      else seek(all, after, owner, root);
    };
    window.addEventListener("keydown", press, true);
    return () => window.removeEventListener("keydown", press, true);
  }, [active]);
}
