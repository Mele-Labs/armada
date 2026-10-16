// The flight deck's map, a battlefield on a tilted plane in perspective. Each repository is a territory with a base at its near edge, its
// landing pads the bays, and its main the objective on the far edge, a flag. Each Job and Session is a drone on a beam over the plots it
// changes (or at its pad where nothing observed puts it elsewhere), a Job a large X quad and a Session a small plus quad. State is the
// drone and its frame, never words. Pull requests are crates on a lifted route to the flag. The activity log drives the scene
// (`plays.ts`). **Every fact on it is read off what Fleet recorded**; the field is handed already read (`useField.ts`).
//
// The 3D is CSS alone: a plane in `perspective`, tilted with rotateX and a little rotateZ, its parts placed with translate3d. Drones,
// labels and flags are billboards, counter-rotated so they stand upright. No loop runs in script. A Job's mission is the same plane cut
// down to its route (`scene`). Ported from proto/cockpit-flight-deck. #2301.

import { useEffect, useMemo, useRef, useState } from "react";
import { Crosshair, Megaphone, Minus, Plus, Scan, ShieldCheck } from "lucide-react";
import { Button, Tooltip, WorkflowStepCard } from "@armada/components";

import { Builds, aimedAt, placeBuilds } from "./Builds";
import { AskUpdate } from "./AskUpdate";
import { Callout, type Anchor } from "./Callout";
import { Doings } from "./Doings";
import { Drone, type OnTip } from "./Drone";
import { Lifted } from "./Lifted";
import { Pins, RouteLegs, tipOf } from "./Route";
import { Base, Goal, Hangar, Pad, Payload, Slab } from "./Scene";
import { onPlane, px, vars } from "./bf-parts";
import { LOCK_EYE, MISSION_EYE, READABLE, WORLD, ZOOM_STEP, edgeOf, held as limited, homeOf, limitsOf, lookingAt, marksOf, reachOf, type Cam } from "./camera";
import { loopsOf, type CallTone } from "./field";
import { frameOf } from "./frame";
import { COS, FOOT, MARGIN, ROUTE, SIN, liftOf } from "./geometry";
import { bodiesOf, labelsOf, plotNamed } from "./labels";
import { ALONG, type Play } from "./plays";
import { CAM, FLAT, alongOf, bigOf, layoutOf, type DroneIn, type P3, type Point, type PrIn, type RepoIn, type Size } from "./layout";
import { sceneOf, type SceneIn } from "./mission";
import type { BuildIn } from "./build";
import { aimAt, camera as kept } from "./store";
import { clashLines, droneTip, type DroneFacts, type GoalIn, type PadIn, type Tip } from "./tip";
import type { Clash } from "./touches";
import { useCamera } from "./useCamera";
import "./battlefield.css";

export type BattlefieldProps = {
  repos: readonly RepoIn[];
  drones: readonly DroneIn[];
  pads: ReadonlyMap<string, PadIn>;
  goals: ReadonlyMap<string, GoalIn>;
  prs: readonly PrIn[];
  trails: ReadonlyMap<string, number>;
  plays: readonly Play[];
  facts: ReadonlyMap<string, DroneFacts>;
  /** The drones that hold a call he has not opened, and whether it is owed to him or its Check is failing. */
  calls: ReadonlyMap<string, CallTone>;
  /** Where Fleet reports two holders writing one area. */
  clashes: readonly Clash[];
  /** The drone a picked event is about. */
  focus?: string | undefined;
  selected: string | undefined;
  /** The ring under each drone that is building one. None where Fleet reports none. */
  builds?: readonly BuildIn[] | undefined;
  /** The waypoint of a Job's route its structure stands at: the step that works the plan. */
  standAt?: string | undefined;
  /** A Job's mission, in place of the whole field: the same plane, cut down to the plots one Job touches and laid with its route. */
  scene?: Omit<SceneIn, "big" | "wide" | "clashes"> | undefined;
  onSelect: (key: string) => void;
  /** A press on the ground, on no drone: whatever was picked lets go. */
  onClear?: () => void;
  onPad: (key: string) => void;
  onGoal: (root: string) => void;
  onPr: (key: string) => void;
};

