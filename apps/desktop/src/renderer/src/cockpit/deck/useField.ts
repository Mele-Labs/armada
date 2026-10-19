// Everything the battlefield draws, read once off the ship, the events and the replay: the drones with what each has touched and is
// doing, the territories, the bays as pads, the pull requests as crates, what plays and what each drone's callout says. The map
// shows all of it. **Nothing here is held locally and nothing is invented**: a drone no
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
import { buildTipOf, shapedBy } from "./build";
import { useBuilds } from "./useBuilds";
import { useSlotTouches } from "./touches-source";
import { holdersOf, slotKey, useBays } from "./bays";
import { withDoing, doingOfEvent } from "./doing";
import { outpostsOf, withCalling } from "./outposts";
import { CLOCK_MS, arrivalsOf, callMarkersOf, eventFactsOf, filesNamed } from "./field";
import { useClock } from "./clock";
import { droneOf, type FileIn, type PrIn, type RepoIn } from "./layout";
import { fieldAt, windowOf } from "./replay";
import { MOST_PLAYING, droneOfEvent, playOfEvent, playsOf, trailsOf, type Play } from "./plays";
import { toneOf } from "./order";
import { callOfSubject, scene, useDeck } from "./store";
import { slotsOf, type Ship, type Subject } from "./ship";
import { clashesOf, touchesFromEvents, touchesOf } from "./touches";
import type { DroneFacts, GoalIn, PadIn } from "./tip";
import { jobIdOf, manifestLabel } from "@armada/shell";

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

