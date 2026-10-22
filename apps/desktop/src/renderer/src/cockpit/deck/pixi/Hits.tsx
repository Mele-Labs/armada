// What a person can press, hover or tab to on the canvas map. The canvas draws and takes no pointer; over it stands this layer, an invisible
// element for each drone, pad, flag, crate and piece of a ring, put where its picture is by `spots.ts` each time the view moves. Each
// carries a role and a label, its keys, its press and its card, so the keyboard, a screen reader, the cards and the owner's notes (which
// read these components' names and these elements) find the things the canvas draws. The
// plots, files and bases that are only hovered are asked of the pointer instead (`plates.ts`), since there are as many as the repository
// has files.

import { createContext, memo, useContext, useEffect, useLayoutEffect, useRef, type MouseEvent as ReactMouseEvent, type MutableRefObject } from "react";
import { ShieldCheck } from "lucide-react";
import { clashSaid, useClashesOf, useOpenClash } from "@armada/components";

import { FRESH_LAND_MS, arcTip, bandTip, pieceTip } from "../builds";
import { keyed, onPlane } from "../bf-parts";
import { arcsOf, bandOf, roadOf, type BuildIn } from "../build";
import type { CallTone } from "../field";
import { alongOf, type P3, type Point } from "../layout";
import { clashLines, clashTip, droneTip, goalSaid, goalTip, outpostTip, padTip, payloadSaid, payloadTip, type DroneFacts, type GoalIn, type PadIn, type Tip } from "../tip";
import { plateAt } from "./plates";
import type { View } from "./project";
import type { ArcSpec, Crate, DroneSpec, OutpostSpec, PixiSceneIn, Placed, WorkerSpec } from "./scene";
import { arcSpot, bandSpot, callSpot, clashSpot, crateSpot, droneSpot, goalSpot, lineSpot, outpostSpot, padSpot, pieceSpot, quoteSpot, workerSpot, type Spot } from "./spots";
import type { Stage } from "./stage";

/** Raised by a thing that is hovered or focused: what to say, what it hangs from, where the pointer was if it is too large to have a middle, and what it lights. */
export type ShowTip = (tip: Tip | undefined, target?: Element, at?: { x: number; y: number }, lit?: { hot?: string; tile?: string; part?: string }) => void;

/** What a press does, and where a card goes: the page's, read when it is wanted so the layer need not be drawn again for a new one. */
export type HitActs = {
  select: (key: string) => void;
  pad: (key: string) => void;
  goal: (root: string) => void;
  pr: (key: string) => void;
  tip: ShowTip;
};

// ---- Putting them where the picture is ------------------------------------------------------------------

class Placer {
  view: View | undefined;
  private readonly spots = new Map<HTMLElement, (view: View) => Spot | undefined>();

  add(el: HTMLElement, spot: (view: View) => Spot | undefined): void {
    this.spots.set(el, spot);
    this.put(el);
  }

  remove(el: HTMLElement): void {
    this.spots.delete(el);
  }

  run(view: View): void {
    this.view = view;
    for (const el of this.spots.keys()) this.put(el);
  }

  put(el: HTMLElement): void {
    const spot = this.view === undefined || this.view.width === 0 ? undefined : this.spots.get(el)?.(this.view);
    if (spot === undefined) {
      el.style.display = "none";
      return;
    }
    el.style.display = "";
    el.style.transform = `translate(${spot.x.toFixed(1)}px, ${spot.y.toFixed(1)}px)`;
    el.style.width = `${spot.w.toFixed(1)}px`;
    el.style.height = `${spot.h.toFixed(1)}px`;
    el.style.clipPath = spot.shape === undefined ? "" : `path(evenodd, "${spot.shape}")`;
  }
}

const PlacerContext = createContext<Placer | undefined>(undefined);

/** An element that stands over the canvas where `spot` says. It is put there again whenever its component is drawn, and by the stage as the view moves. */
function usePlaced<T extends HTMLElement>(spot: (view: View) => Spot | undefined) {
  const placer = useContext(PlacerContext)!;
  const el = useRef<T>(null);
  const latest = useRef(spot);
  latest.current = spot;
  useLayoutEffect(() => {
    const node = el.current!;
    placer.add(node, (view) => latest.current(view));
    return () => placer.remove(node);
  }, [placer]);
  useLayoutEffect(() => placer.put(el.current!));
  return el;
}

