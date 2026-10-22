// A Job's drones over the whole repository it works in: the Cockpit's battlefield, with every area Fleet names as the floor, so its shape
// never changes as the Job moves. The plots the Job has touched (its plan's scope and its drones' paths) are outlined in the accent. Its own
// drone and a small one for each subagent fly to the file each is on; the drones of every other live Job in the repository are drawn faded,
// with no beam, label or action, and pressing one opens that Job. Each file the Job wrote or read glows from the times of Fleet's bursts,
// fading over ten minutes for a write and five for a read (`afterglow.ts`). Nothing here is drawn from what an agent said. The plan is one bar along the foot, and under it a
// slider scrubs back through the Job's reads and writes: the glow, the drone and a numbered route stand as they were, and the right end is
// live. A drone not coding leaves the floor: to the pad at the near edge while a check runs or a walk is open, risen over its file while it
// thinks, beaming to an outpost while it calls (`field-now.ts`). A strip over the map names each drone and what it is doing; a press frames
// the camera on it. A file a drone of the Job is writing lifts as a slab with its minimap, and a card at the top right shows its newest
// hunk; the card follows the newest writer, or the drone a strip press pinned (`field-lens.ts`). While the Job messages another Job, or
// is messaged by one, an arc runs between their drones with packets travelling from the sender, and the other drone un-fades
// (`session.messaged`, `field-now.ts`). `docs/concepts/cockpit.md`, *A Job open*. The control in the band, or `deck_field`'s key (`DeckHost.tsx`), opens the Field large.

import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { Maximize2 } from "lucide-react";
import { Button, Tooltip, agoOf, boundKeyFor, useKeyBindings } from "@armada/components";
import type { RepositorySummary, TimelineEvent } from "@armada/protocol";
import type { Session, WorkDrone } from "@armada/screens/src/draft/sessions";
import { jobIdOf } from "@armada/shell";

import { Battlefield } from "../cockpit/deck/Battlefield";
import { useAreas } from "../cockpit/deck/areas-source";
import { areaOf } from "../cockpit/deck/areas";
import { burstsOf, droneAt, glowAt, routeAt, type Burst } from "../cockpit/deck/afterglow";
import { useClock } from "../cockpit/deck/clock";
import { outpostsOf } from "../cockpit/deck/outposts";
import { messagesOf } from "../cockpit/deck/off-floor";
import type { DroneIn, FileIn, RepoIn } from "../cockpit/deck/layout";
import { neighbours, windowOf } from "../cockpit/deck/replay";
import { ReplayMark, ReplayTrack } from "../cockpit/deck/ReplayBar";
import { countsOf, type Slab } from "../cockpit/deck/lens";
import { same } from "../cockpit/deck/pixi/same";
import { select } from "../cockpit/deck/store";
import { useFleetEvents } from "../cockpit/fleet-events";
import { useSessions } from "../sessions-draft";
import { isWorking, touchedPlots, workingDrones } from "./field";
import { PlanBar } from "./FieldPlan";
import { FieldStrip } from "./FieldStrip";
import { FieldLensCard } from "./FieldLensCard";
import { useLens, useShown } from "./field-lens";
import { placedNow, stripOf, talkOf, useLiftsAndReturns, useMainNow } from "./field-now";
import { groupsOf, plotsOf } from "./field-plan";
import { toggleLayer, useLayer } from "./layers";
import { SvPanel, type PanelState, type Place } from "./SvPanel";
import "../cockpit/deck/deck.css";

const NONE = new Map();
const RESTING: readonly WorkDrone[] = [{ id: "main", path: "", doing: "read" }];
const NO_LIST: never[] = [];
const nothing = () => undefined;

/** The glow and the slider's end move in steps of this: a glow takes minutes to fade, and a step that moved nothing draws nothing. */
const GLOW_STEP = 15_000;
/** The last part of the track that is the live end: let go there and the Field is live again. */
const LIVE_END = 0.02;

