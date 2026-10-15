// Everything the battlefield draws, read once off the ship, the events and the replay: the drones with what each has touched and is
// doing, the territories, the bays as pads, the pull requests as crates, what plays and what each drone's callout says. The map
// shows all of it and a Job's mission the part that is its own. **Nothing here is held locally and nothing is invented**: a drone no
// edit places rests at its pad or base, a territory with no areas draws no plots, and a replay stands drones as the events left them.
// The timeline's appends arrive no faster than four a second (`fleet-timeline.ts`) and the clock moves in five-second steps, so a busy
// window draws the field a few times a second at most. #2301.

import { useEffect, useMemo, useRef, useState } from "react";
import type { TimelineEvent } from "@armada/protocol";

import { age } from "../../Dashboard";
import { useWriteClashes } from "../../clashes";
import { useSessions } from "../../sessions-draft";
import type { CockpitProps } from "./props";
import { useFleetEvents } from "../fleet-events";
import { useAreas } from "./areas-source";
import { useSlotTouches } from "./touches-source";
import { holdersOf, slotKey, useBays } from "./bays";
import { withDoing, doingOfEvent } from "./doing";
import { CLOCK_MS, arrivalsOf, callMarkersOf, eventFactsOf } from "./field";
import { useClock } from "./clock";
import { droneOf, type PrIn, type RepoIn } from "./layout";
import { fieldAt, windowOf } from "./replay";
import { MOST_PLAYING, droneOfEvent, playOfEvent, playsOf, trailsOf, type Play } from "./plays";
import { toneOf } from "./order";
import { callOfSubject, scene, useDeck } from "./store";
import { slotsOf, type Ship, type Subject } from "./ship";
import { clashesOf, touchesFromEvents, touchesOf } from "./touches";
import type { DroneFacts, GoalIn, PadIn } from "./tip";

type FieldProps = Pick<CockpitProps, "state" | "picked">;

/** What the field plays, from the events and from what changes while it is drawn: each played once, and the most that move at once is capped. */
function usePlays(candidates: readonly Play[], drones: readonly string[], landed: ReadonlyMap<number, string>): Play[] {
  const [plays, setPlays] = useState<Play[]>(() => candidates.filter((one) => !scene.seen.has(one.id)));
  const shown = useRef(plays);
  shown.current = plays;
  // What is shown now is shown: it is not played again when the field is drawn another time.
  useEffect(() => {
    for (const one of plays) scene.seen.add(one.id);
    if (plays.length === 0) return;
    const timer = window.setTimeout(() => setPlays([]), 8000);
    return () => window.clearTimeout(timer);
  }, [plays]);
  // Arrivals: a drone that was not there, a pull request that has landed, an event that was not there.
  useEffect(() => {
    const arrived = arrivalsOf({ was: scene.drones, landedWas: scene.landed, drones, landed, seen: scene.seen });
    scene.drones = new Set(drones);
    scene.landed = new Set([...landed.keys()].map(String));
    const late = [...candidates.filter((one) => !scene.seen.has(one.id) && !shown.current.some((p) => p.id === one.id)), ...arrived].filter((one) => !scene.seen.has(one.id));
    if (late.length > 0) setPlays((now) => [...now, ...late].slice(-MOST_PLAYING));
  }, [candidates, drones, landed]);
  return plays;
}