/** `only` is the root of the one repository the map draws; none draws every one. */
export function useField(props: FieldProps, ship: Ship, listed: readonly Subject[], only?: string) {
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
  const repositories = props.state.holds.repositories ?? [];
  const rootOfManifest = (manifest: string | undefined) => repositories.find((one) => jobIdOf(one) === manifest)?.root;
  const slots = useMemo(() => slotsOf(props.state, props.picked).filter((one) => only === undefined || (rootOfManifest(one.manifest_id) ?? one.manifest_id) === only), [props.state, props.picked, only, repositories]);
  const repoNameOf = (manifest: string) => manifestLabel(manifest, repositories);

  const holder = useMemo(() => holdersOf(slots), [slots]);
  const all = useMemo(() => ship.subjects.filter((one) => one.kind !== "pr" && one.past === undefined && (only === undefined || one.root === only)), [ship.subjects, only]);
  const mains = useMemo(() => ship.mains.filter((one) => only === undefined || one.root === only), [ship.mains, only]);
  const shipPrs = useMemo(() => ship.prs.filter((one) => only === undefined || one.root === only), [ship.prs, only]);
  const ownerOf = (number: number) => {
    const pr = ship.prs.find((one) => one.number === number);
    return pr?.owner === undefined ? undefined : `${pr.owner.kind}:${pr.owner.id}`;
  };

  // What each drone has edited, from the bursts Fleet recorded: the paths of its checkout, rolled up to areas.
  const touched = useMemo(
    () =>
      touchesFromEvents(events, {
        keyOf: (event) => (event.subject.kind === "session" ? `${event.subject.kind}:${event.subject.id}` : undefined),
        rootsOf: (key) => {
          const slot = holder.get(key);
          return slot === undefined ? [] : [slot.path];
        },
      }),
    [events, holder],
  );

  const drones = useMemo(() => {
    return all.map((one) => {
      const slot = holder.get(one.key);
      return droneOf(one, callOfSubject(one, answered) !== undefined, {
        slot: slot === undefined ? undefined : slotKey(slot),
        // The file a Session is on, as its own Field has it: its drone hovers at that pill.
        file: one.kind === "session" ? sessions.find((row) => row.id === one.id)?.work?.field?.find((at) => at.agent === undefined)?.file : undefined,
        shown: shown.has(one.key),
        // What a tool burst placed, else what Fleet read of the holder's slot against the base; else nothing.
        touches: touchesOf(one.key, touched, slotTouched),
      });
    });
  }, [all, sessions, holder, answered, shown, touched, slotTouched]);

  // The files each live Session names, drawn as pills on their plots as the Session's own Field draws them. Where two name one path the stronger act
  // stands: a file taken away over one made, one made over one changed, one changed over one at rest. Read each render, kept by what it says, so a
  // Session's message that moves no file hands the layout the same list.
  const named: { root: string; file: FileIn }[] = [];
  for (const one of all) {
    if (one.kind !== "session") continue;
    const row = sessions.find((session) => session.id === one.id);
    if (row === undefined || row.dead !== undefined) continue;
    for (const file of row.work?.files ?? []) named.push({ root: one.root, file });
  }
  const merged = filesNamed(named);
  const filesKey = merged.key;
  const files = useMemo(() => merged.files, [filesKey]);

  const repos = useMemo<RepoIn[]>(() => {
    const roots = [...new Set([...all.map((one) => one.root), ...mains.map((one) => one.root), ...slots.map((one) => rootOfManifest(one.manifest_id) ?? one.manifest_id)])].sort();
    return roots.map((root) => ({
      root,
      name: ship.repos.find((one) => one.root === root)?.name ?? all.find((one) => one.root === root)?.repo ?? root,
      pads: slots
        .filter((one) => (rootOfManifest(one.manifest_id) ?? one.manifest_id) === root)
        .sort((a, b) => a.slot - b.slot)
        .map(slotKey),
      goal: mains.some((one) => one.root === root),
      areas: areasOf(root),
      ...(files.get(root) === undefined ? {} : { files: files.get(root)! }),
    }));
  }, [all, mains, ship.repos, slots, repositories, areasOf, files]);

  const bays = useBays(slots, events, replay?.at, clock);
  const pads = useMemo(() => {
    const out = new Map<string, PadIn>();
    for (const { view } of bays) {
      const who = view.holder ?? view.last;
      const name = who === undefined ? undefined : (who.title ?? who.id);
      out.set(view.key, {
        key: view.key,
        slot: view.slot.slot,
        bay: view.state === "leased" ? "held" : view.state,
        word: view.word,
        repo: repoNameOf(view.slot.manifest_id),
        Glyph: view.Glyph,
        ...(name === undefined ? {} : { holder: name }),
        ...(view.holder === undefined && who !== undefined ? { holderWas: true } : {}),
        ...(view.branch === undefined ? {} : { branch: view.branch }),
      });
    }
    return out;
  }, [bays, repositories]);

  const goals = useMemo(() => new Map<string, GoalIn>(mains.map((main) => [main.root, { root: main.root, red: main.state === "red", said: main.state === "red" ? `Main is red in ${main.repo}` : `Main is green in ${main.repo}` }])), [mains]);
  const prs = useMemo<PrIn[]>(() => shipPrs.map((pr) => ({ key: pr.key, number: pr.number, stage: pr.stage, root: pr.root, ...(pr.owner === undefined ? {} : { owner: `${pr.owner.kind}:${pr.owner.id}` }) })), [shipPrs]);
  // Of the events, only those that happen in the shown repository play on its map.
  const here = useMemo(() => {
    if (only === undefined) return () => true;
    const keys = new Set(all.map((one) => one.key));
    const numbers = new Set(shipPrs.map((one) => one.number));
    return (play: Play) => (play.key !== undefined ? keys.has(play.key) : play.number !== undefined ? numbers.has(play.number) : play.root === undefined || play.root === only);
  }, [only, all, shipPrs]);
  const candidates = useMemo(() => playsOf(events, clock, ownerOf, rootOfManifest), [events, clock, ship.prs, repositories]);
  const keptTrails = (trails: ReadonlyMap<string, number>) => (only === undefined ? trails : new Map([...trails].filter(([key]) => all.some((one) => one.key === key))));
  const trails = useMemo(() => keptTrails(trailsOf(events, clock, ownerOf)), [events, clock, ship.prs, only, all]);
  // Arrivals are read off every repository's drones, so showing another repository does not stand its drones as newly arrived.
  const everyone = useMemo(() => ship.subjects.filter((one) => one.kind !== "pr").map((one) => one.key), [ship.subjects]);
  const everyLanded = useMemo(() => new Map(ship.prs.filter((pr) => pr.stage === "landed").map((pr) => [pr.number, pr.root] as const)), [ship.prs]);
  const arrived = usePlays(candidates, everyone, everyLanded);
  const plays = useMemo(() => arrived.filter(here), [arrived, here]);

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
  // What each Session has built, as Fleet carries it now. A replay stands the field as it was, and a record of now is not that: it draws none.
  const built = useBuilds(clock);
  const builds = useMemo(() => (replay === undefined ? [...built.values()].filter((one) => only === undefined || all.some((drone) => drone.key === one.key)) : []), [built, replay === undefined, only, all]);
  // The services Sessions have called, and the drones calling one now. Calls of Sessions in another repository are left off a map cut to one.
  const outposts = useMemo(
    () => outpostsOf(before, replay?.at ?? clock, (event) => only === undefined || (rootOfManifest(event.manifest_id) ?? event.manifest_id) === only),
    [before, replay?.at, clock, only, repositories],
  );
  const calling = useMemo(() => withCalling(flying, before, replay?.at ?? clock, ownerOf), [flying, before, replay?.at, clock]);
  const shaped = useMemo(() => calling.map((one) => {
    const build = replay === undefined ? built.get(one.key) : undefined;
    return build === undefined ? one : shapedBy(one, build);
  }), [calling, built, replay === undefined]);
  const crates = field?.prs ?? prs;
  const flightTrails = useMemo(() => (replay === undefined ? trails : keptTrails(trailsOf(before, replay.at, ownerOf))), [trails, replay === undefined, passed, only, all]);
  // The event's own effect, once, where it is picked.
  const focusPlays = useMemo(() => {
    const play = picked === undefined ? undefined : playOfEvent(picked, `focus:${picked.id}`, ownerOf, rootOfManifest);
    return play === undefined || !here(play) ? [] : [play];
  }, [focus, here]);
  const drawn = replay === undefined ? [...plays, ...focusPlays] : replay.plays.filter(here);
  const at = replay?.at ?? clock;

  // What each drone's callout says besides the drone: its state as the list draws it, the call it holds, its last event, the hour of events as bars.
  const facts = useMemo(() => {
    const out = new Map<string, DroneFacts>();
    const keyOf = (event: TimelineEvent) => droneOfEvent(event, ownerOf);
    for (const drone of flying) {
      const subject = ship.subjects.find((one) => one.key === drone.key);
      if (subject === undefined) continue;
      const build = replay === undefined ? built.get(drone.key) : undefined;
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
        ...(build === undefined ? {} : { build: buildTipOf(build) }),
      });
    }
    return out;
  }, [flying, built, replay === undefined, ship.subjects, ship.calls, before, at, answered, pads, ship.prs]);

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
        keyOf: (kind, id) => (kind === "session" ? `session:${id}` : undefined),
        rootOf: (key) => all.find((one) => one.key === key)?.root,
      }),
    [written, all],
  );

  return { calls, repos, pads, goals, crates, drones: shaped, outposts, builds, flightTrails, drawn, facts, clashes, focusKey, replay, ordered, picked, ownerOf, rootOfManifest, events };
}