/** The same value as before where nothing in it changed, so a Board update that leaves the Field alone hands the map what it had. */
function useSteady<T>(value: T): T {
  const kept = useRef(value);
  if (!same(kept.current, value)) kept.current = value;
  return kept.current;
}

/** The checkouts a Job's paths may be spelled under: the repository, each slot it held, and where it started. */
function rootsOf(session: Session, root: string): string[] {
  const slots = new Set<number>();
  for (const one of session.attachments) if (one.kind === "slot" || one.kind === "branch") slots.add(one.slot);
  for (const bay of session.trail?.bays ?? []) slots.add(bay.slot);
  const bare = root.replace(/\/+$/, "");
  return [...[...slots].map((slot) => `${bare}/.armada/slots/slot-${slot}`), bare, ...(session.cwd === undefined ? [] : [session.cwd])].filter((one) => one !== "");
}

/** A drone of the Field: the Job's own live, the Job's own as the slider left it, or another Job's. Only the first shows what it is doing. */
function flier(session: Session, one: WorkDrone, root: string, as: "live" | "then" | "ghost"): DroneIn {
  const own = `session:${session.id}`;
  const ghost = as === "ghost";
  const working = as === "live" && isWorking(session, one);
  return {
    key: one.agent === undefined ? own : `${own}/${one.id}`,
    kind: "session",
    title: one.agent ?? session.title ?? session.id,
    root,
    said: one.task ?? "Working",
    hue: "calm",
    spin: working,
    hover: true,
    ping: false,
    stuck: false,
    flare: false,
    working,
    // Another Job's drone shows where it is and nothing of what it is doing.
    ...(one.path === "" || as !== "live" ? {} : { doing: one.doing, via: one.path }),
    ...(one.agent === undefined ? {} : { parent: own }),
    ...(one.file === undefined ? {} : { file: one.file }),
    touches: one.path === "" ? [] : [{ path: one.file ?? one.path, w: 4 }],
    shown: true,
    ...(ghost ? { ghost: true as const } : {}),
  };
}

const TOOL_KINDS: readonly TimelineEvent["kind"][] = ["session.read", "session.edited", "session.ran", "session.called"];

/**
 * The Job's tool entries (reads, writes, commands and calls), its reads and writes alone and as bursts, and the slider's window of
 * them: the last two hours, as a replay's.
 */
function useJobEvents(session: Session, roots: readonly string[], clock: number) {
  const events = useFleetEvents();
  const rootsKey = roots.join("|");
  const job = useMemo(() => events.filter((one) => one.subject.kind === "session" && one.subject.id === session.id && TOOL_KINDS.includes(one.kind)), [events, session.id]);
  // The messages this Job sent and those sent to it: Fleet writes the sender as the subject and the receiver as `to`.
  const talk = useMemo(() => events.filter((one) => one.kind === "session.messaged" && ((one.subject.kind === "session" && one.subject.id === session.id) || one.to === session.id)), [events, session.id]);
  const mine = useMemo(() => job.filter((one) => one.kind === "session.read" || one.kind === "session.edited"), [job]);
  const bursts = useMemo<Burst[]>(() => burstsOf(mine, session.id, roots), [mine, session.id, rootsKey]);
  const held = useMemo(() => windowOf(mine, clock), [mine, clock]);
  return { job, mine, bursts, held, talk };
}