const NO_BUILDS: readonly BuildIn[] = [];

export function Battlefield({ repos, drones, pads, goals, prs, trails, plays, facts, calls, clashes, focus, selected, scene, builds = NO_BUILDS, standAt, onSelect, onClear, onPad, onGoal, onPr }: BattlefieldProps) {
  const host = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState<Size>({ width: 900, height: 560 });
  const [measured, setMeasured] = useState(false);
  useEffect(() => {
    const one = host.current;
    if (one === null) return;
    const read = () => {
      const box = one.getBoundingClientRect();
      if (box.width <= 0 || box.height <= 0) return;
      setMeasured(true);
      setSize((was) => (Math.round(box.width) === was.width && Math.round(box.height) === was.height ? was : { width: Math.round(box.width), height: Math.round(box.height) }));
    };
    read();
    const watch = new ResizeObserver(read);
    watch.observe(one);
    return () => watch.disconnect();
  }, []);

  const big = bigOf(size);
  // The world is larger than the pane: drones and plots are laid out at their own size and he pans and zooms for the rest.
  const sized = useMemo<Size>(() => ({ width: Math.round(size.width * WORLD), height: Math.round(size.height * WORLD) }), [size]);
  // A Job's mission lays its own world round its route; the whole field lays out every repository to fit the pane.
  const wide = size.width >= size.height * 1.4;
  const built = useMemo(() => (scene === undefined ? undefined : sceneOf({ ...scene, big, wide, clashes })), [scene, big, wide, clashes]);
  const world = built?.world ?? sized;
  const layout = useMemo(() => built?.layout ?? layoutOf(repos, drones, world, big, prs, clashes), [built, repos, drones, world, big, prs, clashes]);
  const plane = { width: world.width, height: world.height * FLAT };
  const territory = new Map(layout.territories.map((one) => [one.root, one] as const));
  const padAt = new Map(layout.territories.flatMap((one) => one.pads.map((pad) => [pad.key, onPlane(pad)] as const)));
  const baseOf = (root: string): P3 => {
    const t = territory.get(root);
    const at = onPlane(t === undefined ? { x: world.width / 2, y: world.height } : { x: t.base.x - t.base.w / 2 + 36, y: t.base.y + 8 });
    return { ...at, z: 20 };
  };
  const stand = standAt === undefined ? undefined : built?.route.pins.find((one) => one.id === standAt)?.at;
  const placed = useMemo(() => placeBuilds(builds, layout, stand), [builds, layout, stand?.x, stand?.y]);
  // A Job's ring in its mission is asked for an update from the stack beside the zoom, since the ring itself takes no press.
  const mine = scene === undefined ? undefined : placed.find((one) => one.build.key.startsWith("job:"))?.build.key;
  const base = useMemo(() => frameOf({ layout, drones, plays, big, missionJob: scene?.job }), [layout, drones, plays, big, scene?.job]);
  const frame = useMemo(() => aimedAt(base, placed), [base, placed]);
  const { heights, floorAt, altOf, flying, aimOf, doers } = frame;
  const droneBy = new Map(drones.map((one) => [one.key, one] as const));
  const sorted = [...drones].sort((a, b) => (layout.at.get(a.key)?.y ?? 0) - (layout.at.get(b.key)?.y ?? 0));
  const playOf = (key: string, kind: Play["kind"]) => plays.find((one) => one.key === key && one.kind === kind);

  // The whole field at rest, found from the projection itself: every territory with its base and flag and every drone with its altitude.
  const home = useMemo<Cam>(
    () => homeOf(size, plane, marksOf(layout.territories, [...layout.at].map(([key, at]) => ({ at, call: calls.has(key) })), big, reachOf(FOOT, MARGIN, big))),
    [layout, plane.width, plane.height, size, big, calls],
  );
  const limits = limitsOf(size, plane, world.width, home);
  const fit = (c: Cam): Cam => limited(plane, limits, c);

  // An arrival (a drone that was not there, a merge that landed) is shown whole: the camera pulls back to it, then returns.
  const arrival = plays.filter((one) => one.id.startsWith("arrived:") || one.id.startsWith("merged:"));
  const pull = (() => {
    if (arrival.length === 0) return undefined;
    const marks: Point[] = [];
    for (const play of arrival) {
      const own = play.key === undefined ? undefined : layout.at.get(play.key);
      const root = play.root ?? own?.root ?? prs.find((pr) => pr.number === play.number)?.root;
      if (own !== undefined) marks.push(own);
      const goal = root === undefined ? undefined : territory.get(root)?.goal;
      if (goal !== undefined) marks.push(goal);
    }
    if (marks.length === 0) return undefined;
    const l = Math.min(...marks.map((m) => m.x)) - 220;
    const r = Math.max(...marks.map((m) => m.x)) + 220;
    const t = Math.min(...marks.map((m) => m.y)) - 200;
    const b = Math.max(...marks.map((m) => m.y)) + 160;
    const fitted = Math.min(size.width / (r - l), size.height / ((b - t) * FLAT * COS)) * 0.92;
    return fit({ x: (l + r) / 2, y: ((t + b) / 2) * FLAT, eye: Math.min(home.eye, fitted), tilt: false });
  })();

  const [tip, setTip] = useState<{ tip: Tip; anchor: Anchor }>();
  const [hot, setHot] = useState<string>();
  const locateRef = useRef<() => void>(() => undefined);
  const view = useCamera({
    host,
    size,
    plane,
    limits,
    home,
    pull,
    arriving: arrival.length > 0,
    keep: scene === undefined,
    locate: () => locateRef.current(),
    onHand: () => {
      setTip(undefined);
      setHot(undefined);
    },
  });
  const { eff, latest, go, zoomMid, homeAgain } = view;
  const pitch = eff.tilt ? CAM.near : CAM.wide;

  // The mission opens on the step at work, close enough to read it, and returns there when asked.
  const liveAt = built?.route.live;
  const toLive = () => (liveAt === undefined ? homeAgain() : go(lookingAt(liveAt, Math.max(MISSION_EYE, home.eye * 1.3), false), "ease"));
  useEffect(() => {
    if (measured && liveAt !== undefined) toLive();
  }, [measured, liveAt?.x, liveAt?.y]);
  const lock = selected === undefined ? undefined : layout.at.get(selected);
  const toSelection = () => (scene !== undefined ? toLive() : lock === undefined ? homeAgain() : go(lookingAt(lock, Math.max(latest.current.eye, LOCK_EYE)), "ease"));
  locateRef.current = toSelection;
  // Lock-on: the camera eases to the drone picked, closer and lower. Not at the mount while the camera was left somewhere.
  const first = useRef(scene === undefined && kept.current !== undefined);
  useEffect(() => {
    if (lock === undefined) return;
    if (first.current) {
      first.current = false;
      return;
    }
    go(lookingAt(lock, Math.max(latest.current.eye, LOCK_EYE)), "ease");
  }, [selected, lock?.x, lock?.y]);
  // `w` named a drone to go to: the camera eases to it once the map is drawn.
  useEffect(() => {
    const key = scene === undefined ? aimAt.key : undefined;
    const at = key === undefined ? undefined : layout.at.get(key);
    if (at === undefined) return;
    aimAt.key = undefined;
    first.current = false;
    go(lookingAt(at, Math.max(latest.current.eye, LOCK_EYE)), "ease");
  }, [layout]);

  // Another repository drawn: the camera lets go of where it was and eases to the fit of the new field.
  const shown = repos.map((one) => one.root).join("\n");
  const was = useRef(shown);
  useEffect(() => {
    if (was.current === shown) return;
    was.current = shown;
    if (scene === undefined) homeAgain();
  }, [shown]);

  // The lifted lines are hidden while the pitch eases, since they are drawn for the pitch it is going to.
  const [easing, setEasing] = useState(false);
  const eased = useRef(pitch);
  useEffect(() => {
    if (eased.current === pitch) return;
    eased.current = pitch;
    setEasing(true);
    const timer = window.setTimeout(() => setEasing(false), 1200);
    return () => window.clearTimeout(timer);
  }, [pitch]);
  const lift = liftOf(pitch);
  const curve = (a: P3, b: P3, rise: number, n = 18) => {
    const points = Array.from({ length: n + 1 }, (_, i) => lift(alongOf(a, b, rise, i / n)));
    return `M${points.map((p) => `${p.x.toFixed(1)},${p.y.toFixed(1)}`).join(" L")}`;
  };

  // A marker whose call was opened or answered stays a moment and fades.
  const [gone, setGone] = useState<ReadonlyMap<string, CallTone>>(new Map());
  const marked = useRef(calls);
  useEffect(() => {
    const left = [...marked.current].filter(([key]) => !calls.has(key));
    marked.current = calls;
    if (left.length === 0) return;
    setGone((was) => new Map([...was, ...left]));
    const timer = window.setTimeout(() => setGone((was) => new Map([...was].filter(([key]) => !left.some(([one]) => one === key)))), 320);
    return () => window.clearTimeout(timer);
  }, [calls]);
  const callOf = (key: string) => calls.get(key) ?? gone.get(key);

  const loops = loopsOf(doers, { focus, selected, hot });
  // Rotors turn and hovering bobs for at most as many drones as the actions loop for, the focused, selected and hot first; the rest hold the same pose still.
  const alive = loopsOf(drones.filter((one) => one.hover), { focus, selected, hot });
  const writing = new Map<string, "loop" | "still">();
  for (const one of doers) if (one.doing === "write") writing.set(aimOf(one)!.path, loops.has(one.key) || writing.get(aimOf(one)!.path) === "loop" ? "loop" : "still");
  const showTip: OnTip = (next, target, at) => {
    if (next === undefined || target === undefined) return setTip(undefined);
    setTip({ tip: next, anchor: { target, ...(at === undefined ? {} : { at }) } });
  };
  // A pin pressed: its step's card stays open beside it until Esc, a press on the ground, or another pin.
  const [pinned, setPinned] = useState<{ id: string; target: Element }>();
  const open = useRef(pinned);
  open.current = pinned;
  const pin = pinned === undefined ? undefined : built?.route.pins.find((one) => one.id === pinned.id);
  const card = useMemo<Tip | undefined>(() => (pin === undefined ? undefined : { ...tipOf(pin), node: <WorkflowStepCard {...pin.card} /> }), [pin]);
  // Esc puts the callout away; it comes back when the pointer or focus comes to something again. A card pressed open goes first, and is the whole of that Esc.
  useEffect(() => {
    const away = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      if (open.current !== undefined) {
        event.preventDefault();
        event.stopImmediatePropagation();
        setPinned(undefined);
        return;
      }
      setTip(undefined);
      setHot(undefined);
    };
    window.addEventListener("keydown", away, true);
    return () => window.removeEventListener("keydown", away, true);
  }, []);

  // The Session he is looking at, if it is building something: its card stays open beside it until the pointer is on something else.
  const building = selected === undefined ? undefined : facts.get(selected)?.build;
  const lockedDrone = selected === undefined ? undefined : droneBy.get(selected);
  const lockedEl = building === undefined || host.current === null ? null : host.current.querySelector(`[data-key="${selected}"]`);
  const lockedTip = useMemo(() => (building === undefined || lockedDrone === undefined || selected === undefined ? undefined : droneTip(lockedDrone, facts.get(selected), ShieldCheck)), [building, lockedDrone, facts, selected]);
  const lockedAnchor = useMemo(() => (lockedEl === null ? undefined : { target: lockedEl }), [lockedEl]);
  const locked = selected !== undefined && lock !== undefined;
  const [hotTile, setHotTile] = useState<string>();
  // The plots: a name for those the selected drone is over and those a calling drone is over, a lift for those a working drone is over,
  // an amber slab where Fleet says two are writing.
  const pulsing = new Set((focus === undefined ? [] : (layout.over.get(focus) ?? [])).map((one) => one.tile.path));
  const clashAt = new Map(layout.clashes.map((one) => [one.path, one] as const));
  const named = new Set<string>();
  const lit = new Set<string>();
  for (const drone of drones) {
    if (!drone.shown) continue;
    for (const { tile } of layout.over.get(drone.key) ?? []) {
      if (drone.key === selected || drone.ping) named.add(tile.path);
      if (drone.spin) lit.add(tile.path);
    }
  }
  // Where each drone shows on the screen, near enough to keep one name off another: the plane stands out of the screen by its tilt.
  const place = (key: string) => {
    const at = layout.at.get(key);
    const drone = droneBy.get(key);
    return at === undefined || drone === undefined ? undefined : { x: at.x, y: at.y, up: altOf(drone) * SIN };
  };
  const bodies = bodiesOf(drones, place, big);
  // Zoomed out to the whole field the type is too small to read: a name is dropped rather than drawn unreadable, and the callout says it.
  const readable = eff.eye >= READABLE;
  const { labels, taken } = labelsOf({ drones, place, bodies, big, readable, selected, hot });
  // The slabs that rise when something touches them: a Check, a turn, a launch under a drone; and the wave a merge sends over its territory.
  const bump = new Map<string, { wait: number; amp: number }>();
  for (const play of plays) {
    if ((play.kind === "launch" || play.kind === "sweep" || play.kind === "tracer") && play.key !== undefined) {
      for (const { tile } of layout.over.get(play.key) ?? []) if (!bump.has(tile.path)) bump.set(tile.path, { wait: play.delay + (play.kind === "launch" ? 900 : 250), amp: 8 });
    }
    if (play.kind === "capture") {
      const root = play.root ?? prs.find((pr) => pr.number === play.number)?.root;
      const at = root === undefined ? undefined : territory.get(root);
      if (at?.goal === undefined) continue;
      for (const tile of at.tiles) bump.set(tile.path, { wait: Math.round(play.delay + 900 + Math.hypot(tile.cx - at.goal.x, (tile.cy - at.goal.y) * FLAT) * 1.1), amp: 6 });
    }
  }
  const landing = new Set(plays.filter((one) => one.kind === "capture" && one.number !== undefined).map((one) => one.number));
  const transform = `rotateX(${pitch}deg) rotateZ(${CAM.yaw}deg) scale3d(${eff.eye}, ${eff.eye}, ${eff.eye}) translate3d(${px(-eff.x)}, ${px(-eff.y)}, ${px(0)})`;
  const locateWord = scene !== undefined ? "Back to the step at work" : lock === undefined ? "Back to the whole field" : "Locate the selection";

  return (
    <div
      ref={host}
      className="bf"
      tabIndex={-1}
      data-locked={locked || undefined}
      data-live={view.live || undefined}
      data-grab={view.grab || undefined}
      {...view.handlers}
      style={vars({ "--pitch": `${pitch}deg`, "--yaw": `${CAM.yaw}deg`, "--eye": eff.eye.toFixed(3), perspective: px(Math.round(size.width * 2.8)) })}
    >
      <svg className="bf-grain" aria-hidden="true" width="100%" height="100%">
        <filter id="bf-noise" x="0" y="0" width="100%" height="100%">
          <feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="1" seed="5" />
          <feColorMatrix values="0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  0 0 0 0.5 0" />
        </filter>
        <rect width="100%" height="100%" filter="url(#bf-noise)" />
      </svg>
      <div className="bf-origin">
        <div className="bf-cam" style={{ transform }}>
          <div className="bf-plane" role="listbox" aria-label={scene === undefined ? "Map" : "Route"} onClick={(event) => {
            if ((event.target as Element).closest("[role=option]") !== null) return;
            setPinned(undefined);
            onClear?.();
          }} style={{ width: plane.width, height: plane.height }}>
            <div className="bf-floor" style={{ left: -plane.width * 0.6, top: -plane.height * 0.9, width: plane.width * 2.2, height: plane.height * 2.1 }} />

            {/* The ground itself: each territory's front line, the border round each top directory, the bases and their pads. */}
            <svg className="bf-ground" width={plane.width} height={plane.height}>
              {layout.territories.map((one) => (
                <g key={one.root} className="bf-territory">
                  <rect className="bf-front" aria-hidden="true" x={one.rect.x} y={one.rect.y * FLAT} width={one.rect.w} height={one.rect.h * FLAT} rx={30} />
                  {one.groups.map((group) => (
                    <rect key={group.dir} className="bf-group" aria-hidden="true" x={group.rect.x} y={group.rect.y * FLAT} width={group.rect.w} height={group.rect.h * FLAT} rx={8} />
                  ))}
                </g>
              ))}
              {built === undefined ? null : <RouteLegs route={built.route} />}
              {layout.territories.map((one) => (
                <Base key={one.root} territory={one} onTip={showTip} />
              ))}
              {layout.territories.map((one) =>
                one.pads.map((pad) => {
                  const info = pads.get(pad.key);
                  return info === undefined ? null : <Pad key={pad.key} pad={info} at={onPlane(pad)} onPress={() => onPad(pad.key)} onTip={showTip} />;
                }),
              )}
            </svg>
            {layout.territories.map((one) => (
              <Hangar key={one.root} territory={one} />
            ))}

            {/* The slabs. */}
            {layout.territories.map((one) =>
              one.tiles.map((tile) => {
                const room = Math.max(4, Math.floor((tile.rect.w - 12) / 6.3));
                const there = (heights.get(tile.path) ?? 0) * SIN;
                const box = { l: tile.rect.x + 5, r: tile.rect.x + 9 + Math.min(room, tile.path.length) * 6.4, t: tile.rect.y - there - 16, b: tile.rect.y - there };
                const show = plotNamed({ box, wanted: named.has(tile.path), readable, hot: hotTile === tile.path, bodies, taken });
                const clash = clashAt.get(tile.path);
                return (
                  <Slab
                    key={tile.path}
                    tile={tile}
                    height={heights.get(tile.path) ?? 0}
                    lit={lit.has(tile.path)}
                    clash={clash !== undefined}
                    named={show}
                    pulse={pulsing.has(tile.path)}
                    writing={writing.get(tile.path)}
                    bump={bump.get(tile.path)}
                    room={room}
                    onEnter={(event) => {
                      setHotTile(tile.path);
                      showTip(clash === undefined ? { label: "Directory", title: tile.path, hue: "quiet" } : { label: "Write clash", title: tile.path, hue: "ask", lines: clashLines(clash) }, event.currentTarget, { x: event.clientX, y: event.clientY });
                    }}
                    onLeave={() => {
                      setHotTile(undefined);
                      showTip(undefined);
                    }}
                  />
                );
              }),
            )}

            <Doings layout={layout} frame={frame} plays={plays} droneBy={droneBy} loops={loops} focus={focus} />

            <Builds placed={placed} frame={frame} big={big} loops={loops} onTip={showTip} />

            <Lifted
              layout={layout}
              frame={frame}
              drones={drones}
              droneBy={droneBy}
              territory={territory}
              padAt={padAt}
              baseOf={baseOf}
              trails={trails}
              plays={plays}
              prs={prs}
              selected={selected}
              loops={loops}
              crate={built?.route.crate}
              plane={plane}
              pitch={pitch}
              easing={easing}
              curve={curve}
              lift={lift}
              onTip={showTip}
            />

            {layout.territories.map((one) => {
              const goal = goals.get(one.root);
              if (goal === undefined || one.goal === undefined) return null;
              const capture = plays.find((p) => p.kind === "capture" && (p.root === one.root || (p.number !== undefined && prs.some((pr) => pr.number === p.number && pr.root === one.root))));
              return <Goal key={one.root} at={onPlane(one.goal)} goal={goal} repo={one.name} capture={capture} onPress={() => onGoal(one.root)} onTip={showTip} />;
            })}

            {/* Pull requests in the air, on the route from the drone that owns one, or from the base where none does, to the objective. */}
            {prs.map((pr) => {
              const at = territory.get(pr.root);
              if (at?.goal === undefined) return null;
              const owner = pr.owner === undefined ? undefined : droneBy.get(pr.owner);
              const start = built !== undefined ? { ...onPlane(built.route.crate), z: ROUTE } : ((owner === undefined ? undefined : flying(owner)) ?? { ...baseOf(pr.root), z: ROUTE });
              const end: P3 = { ...onPlane(at.goal), z: ROUTE };
              const landed = pr.stage === "landed";
              if (landed && !landing.has(pr.number)) return null;
              const rest = alongOf(start, end, 34, landed ? ALONG.queued : ALONG[pr.stage]);
              const arrived = plays.find((p) => p.kind === "capture" && p.number === pr.number);
              const opened = plays.find((p) => p.kind === "payload" && p.number === pr.number);
              return (
                <div key={pr.key} className="bf-routing" data-dim={(owner !== undefined && !owner.shown) || undefined}>
                  <Payload at={landed ? end : rest} from={landed ? rest : start} pr={pr} arriving={landed ? arrived : undefined} fresh={landed ? undefined : opened} onPress={() => onPr(pr.key)} onTip={showTip} />
                </div>
              );
            })}

            {built === undefined ? null : (
              <Pins
                route={built.route}
                pinned={pinned?.id}
                readable={readable}
                onPress={(one, target) => {
                  setTip(undefined);
                  setPinned((was) => (was?.id === one.id ? undefined : { id: one.id, target }));
                }}
                onTip={(next, target) => (next === undefined || pinned !== undefined ? showTip(undefined) : showTip(next, target))}
              />
            )}

            {sorted.map((drone, index) => {
              const at = layout.at.get(drone.key);
              if (at === undefined) return null;
              const launch = playOf(drone.key, "launch");
              const pad = drone.slot === undefined ? undefined : padAt.get(drone.slot);
              const origin = pad === undefined ? baseOf(drone.root) : { ...pad, z: 4 };
              const on = selected === drone.key;
              const centre = onPlane(at);
              // Running tools or Checks, it flies round a ring about the place it hovers: its start is on the ring, and the wrapper turns.
              const circling = drone.doing === "run" && aimOf(drone) !== undefined;
              const here = circling ? { x: centre.x + frame.orbit, y: centre.y } : centre;
              const loop = loops.has(drone.key);
              return (
                <div key={drone.key} className="bf-orbit" data-circling={(circling && loop) || undefined} style={{ transformOrigin: `${px(centre.x)} ${px(centre.y)}` }}>
                  <div className="bf-orbit__face" style={{ transformOrigin: `${px(here.x)} ${px(here.y)}` }}>
                    <Drone
                      drone={drone}
                      at={here}
                      floor={floorAt(at)}
                      alt={altOf(drone)}
                      from={launch === undefined ? undefined : { x: origin.x - here.x, y: origin.y - here.y, z: origin.z }}
                      index={index}
                      big={big}
                      selected={on}
                      hot={hot === drone.key}
                      labelled={labels.has(drone.key)}
                      muted={drone.shown && locked && !on && drone.hue === "calm"}
                      away={!drone.shown}
                      focused={focus === drone.key}
                      dim={focus !== undefined && focus !== drone.key}
                      play={launch}
                      facts={facts.get(drone.key)}
                      loop={loop}
                      alive={alive.has(drone.key)}
                      call={callOf(drone.key)}
                      leaving={!calls.has(drone.key)}
                      onSelect={() => onSelect(drone.key)}
                      onTip={(next, target) => {
                        setHot(next === undefined ? undefined : drone.key);
                        showTip(next, target);
                      }}
                    />
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      </div>
      <div className="bf-haze" aria-hidden="true" />
      {/* A drone that holds a call and is outside the pane: its badge at the border, an arrow toward it. Pressing it eases the camera there. */}
      {[...new Map([...gone, ...calls])].map(([key, tone]) => {
        const drone = droneBy.get(key);
        const at = layout.at.get(key);
        if (drone === undefined || at === undefined) return null;
        const edge = edgeOf(size, eff, { x: at.x, y: at.y * FLAT, z: altOf(drone) + 40 * big });
        if (edge === undefined) return null;
        const leaving = !calls.has(key);
        const tipped: Tip = { label: drone.kind === "job" ? "Job" : "Session", title: drone.title, hue: tone, lines: ["Needs you"] };
        return (
          <button
            key={key}
            type="button"
            className="bf-edge"
            data-tone={tone}
            data-leaving={leaving || undefined}
            tabIndex={leaving ? -1 : 0}
            aria-label={`${drone.title} needs you`}
            style={{ left: edge.x, top: edge.y }}
            onClick={() => go({ x: at.x, y: (at.y - 30) * FLAT, eye: latest.current.eye, tilt: latest.current.tilt }, "ease")}
            onMouseEnter={(event) => showTip(tipped, event.currentTarget)}
            onMouseLeave={() => showTip(undefined)}
            onFocus={(event) => showTip(tipped, event.currentTarget)}
            onBlur={() => showTip(undefined)}
          >
            <Megaphone size={16} strokeWidth={2.2} aria-hidden="true" />
            <i className="bf-edge__arrow" style={{ transform: `rotate(${edge.angle.toFixed(1)}deg) translateX(${px(24)})` }} />
          </button>
        );
      })}
      {tip === undefined || host.current === null ? null : <Callout tip={tip.tip} anchor={tip.anchor} host={host.current} />}
      {tip !== undefined || pinned !== undefined || lockedTip === undefined || lockedAnchor === undefined || host.current === null ? null : <Callout key={`locked:${selected}`} tip={lockedTip} anchor={lockedAnchor} host={host.current} pinned />}
      {pinned === undefined || card === undefined || host.current === null ? null : <Callout key={pinned.id} tip={card} anchor={{ target: pinned.target }} host={host.current} pinned />}
      <div className="bf-zoom">
        {mine === undefined ? null : <AskUpdate subject={mine} />}
        <Tooltip label="Zoom in" shortcut="+" asChild>
          <Button variant="ghost" iconOnly aria-label="Zoom in" disabled={eff.eye >= limits.most - 0.001} onClick={() => zoomMid(ZOOM_STEP)}>
            <Plus size={16} aria-hidden="true" />
          </Button>
        </Tooltip>
        <Tooltip label="Zoom out" shortcut="-" asChild>
          <Button variant="ghost" iconOnly aria-label="Zoom out" disabled={eff.eye <= limits.least + 0.001} onClick={() => zoomMid(1 / ZOOM_STEP)}>
            <Minus size={16} aria-hidden="true" />
          </Button>
        </Tooltip>
        <Tooltip label={locateWord} shortcut="Home" asChild>
          <Button variant="ghost" iconOnly aria-label={locateWord} onClick={toSelection}>
            <Crosshair size={16} aria-hidden="true" />
          </Button>
        </Tooltip>
        {scene === undefined ? null : (
          <Tooltip label="Whole route" shortcut="0" asChild>
            <Button variant="ghost" iconOnly aria-label="Whole route" onClick={homeAgain}>
              <Scan size={16} aria-hidden="true" />
            </Button>
          </Tooltip>
        )}
      </div>
    </div>
  );
}