/** The handlers a hovered or focused thing shares: it says its card while the pointer or focus is on it. */
const speaking = (say: (target: Element) => void, leave: () => void) => ({
  onMouseEnter: (event: ReactMouseEvent) => say(event.currentTarget),
  onMouseLeave: leave,
  onFocus: (event: { currentTarget: Element }) => say(event.currentTarget),
  onBlur: leave,
});

// ---- The things ----------------------------------------------------------------------------------------

type DroneProps = { spec: DroneSpec; facts: DroneFacts | undefined; big: number; acts: MutableRefObject<HitActs> };

/** A drone, and the marks over it that can be pressed: the call it holds and each write clash it is in. */
function Drone({ spec, facts, big, acts }: DroneProps) {
  const d = spec.drone;
  const ref = usePlaced<HTMLDivElement>((view) => droneSpot(view, spec, big));
  const pick = () => acts.current.select(d.key);
  const say = (target: Element) => acts.current.tip(droneTip(d, facts, ShieldCheck), target, undefined, { hot: d.key });
  const leave = () => acts.current.tip(undefined);
  return (
    <>
      <div
        ref={ref}
        className="bf-hits__item"
        role="option"
        tabIndex={0}
        aria-selected={spec.selected}
        aria-label={`${d.title}, ${d.job === undefined ? "Job" : `subagent of ${d.job}`}, ${d.said}`}
        data-hit="drone"
        data-key={d.key}
        data-label={d.title}
        data-kind={d.kind}
        data-hue={d.hue}
        data-doing={d.doing}
        data-selected={spec.selected || undefined}
        data-away={spec.away || undefined}
        data-hot={spec.hot || undefined}
        data-focus={spec.focused || undefined}
        data-dim={spec.dim || undefined}
        onClick={pick}
        onKeyDown={(event) => keyed(event, pick)}
        {...speaking(say, leave)}
      />
      {spec.call === undefined ? null : <Call spec={spec} tone={spec.call} acts={acts} big={big} say={say} />}
      <Clashes spec={spec} acts={acts} big={big} />
    </>
  );
}

/** The badge of a call a drone holds: pressing it opens the drone, as pressing the drone does. */
function Call({ spec, tone, acts, big, say }: { spec: DroneSpec; tone: CallTone; acts: MutableRefObject<HitActs>; big: number; say: (target: Element) => void }) {
  const ref = usePlaced<HTMLDivElement>((view) => callSpot(view, spec, big));
  const pick = () => acts.current.select(spec.drone.key);
  return (
    <div
      ref={ref}
      className="bf-hits__item"
      role="button"
      tabIndex={spec.leaving ? -1 : 0}
      aria-label={`${spec.drone.title} needs you`}
      data-hit="call"
      data-tone={tone}
      data-key={spec.drone.key}
      onClick={pick}
      onKeyDown={(event) => keyed(event, pick)}
      {...speaking(say, () => acts.current.tip(undefined))}
    />
  );
}

/** The marks of the write clashes a drone is in. Fleet's are read one drone at a time, by a hook. */
function Clashes({ spec, acts, big }: { spec: DroneSpec; acts: MutableRefObject<HitActs>; big: number }) {
  const d = spec.drone;
  const clashes = useClashesOf(d.kind, d.key.slice(d.kind.length + 1));
  return (
    <>
      {clashes.map((clash, i) => (
        <ClashMark key={`${clash.with.kind}:${clash.with.id}`} spec={spec} i={i} big={big} acts={acts} clash={clash} />
      ))}
    </>
  );
}

function ClashMark({ spec, i, big, acts, clash }: { spec: DroneSpec; i: number; big: number; acts: MutableRefObject<HitActs>; clash: ReturnType<typeof useClashesOf>[number] }) {
  const ref = usePlaced<HTMLDivElement>((view) => clashSpot(view, spec, i, big));
  const openClash = useOpenClash();
  const opens = clash.with.kind !== "main";
  return (
    <div
      ref={ref}
      className="bf-hits__item"
      role={opens ? "button" : "img"}
      tabIndex={opens ? 0 : -1}
      aria-label={clashSaid(clash)}
      data-hit="clash"
      data-key={spec.drone.key}
      data-opens={opens || undefined}
      onClick={(event) => {
        event.stopPropagation();
        if (opens) openClash(clash.with);
      }}
      onKeyDown={(event) => opens && keyed(event, () => openClash(clash.with))}
      {...speaking((target) => acts.current.tip(clashTip(clash), target), () => acts.current.tip(undefined))}
    />
  );
}