/** Everything the Field draws for a Job, read once: the floor, the drones, the glow, the route, and the plan as bar and as plots. */
function useJobField(session: Session, repositories: readonly RepositorySummary[], at: number | undefined) {
  const source = useAreas();
  const sessions = useSessions();
  const clock = useClock(GLOW_STEP);
  const work = session.work;
  // A Job in a folder with no Manifest is found by where it started.
  const repository = repositories.find((one) => jobIdOf(one) === session.manifestId) ?? (session.manifestId === undefined ? repositories.find((one) => one.root === session.cwd) : undefined);
  const root = repository?.root ?? session.manifestId ?? "";
  const name = repository?.manifest?.repository ?? repository?.root.split("/").pop() ?? root;
  const roots = useSteady(rootsOf(session, root));
  const { job, mine, bursts, held, talk } = useJobEvents(session, roots, clock);
  const now = at ?? clock;

  // Before any drone has work the floor has the Job's own drone alone, at rest. Only the subagents at work are drawn. Scrubbed back, the
  // Job's drone stands at the file of its last read or write before then, and its subagents, which no burst places, are left off.
  const working = useMemo(() => workingDrones(session, work?.field), [session, work?.field]);
  // Live, the Job's drone stands at the file it last read or wrote, where Fleet's bursts name one, else at the file its touches last name.
  const lastFile = bursts.at(-1)?.paths[0];
  const live = useSteady<readonly WorkDrone[]>(
    (working === undefined || working.length === 0 ? RESTING : working).map((one) => (one.agent !== undefined || lastFile === undefined ? one : { ...one, path: areaOf(lastFile) ?? one.path, file: lastFile })),
  );
  const mainFile = live.find((one) => one.agent === undefined)?.file;
  const { now: mainState, lights, walk } = useMainNow(session, job, roots, clock, mainFile);
  const then = at === undefined ? undefined : droneAt(bursts, at);
  const drones = useSteady<readonly WorkDrone[]>(at === undefined ? live : then === undefined ? RESTING : [{ id: "main", path: areaOf(then.path) ?? "", doing: then.act, file: then.path }]);

  const glow = useSteady(useMemo(() => glowAt(bursts, now), [bursts, now]));
  const route = useSteady(useMemo(() => (at === undefined ? undefined : routeAt(bursts, at)), [bursts, at]));

  const areas = source(root);
  const scope = work?.plan?.scope;
  const paths = useSteady(new Set([...(scope ?? []), ...live.map((one) => one.path)].filter((one) => one !== "")));
  const touched = useSteady(useMemo(() => touchedPlots(areas, paths), [areas, paths]));

  // The files drawn as bars: those the Job names, then each one glowing or on the route, so the glow and the route have a bar to stand on.
  const named = work?.files;
  const files = useSteady(
    useMemo<FileIn[]>(() => {
      const out = new Map<string, FileIn>((named ?? []).map((one) => [one.path, one]));
      const blamed = lights.flatMap((one) => (one.blame === undefined ? [] : [one.blame]));
      for (const path of [...glow.keys(), ...(route ?? []).map((one) => one.path), ...drones.flatMap((one) => (one.file === undefined ? [] : [one.file])), ...blamed]) if (!out.has(path)) out.set(path, { path });
      return [...out.values()];
    }, [named, glow, route, drones, lights]),
  );
  const repos = useMemo<RepoIn[]>(() => [{ root, name, pads: [], goal: false, areas, pad: true, ...(files.length === 0 ? {} : { files }) }], [root, name, areas, files]);

  // Off the code floor where the Job's own drone is not coding: on the pad, risen over its file, or calling a service.
  const placed = useSteady(useMemo(() => drones.map((one) => {
    const drone = flier(session, one, root, at === undefined ? "live" : "then");
    return at === undefined && one.agent === undefined ? placedNow(drone, mainState) : drone;
  }), [drones, root, session, at === undefined, mainState]));
  const { drones: own, plays } = useLiftsAndReturns(placed, at === undefined);
  const outposts = useSteady(useMemo(() => outpostsOf(job, Math.max(clock, Date.now())), [job, clock]));
  // The drones of every other live Job in the repository, faded.
  const ghosts = useSteady(
    sessions
      .filter((one) => one.id !== session.id && one.helm !== true && one.dead === undefined && one.replacedBy === undefined && one.waits === undefined && (one.manifestId === undefined ? one.cwd === root : one.manifestId === session.manifestId))
      .flatMap((one) => (workingDrones(one, one.work?.field) ?? RESTING).map((drone) => flier(one, drone, root, "ghost"))),
  );
  // A message to or from another Job while fresh, read on the clock's step and not between: an arc to its drone, which comes up to full
  // strength, and a word on the strip. None while scrubbed back, which stands the Job as it was.
  const messages = useMemo(() => (at !== undefined ? [] : messagesOf(talk, session.id, Math.max(clock, Date.now()))), [talk, session.id, clock, at === undefined]);
  const talking = useMemo(() => talkOf(session.id, messages, ghosts, (id) => sessions.find((one) => one.id === id)?.title), [session.id, messages, ghosts, sessions]);
  const links = useSteady(talking.links);
  const others = useSteady(useMemo(() => {
    const linked = new Set(links.map((one) => one.other));
    return linked.size === 0 ? ghosts : ghosts.map((one) => (linked.has(one.key) ? { ...one, linked: true as const } : one));
  }, [ghosts, links]));
  const strip = useSteady(stripOf(session, mainState, work?.field, talking.word));
  const flying = useSteady(useMemo(() => [...others, ...own], [others, own]));
  const framing = useSteady(own.map((one) => one.key));

  const plan = work?.plan;
  const groups = useMemo(() => (plan === undefined ? [] : groupsOf(plan)), [plan]);
  const plots = areas.flatMap((area) => (area.kids.length === 0 ? [area.dir] : area.kids.map(([kid]) => `${area.dir}/${kid}`)));
  const planned = useMemo(() => (plan === undefined ? undefined : plotsOf(plan, plots)), [plan, plots.join("|")]);
  return { repos, flying, groups, planned, touched, glow, route, framing, mine, bursts, held, clock, lights: useSteady(lights), walk: at === undefined ? walk : undefined, plays, outposts, strip, own, job, roots, links };
}