export function useField(props: FieldProps, ship: Ship, listed: readonly Subject[]) {
  const answered = useDeck((deck) => deck.held);
  const visited = useDeck((deck) => deck.visited);
  const replay = useDeck((deck) => deck.replay);
  const focus = useDeck((deck) => deck.focus);
  const sessions = useSessions();
  const events = useFleetEvents();
  const areasOf = useAreas();
  const slotTouched = useSlotTouches();
  const written = useWriteClashes();
  // The field moves in five-second steps whatever the deck is handed.
  const clock = useClock(CLOCK_MS);
  const shown = useMemo(() => new Set(listed.map((one) => one.key)), [listed]);
  const slots = slotsOf(props.state, props.picked);
  const repositories = props.state.holds.repositories ?? [];
  const rootOfManifest = (manifest: string | undefined) => repositories.find((one) => one.manifest?.id === manifest)?.root;
  const repoNameOf = (manifest: string) => repositories.find((one) => one.manifest?.id === manifest)?.manifest?.repository ?? manifest;

  const holder = useMemo(() => holdersOf(slots), [slots]);
  const all = useMemo(() => ship.subjects.filter((one) => one.kind !== "pr"), [ship.subjects]);
  const ownerOf = (number: number) => {
    const pr = ship.prs.find((one) => one.number === number);
    return pr?.owner === undefined ? undefined : `${pr.owner.kind}:${pr.owner.id}`;
  };

  // What each drone has edited, from the bursts Fleet recorded: the paths of its checkout, rolled up to areas.
  const touched = useMemo(
    () =>
      touchesFromEvents(events, {
        keyOf: (event) => (event.subject.kind === "job" || event.subject.kind === "session" ? `${event.subject.kind}:${event.subject.id}` : undefined),
        rootsOf: (key) => {
          const slot = holder.get(key);
          return slot === undefined ? [] : [slot.path];
        },
      }),
    [events, holder],
  );

  const drones = useMemo(() => {
    const has = new Set(all.map((one) => one.key));
    return all.map((one) => {
      const attached = one.kind === "session" ? sessions.find((row) => row.id === one.id)?.attachments.find((a) => a.kind === "job" && a.looking !== true) : undefined;
      const made = one.kind === "job" ? props.state.jobs.find((job) => job.id === one.id)?.dispatched_by : undefined;
      const parent = attached !== undefined && attached.kind === "job" ? `job:${attached.id}` : made === undefined ? undefined : `job:${made}`;
      const slot = holder.get(one.key);
      return droneOf(one, callOfSubject(one, answered) !== undefined, {
        parent: parent !== undefined && has.has(parent) ? parent : undefined,
        slot: slot === undefined ? undefined : slotKey(slot),
        shown: shown.has(one.key),
        // What a tool burst placed, else what Fleet read of the holder's slot against the base; else nothing.
        touches: touchesOf(one.key, touched, slotTouched),
      });
    });
  }, [all, sessions, holder, props.state.jobs, answered, shown, touched, slotTouched]);

  const repos = useMemo<RepoIn[]>(() => {
    const roots = [...new Set([...all.map((one) => one.root), ...ship.mains.map((one) => one.root), ...slots.map((one) => rootOfManifest(one.manifest_id) ?? one.manifest_id)])].sort();
    return roots.map((root) => ({
      root,
      name: ship.repos.find((one) => one.root === root)?.name ?? all.find((one) => one.root === root)?.repo ?? root,
      pads: slots
        .filter((one) => (rootOfManifest(one.manifest_id) ?? one.manifest_id) === root)
        .sort((a, b) => a.slot - b.slot)
        .map(slotKey),
      goal: ship.mains.some((one) => one.root === root),
      areas: areasOf(root),
    }));
  }, [all, ship.mains, ship.repos, slots, repositories, areasOf]);

  const bays = useBays(slots, events, replay?.at, clock);
  const pads = useMemo(() => {
    const out = new Map<string, PadIn>();
    for (const { view } of bays) {
      const who = view.holder ?? view.last;
      const name = who === undefined ? undefined : (who.title ?? who.id);
      out.set(view.key, {
        key: view.key,
        slot: view.slot.slot,
        bay: view.state === "leased" ? (view.kept === undefined ? "held" : "kept") : view.state,
        word: view.word,
        repo: repoNameOf(view.slot.manifest_id),
        Glyph: view.Glyph,
        ...(name === undefined ? {} : { holder: name }),
        ...(who === undefined ? {} : { holderKind: who.kind }),
        ...(view.holder === undefined && who !== undefined ? { holderWas: true } : {}),
        ...(view.branch === undefined ? {} : { branch: view.branch }),
      });
    }
    return out;
  }, [bays, repositories]);

  const goals = useMemo(() => new Map<string, GoalIn>(ship.mains.map((main) => [main.root, { root: main.root, red: main.state === "red", said: main.state === "red" ? `Main is red in ${main.repo}` : `Main is green in ${main.repo}` }])), [ship.mains]);
  const prs = useMemo<PrIn[]>(() => ship.prs.map((pr) => ({ key: pr.key, number: pr.number, stage: pr.stage, root: pr.root, ...(pr.owner === undefined ? {} : { owner: `${pr.owner.kind}:${pr.owner.id}` }) })), [ship.prs]);
  const landed = useMemo(() => new Map(ship.prs.filter((pr) => pr.stage === "landed").map((pr) => [pr.number, pr.root] as const)), [ship.prs]);
  const candidates = useMemo(() => playsOf(events, clock, ownerOf, rootOfManifest), [events, clock, ship.prs, repositories]);
  const trails = useMemo(() => trailsOf(events, clock, ownerOf), [events, clock, ship.prs]);
  const plays = usePlays(
    candidates,
    useMemo(() => drones.map((one) => one.key), [drones]),
    landed,
  );

  // A picked event, and a replay: the field as it was at the playhead, from the events up to it. A replay is the window's: the last two hours.
  const window2h = useMemo(() => windowOf(events, clock), [events, clock]);
  const ordered = window2h?.events ?? [];
  const passed = replay === undefined ? 0 : ordered.filter((one) => Date.parse(one.at) <= replay.at).length;
  const before = useMemo(() => (replay === undefined ? events : ordered.filter((one) => Date.parse(one.at) <= replay.at)), [events, ordered, replay === undefined, passed]);
  const field = useMemo(() => (replay === undefined ? undefined : fieldAt(ordered, replay.at, drones, prs, ownerOf)), [ordered, replay === undefined, replay?.run, passed, drones, prs]);
  const picked = focus === undefined ? undefined : events.find((one) => String(one.id) === focus);
  const focusKey = picked === undefined ? undefined : droneOfEvent(picked, ownerOf);
  // What each working drone is doing, from the tool events up to the playhead; a picked tool event shows its own action on its own drone.
  const held = replay === undefined && picked !== undefined && focusKey !== undefined && doingOfEvent(picked) !== undefined ? { key: focusKey, event: picked } : undefined;
  const flying = useMemo(() => withDoing(field?.drones ?? drones, before, replay?.at ?? clock, ownerOf, held), [field, drones, before, clock, held?.event.id]);
  const crates = field?.prs ?? prs;
  const flightTrails = useMemo(() => (replay === undefined ? trails : trailsOf(before, replay.at, ownerOf)), [trails, replay === undefined, passed]);
  // The event's own effect, once, where it is picked.
  const focusPlays = useMemo(() => {
    const play = picked === undefined ? undefined : playOfEvent(picked, `focus:${picked.id}`, ownerOf, rootOfManifest);
    return play === undefined ? [] : [play];
  }, [focus]);
  const drawn = replay === undefined ? [...plays, ...focusPlays] : replay.plays;
  const at = replay?.at ?? clock;

  // What each drone's callout says besides the drone: its state as the list draws it, the call it holds, its last event, the hour of events as bars.
  const facts = useMemo(() => {
    const out = new Map<string, DroneFacts>();
    const keyOf = (event: TimelineEvent) => droneOfEvent(event, ownerOf);
    for (const drone of flying) {
      const subject = ship.subjects.find((one) => one.key === drone.key);
      if (subject === undefined) continue;
      const { last, ticks } = eventFactsOf(before, drone.key, keyOf, at);
      const owed = callOfSubject(subject, answered);
      const call = owed === undefined ? undefined : ship.calls.find((one) => one.key === owed)?.title;
      const pad = drone.slot === undefined ? undefined : pads.get(drone.slot);
      out.set(drone.key, {
        state: { Glyph: subject.mark.Glyph, said: subject.mark.said, tone: toneOf(subject) },
        ticks,
        ...(call === undefined ? {} : { call }),
        ...(last === undefined ? {} : { last: { says: last.says, ago: age(last.at, at) } }),
        ...(pad === undefined ? {} : { bay: { slot: pad.slot, Glyph: pad.Glyph, word: pad.word } }),
      });
    }
    return out;
  }, [flying, ship.subjects, ship.calls, before, at, answered, pads, ship.prs]);

  // The drones that hold a call he has not opened: a call is marked until it is visited or answered. In a replay, those that were calling at the playhead.
  const calls = useMemo(
    () =>
      callMarkersOf(
        flying,
        (key) => {
          const subject = ship.subjects.find((one) => one.key === key);
          return subject === undefined ? undefined : callOfSubject(subject, answered);
        },
        visited,
        replay === undefined,
      ),
    [flying, ship.subjects, answered, visited, replay === undefined],
  );

  // Where Fleet says two holders write one area, as the marks of the drones the map holds.
  const clashes = useMemo(
    () =>
      clashesOf(written, {
        keyOf: (kind, id) => (kind === "job" || kind === "session" ? `${kind}:${id}` : undefined),
        rootOf: (key) => all.find((one) => one.key === key)?.root,
      }),
    [written, all],
  );

  return { calls, repos, pads, goals, crates, drones: flying, flightTrails, drawn, facts, clashes, focusKey, replay, ordered, picked, ownerOf, rootOfManifest, events };
}