/** A bay's landing pad. */
function Pad({ pad, at, acts }: { pad: PadIn; at: Point; acts: MutableRefObject<HitActs> }) {
  const ref = usePlaced<HTMLDivElement>((view) => padSpot(view, onPlane(at)));
  const pick = () => acts.current.pad(pad.key);
  return (
    <div
      ref={ref}
      className="bf-hits__item"
      role="option"
      tabIndex={0}
      aria-selected={false}
      aria-label={`slot-${pad.slot}, ${pad.repo}, ${pad.word}`}
      data-hit="pad"
      data-bay={pad.bay}
      onClick={pick}
      onKeyDown={(event) => keyed(event, pick)}
      {...speaking((target) => acts.current.tip(padTip(pad), target, undefined, { part: `pad:${pad.key}` }), () => acts.current.tip(undefined))}
    />
  );
}

/** Main: the flag at a repository's objective. */
function Goal({ root, repo, goal, at, acts }: { root: string; repo: string; goal: GoalIn; at: Point; acts: MutableRefObject<HitActs> }) {
  const ref = usePlaced<HTMLDivElement>((view) => goalSpot(view, at));
  const pick = () => acts.current.goal(root);
  return (
    <div
      ref={ref}
      className="bf-hits__item"
      role="option"
      tabIndex={0}
      aria-selected={false}
      aria-label={goalSaid(goal, repo)}
      data-hit="goal"
      data-red={goal.red || undefined}
      onClick={pick}
      onKeyDown={(event) => keyed(event, pick)}
      {...speaking((target) => acts.current.tip(goalTip(goal, repo), target, undefined, { part: `goal:${root}` }), () => acts.current.tip(undefined))}
    />
  );
}

/** A service outpost at the far edge of the field. It is a place to stop and a card, and a press does nothing. */
function Outpost({ outpost, acts }: { outpost: OutpostSpec; acts: MutableRefObject<HitActs> }) {
  const ref = usePlaced<HTMLDivElement>((view) => outpostSpot(view, outpost.at));
  return (
    <div
      ref={ref}
      className="bf-hits__item"
      role="option"
      tabIndex={0}
      aria-selected={false}
      aria-label={outpost.name}
      data-hit="outpost"
      data-service={outpost.service}
      data-lit={outpost.lit || undefined}
      {...speaking((target) => acts.current.tip(outpostTip(outpost), target, undefined, { part: `outpost:${outpost.service}` }), () => acts.current.tip(undefined))}
    />
  );
}

/** A pull request's crate, on its route to the objective. */
function Payload({ crate, acts }: { crate: Crate; acts: MutableRefObject<HitActs> }) {
  const ref = usePlaced<HTMLDivElement>((view) => crateSpot(view, crate.at));
  const pick = () => acts.current.pr(crate.pr.key);
  return (
    <div
      ref={ref}
      className="bf-hits__item"
      role="option"
      tabIndex={0}
      aria-selected={false}
      aria-label={payloadSaid(crate.pr)}
      data-hit="payload"
      data-stage={crate.pr.stage}
      onClick={pick}
      onKeyDown={(event) => keyed(event, pick)}
      {...speaking((target) => acts.current.tip(payloadTip(crate.pr), target, undefined, { part: `crate:${crate.pr.key}` }), () => acts.current.tip(undefined))}
    />
  );
}

type Said = { tip: Tip; said: string };

/** A piece of a ring, an arc of its road or its band: a card, and a place to stop. */
function RingPart({ kind, said, spot, acts }: { kind: "task" | "road" | "phase"; said: Said; spot: (view: View) => Spot; acts: MutableRefObject<HitActs> }) {
  const ref = usePlaced<HTMLDivElement>(spot);
  return (
    <div
      ref={ref}
      className="bf-hits__item"
      role="option"
      tabIndex={0}
      aria-selected={false}
      aria-label={said.said}
      data-hit={kind}
      {...speaking((target) => acts.current.tip(said.tip, target), () => acts.current.tip(undefined))}
    />
  );
}