/** The slider under the plan: from the Job's first read or write in the window to now, a tick for each. The right end is live. */
function FieldReplay({ events, from, end, at, onAt }: { events: readonly TimelineEvent[]; from: number; end: number; at: number | undefined; onAt: (at: number | undefined) => void }) {
  const span = Math.max(1, end - from);
  // A press or a drag lets go into live anywhere in the last stretch of the track; a key steps onto each read or write, the newest too.
  const settle = (ms: number) => onAt(ms >= end - span * LIVE_END ? undefined : Math.max(from, ms));
  const step = (event: KeyboardEvent<HTMLDivElement>) => {
    const here = at ?? end;
    const { before, after } = neighbours(events, here);
    const to = event.key === "ArrowLeft" ? (before ?? from) : event.key === "ArrowRight" ? (after ?? end) : event.key === "Home" ? from : event.key === "End" ? end : undefined;
    if (to === undefined) return;
    event.preventDefault();
    onAt(to >= end ? undefined : Math.max(from, to));
  };
  return (
    <div className="sv-fieldreplay" role="group" aria-label="Replay">
      <ReplayTrack events={events} from={from} end={end} at={at ?? end} label="When in the Job" valueText={at === undefined ? "Live" : agoOf(new Date(at).toISOString(), end)} onSeek={settle} onKey={step} />
    </div>
  );
}

/**
 * A press on another Job's drone opens that Job on the stage, as a press on its drone on the Cockpit map does (`MapPane.tsx`); a subagent's
 * drone opens the Job that sent it. This Job's own drones open nothing.
 */
function openOther(flying: readonly DroneIn[], key: string): void {
  const drone = flying.find((one) => one.key === key);
  if (drone?.ghost !== true) return;
  // A subagent's drone is keyed `session:<id>/<subagent>`.
  select({ kind: "session", key: drone.key.split("/")[0]! });
}

/** The map and, along its foot, the plan and the slider. The same in the panel and in the layer; `large` is the layer's, which names each group under its stretch. */
export function FieldMap({ session, repositories, large = false }: { session: Session; repositories: readonly RepositorySummary[]; large?: boolean }) {
  const [at, setAt] = useState<number>();
  // The drone a strip button pinned: the camera frames it.
  const [pinned, setPinned] = useState<string>();
  // The slider and the pin belong to the Job they were set on.
  useEffect(() => {
    setAt(undefined);
    setPinned(undefined);
  }, [session.id]);
  const pick = useCallback((key: string) => setPinned((was) => (was === key ? undefined : key)), []);
  const { repos, flying, groups, planned, touched, glow, route, framing, mine, bursts, held, clock, lights, walk, plays, outposts, strip, own, job, roots, links } = useJobField(session, repositories, at);
  // The lens: asked for and drawn only while the Field is live and shown.
  const box = useRef<HTMLDivElement>(null);
  const shown = useShown(box);
  const lens = useLens({ session: session.id, drones: own, pinned, events: job, roots, bursts, active: at === undefined && shown });
  const slabs = useMemo<Slab[]>(() => lens.slabs.map((one) => ({ path: one.path, card: one.card, rows: one.diff.rows, counts: countsOf(one.diff) })), [lens.slabs]);
  const [typing, setTyping] = useState(false);
  return (
    <div ref={box} className="sv-field sv-field--plan" data-large={large || undefined}>
      <FieldStrip entries={strip} pinned={pinned} onPick={pick} />
      <div className="sv-field__map">
        {at === undefined ? null : <ReplayMark />}
        <Battlefield
          repos={repos}
          drones={flying}
          pads={NONE}
          goals={NONE}
          prs={NO_LIST}
          trails={NONE}
          plays={plays}
          facts={NONE}
          calls={NONE}
          clashes={NO_LIST}
          selected={pinned}
          outposts={outposts}
          lights={lights}
          walk={walk}
          lens={slabs}
          lensCard={lens.card === undefined ? undefined : <FieldLensCard card={lens.card} large={large} onTyping={setTyping} />}
          typing={typing && lens.card !== undefined}
          links={links}
          onClear={() => setPinned(undefined)}
          own={large ? `${session.id}:large` : session.id}
          {...(planned === undefined ? {} : { planned })}
          touched={touched}
          glow={glow}
          route={route}
          framing={framing}
          onSelect={(key) => openOther(flying, key)}
          onPad={nothing}
          onGoal={nothing}
          onPr={nothing}
        />
      </div>
      {groups.length === 0 ? null : <PlanBar groups={groups} titled={large} />}
      {held === undefined ? null : <FieldReplay events={mine} from={held.from} end={Math.max(clock, Date.parse(held.events.at(-1)!.at))} at={at} onAt={setAt} />}
    </div>
  );
}

export function FieldPanel({ session, repositories, place }: { session: Session; repositories: readonly RepositorySummary[]; place: Place }) {
  const working = workingDrones(session, session.work?.field);
  const resting = working === undefined || working.length === 0;
  const expanded = useLayer(session.id) === "field";
  const id = session.id;
  useKeyBindings();
  const state: PanelState | undefined = resting ? undefined : "live";
  const expand = (
    <Tooltip label={expanded ? "Collapse" : "Expand"} shortcut={boundKeyFor("deck_field")}>
      <Button variant="ghost" size="sm" iconOnly aria-label={expanded ? "Collapse" : "Expand"} onClick={() => toggleLayer(id, "field")}>
        <Maximize2 size={16} strokeWidth={2} aria-hidden />
      </Button>
    </Tooltip>
  );
  return (
    <SvPanel label="Field" area="field" place={place} state={state} acts={expand}>
      {/* One canvas at a time: while the layer holds the Field large, the panel draws none. */}
      {expanded ? <div className="sv-field sv-field--plan" /> : <FieldMap session={session} repositories={repositories} />}
    </SvPanel>
  );
}