/** What a ring has to be hovered over: a piece for each task, or the band where there is no list, and an arc for each stop of the road. */
function Ring({ one, heights, big, acts }: { one: Placed; heights: ReadonlyMap<string, number>; big: number; acts: MutableRefObject<HitActs> }) {
  const build: BuildIn = one.build;
  // A landed ring is cleared a quarter hour on.
  if (build.landedAt !== undefined && build.now - build.landedAt > FRESH_LAND_MS) return null;
  const road = roadOf(build);
  const m = Math.max(arcsOf(build).length, 1);
  const n = Math.max(build.tasks.length, 1);
  const band = build.tasks.length === 0 && bandOf(build) !== undefined ? bandTip(build) : undefined;
  return (
    <>
      {band === undefined ? null : <RingPart kind="phase" said={band} spot={(view) => bandSpot(view, one, heights, big)} acts={acts} />}
      {build.tasks.map((_, k) => {
        const said = pieceTip(build, k);
        return said === undefined ? null : <RingPart key={`${build.key}:task:${k}`} kind="task" said={said} spot={(view) => pieceSpot(view, one, heights, k, n, big)} acts={acts} />;
      })}
      {road === undefined
        ? null
        : road.map((_, k) => {
            const said = arcTip(build, k);
            return said === undefined ? null : <RingPart key={`${build.key}:road:${k}`} kind="road" said={said} spot={(view) => arcSpot(view, one, heights, k, m, big)} acts={acts} />;
          })}
    </>
  );
}

/** A subagent hovering at its place on a ring. */
function Worker({ worker, stage, acts }: { worker: WorkerSpec; stage: Stage; acts: MutableRefObject<HitActs> }) {
  const ref = usePlaced<HTMLDivElement>((view) => workerSpot(view, worker, stage.floorOf(worker.tile)));
  const tip: Tip = { label: "Subagent", title: worker.detail, hue: "quiet" };
  return (
    <div
      ref={ref}
      className="bf-hits__item"
      role="option"
      tabIndex={0}
      aria-selected={false}
      aria-label={`Subagent, ${worker.detail}`}
      data-hit="worker"
      {...speaking((target) => acts.current.tip(tip, target), () => acts.current.tip(undefined))}
    />
  );
}

/** The line between two drones changing one area: hovered for its card, and not a place to stop. */
function ClashLine({ line, tip, acts }: { line: readonly P3[]; tip: Tip; acts: MutableRefObject<HitActs> }) {
  const ref = usePlaced<HTMLDivElement>((view) => lineSpot(view, line, 7));
  return (
    <div
      ref={ref}
      className="bf-hits__item"
      aria-hidden="true"
      data-hit="clash-line"
      onMouseEnter={(event) => acts.current.tip(tip, event.currentTarget, { x: event.clientX, y: event.clientY })}
      onMouseLeave={() => acts.current.tip(undefined)}
    />
  );
}

/**
 * The arc between this Job's drone and another Job's while one messages the other, and the quote at its middle: a press on either opens
 * the other Job, as a press on its drone does. The quote is the place to stop; the line takes the pointer only.
 */
function Message({ arc, acts }: { arc: ArcSpec; acts: MutableRefObject<HitActs> }) {
  const line = usePlaced<HTMLDivElement>((view) => lineSpot(view, Array.from({ length: 19 }, (_, i) => alongOf(arc.from, arc.to, arc.lift, i / 18)), 7));
  const quote = usePlaced<HTMLDivElement>((view) => quoteSpot(view, arc));
  const pick = () => acts.current.select(arc.other);
  const said = `${arc.sent ? "Message to" : "Message from"} ${arc.title}: ${arc.quote}`;
  const tip: Tip = { label: arc.sent ? "Message to" : "Message from", title: arc.title, hue: "quiet", lines: [arc.quote] };
  return (
    <>
      <div ref={line} className="bf-hits__item" aria-hidden="true" data-hit="message-line" data-key={arc.other} onClick={pick} />
      <div
        ref={quote}
        className="bf-hits__item"
        role="button"
        tabIndex={0}
        aria-label={said}
        data-hit="message"
        data-key={arc.other}
        data-from={arc.sent ? "this" : "other"}
        onClick={pick}
        onKeyDown={(event) => keyed(event, pick)}
        {...speaking((target) => acts.current.tip(tip, target), () => acts.current.tip(undefined))}
      />
    </>
  );
}

// ---- The layer ---------------------------------------------------------------------------------------

/**
 * The plots, files and bases are hovered and never pressed, and a repository has as many as it has directories and files; one listener
 * on the pane asks `plateAt` where the pointer is, and says the card of what is there. What a card hangs from is a stand-in element for the
 * plate, kept so that the card knows it is the same plate while the pointer crosses it.
 */
function usePlates(layer: MutableRefObject<HTMLDivElement | null>, stage: Stage, scene: PixiSceneIn, acts: MutableRefObject<HitActs>): void {
  const latest = useRef(scene);
  latest.current = scene;
  useEffect(() => {
    const host = layer.current?.parentElement;
    if (host === null || host === undefined) return;
    const stand = new Map<string, HTMLElement>();
    let current: string | undefined;
    let frame = 0;
    let last: PointerEvent | undefined;
    const look = () => {
      frame = 0;
      if (last === undefined) return;
      const box = host.getBoundingClientRect();
      const plate = plateAt(latest.current, stage.shown, last.clientX - box.left, last.clientY - box.top);
      if (plate?.key === current) return;
      current = plate?.key;
      if (plate === undefined) return acts.current.tip(undefined);
      let target = stand.get(plate.key);
      if (target === undefined) stand.set(plate.key, (target = document.createElement("div")));
      acts.current.tip(plate.tip, target, { x: last.clientX, y: last.clientY }, plate.tile === undefined ? undefined : { tile: plate.tile });
    };
    const move = (event: PointerEvent) => {
      // A thing with a card of its own says it; a hand on the map is moving it.
      if (event.buttons !== 0 || (event.target as Element).closest("[data-hit]") !== null) {
        current = undefined;
        return;
      }
      last = event;
      if (frame === 0) frame = window.requestAnimationFrame(look);
    };
    const leave = () => {
      last = undefined;
      if (current === undefined) return;
      current = undefined;
      acts.current.tip(undefined);
    };
    host.addEventListener("pointermove", move);
    host.addEventListener("pointerleave", leave);
    return () => {
      window.cancelAnimationFrame(frame);
      host.removeEventListener("pointermove", move);
      host.removeEventListener("pointerleave", leave);
    };
  }, [stage]);
}

export type HitLayerProps = {
  stage: Stage;
  scene: PixiSceneIn;
  facts: ReadonlyMap<string, DroneFacts>;
  acts: MutableRefObject<HitActs>;
};

/** The layer over the canvas. In the order Tab goes round: pads, flags, outposts, crates, rings, the messages between Jobs, then the drones from the far side of the field to the near. */
function HitLayer({ stage, scene, facts, acts }: HitLayerProps) {
  const layer = useRef<HTMLDivElement>(null);
  const placer = useRef(new Placer()).current;
  useLayoutEffect(() => {
    placer.run(stage.shown);
    return stage.onView((view) => placer.run(view));
  }, [stage, placer]);
  usePlates(layer, stage, scene, acts);
  const drones = [...scene.drones].sort((a, b) => a.at.y - b.at.y);
  return (
    <PlacerContext.Provider value={placer}>
      <div ref={layer} className="bf-hits" role="listbox" aria-label="Map">
        {scene.layout.territories.flatMap((one) =>
          one.pads.map((pad) => {
            const info = scene.pads.get(pad.key);
            return info === undefined ? null : <Pad key={pad.key} pad={info} at={pad} acts={acts} />;
          }),
        )}
        {scene.layout.territories.map((one) => {
          const goal = scene.goals.get(one.root);
          return goal === undefined || one.goal === undefined ? null : <Goal key={one.root} root={one.root} repo={one.name} goal={goal} at={one.goal} acts={acts} />;
        })}
        {scene.outposts.map((one) => (
          <Outpost key={one.service} outpost={one} acts={acts} />
        ))}
        {scene.crates.map((crate) => (
          <Payload key={crate.pr.key} crate={crate} acts={acts} />
        ))}
        {scene.builds.map((one) => (
          <Ring key={one.build.key} one={one} heights={scene.heights} big={scene.big} acts={acts} />
        ))}
        {scene.workers.map((one) => (
          <Worker key={one.key} worker={one} stage={stage} acts={acts} />
        ))}
        {scene.clashLines.map((one) => {
          const clash = scene.layout.clashes.find((c) => `${c.root}:${c.path}` === one.key);
          return clash === undefined ? null : <ClashLine key={one.key} line={Array.from({ length: 19 }, (_, i) => alongOf(one.from, one.to, 22, i / 18))} tip={{ label: "Write clash", title: clash.path, hue: "ask", lines: clashLines(clash) }} acts={acts} />;
        })}
        {scene.arcs.map((one) => (
          <Message key={one.key} arc={one} acts={acts} />
        ))}
        {drones.map((spec) => (
          <Drone key={spec.drone.key} spec={spec} facts={facts.get(spec.drone.key)} big={scene.big} acts={acts} />
        ))}
      </div>
    </PlacerContext.Provider>
  );
}

export const Hits = memo(HitLayer);
