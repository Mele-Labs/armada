// The flight deck's map, a battlefield on a tilted plane drawn on a canvas. Each repository is a territory with a base at its near edge,
// its landing pads the bays, and its main the objective on the far edge, a flag. Each Session is a drone on a beam over the plots it
// changes (or at its pad where nothing observed puts it elsewhere), a small plus quad. State is the drone and its frame, never words.
// Pull requests are crates on a lifted route to the flag. The activity log drives the scene (`plays.ts`). **Every fact on it is read off
// what Fleet recorded**; the field is handed already read (`useField.ts`).
//
// This reads the field into a scene (`pixi/scene.ts`) and keeps the camera (`useCamera.ts`); the canvas draws it (`pixi/stage.ts`) and an
// invisible layer over it carries what can be pressed, hovered or tabbed to (`pixi/Hits.tsx`). Nothing here draws a frame. Ported from
// proto/cockpit-flight-deck. #2301.

import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { Crosshair, Megaphone, Minus, Plus, ShieldCheck } from "lucide-react";
import { Button, Tooltip, useClashesOf } from "@armada/components";

import { aimedAt, placeBuilds } from "./builds";
import type { Outpost } from "./outposts";
import { Hits, type HitActs } from "./pixi/Hits";
import { PixiMap } from "./pixi/PixiMap";
import { outpostAt, sceneOf as pixiSceneOf, type PixiSceneIn } from "./pixi/scene";
import type { Stage } from "./pixi/stage";
import { onPlane } from "./bf-parts";
import { LOCK_EYE, READABLE, WORLD, ZOOM_STEP, edgeOf, held as limited, homeOf, limitsOf, lookingAt, marksOf, reachOf, type Cam } from "./camera";
import { loopsOf, type CallTone } from "./field";
import { frameOf } from "./frame";
import { COS, FOOT, MARGIN, SIN, short } from "./geometry";
import { bodiesOf, labelsOf, plotNamed } from "./labels";
import type { Play } from "./plays";
import { FLAT, bigOf, layoutOf, type DroneIn, type P3, type Point, type PrIn, type RepoIn, type Size, type Tile } from "./layout";
import type { BuildIn } from "./build";
import { aimAt, camera as kept } from "./store";
import { TipLayer } from "./TipLayer";
import { steadyTips, type Waits } from "./tip-hover";

import { droneTip, type DroneFacts, type GoalIn, type PadIn, type Tip } from "./tip";
import type { Clash } from "./touches";
import type { Glow, Stop } from "./afterglow";
import type { Light, LinkIn } from "./off-floor";
import { MOST_LOOPS } from "./doing";
import type { Slab } from "./lens";
import { LensWires } from "./LensWires";
import { useCamera } from "./useCamera";
import "./battlefield.css";

const cssMs = (name: string, fallback: number): number => {
  const parsed = Number.parseInt(getComputedStyle(document.documentElement).getPropertyValue(name), 10);
  return Number.isNaN(parsed) ? fallback : parsed;
};
const SHARED_WAITS: Waits = { delay: () => cssMs("--tooltip-delay", 400), grace: () => cssMs("--tooltip-grace", 300) };

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
  /** The root of the one repository drawn, where the map is cut to one. A change of it sends the camera to the new fit. */
  scope?: string | undefined;
  /** This map's camera is its own, not the window's: it opens on the whole field, and goes back there when this changes, and when the pane is resized or unfolded. A Job's Field. */
  own?: string | undefined;
  /** The ring under each drone that is building one. None where Fleet reports none. */
  builds?: readonly BuildIn[] | undefined;
  /** The plots a Session's plan names, and what the plan says of each. A Session's Field only. */
  planned?: PixiSceneIn["planned"];
  /** The services Sessions have called, each an outpost past the far edge of the field. None draws none. */
  outposts?: readonly Outpost[] | undefined;
  /** The plots a Job has touched, outlined in the accent. A Job's Field only. */
  touched?: ReadonlySet<string> | undefined;
  /** The glow on each file's bar from the Job's reads and writes. A Job's Field only. */
  glow?: ReadonlyMap<string, Glow> | undefined;
  /** The Job's stops, drawn while the Field is scrubbed back. A Job's Field only. */
  route?: readonly Stop[] | undefined;
  /** The Job's own drones, where other Jobs' are drawn too: the button that brings the camera home names the Job. */
  framing?: readonly string[] | undefined;
  /** The Job's checks, a light each along its pad at the near edge. A Job's Field only: absent draws no pad. */
  lights?: readonly Light[] | undefined;
  /** The walk a drone of the Job is on, by its name, in a window over the pad. A Job's Field only. */
  walk?: string | undefined;
  /** The files the Job's drones are writing, lifted, and the card at the top right joined to the card's own. A Job's Field only. */
  lens?: readonly Slab[] | undefined;
  lensCard?: ReactNode;
  /** The card is typing out a hunk: one of the few loops the map runs at once. */
  typing?: boolean;
  /** The messages between the Job and other Jobs while fresh, an arc each between two drones. A Job's Field only. */
  links?: readonly LinkIn[] | undefined;
  onSelect: (key: string) => void;
  /** A press on the ground, on no drone: whatever was picked lets go. */
  onClear?: () => void;
  onPad: (key: string) => void;
  onGoal: (root: string) => void;
  onPr: (key: string) => void;
};

const NO_BUILDS: readonly BuildIn[] = [];
const NO_OUTPOSTS: readonly Outpost[] = [];
const NO_LINKS: readonly LinkIn[] = [];

/** The points the whole field must show for its outposts: the foot of each and a height over its badge, which keeps the badge clear of the haze at the pane's top edge. */
const outpostMarks = (world: Size, n: number): P3[] =>
  Array.from({ length: n }, (_, k) => {
    const at = outpostAt(world, k, n);
    return [{ x: at.x - 40, y: at.y * FLAT, z: 0 }, { x: at.x + 40, y: at.y * FLAT, z: 230 }];
  }).flat();

export function Battlefield({ repos, drones, pads, goals, prs, trails, plays, facts, calls, clashes, focus, selected, scope, own, builds = NO_BUILDS, planned, outposts = NO_OUTPOSTS, touched, glow, route, framing, lights, walk, lens, lensCard, typing = false, links = NO_LINKS, onSelect, onClear, onPad, onGoal, onPr }: BattlefieldProps) {
  const host = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState<Size>({ width: 900, height: 560 });
  // Bumped when the pane takes a new size, or has one again after being folded away: an own camera refits then.
  const [fresh, setFresh] = useState(0);
  // Folded away, the pane has no size: its canvas draws nothing until it has one again.
  const [folded, setFolded] = useState(false);
  useEffect(() => {
    const one = host.current;
    if (one === null) return;
    let shown: string | undefined;
    const read = () => {
      const box = one.getBoundingClientRect();
      if (box.width <= 0 || box.height <= 0) {
        shown = undefined;
        setFolded(true);
        return;
      }
      setFolded(false);
      const now = `${Math.round(box.width)}x${Math.round(box.height)}`;
      if (now !== shown) {
        shown = now;
        setFresh((was) => was + 1);
      }
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
  // The whole field lays out every repository to fit the pane.
  const world = sized;
  const layout = useMemo(() => layoutOf(repos, drones, world, big, prs, clashes), [repos, drones, world, big, prs, clashes]);
  const plane = { width: world.width, height: world.height * FLAT };
  const territory = new Map(layout.territories.map((one) => [one.root, one] as const));
  const padAt = new Map(layout.territories.flatMap((one) => one.pads.map((pad) => [pad.key, onPlane(pad)] as const)));
  const placed = useMemo(() => placeBuilds(builds, layout), [builds, layout]);
  const base = useMemo(() => frameOf({ layout, drones, plays, big }), [layout, drones, plays, big]);
  const frame = useMemo(() => aimedAt(base, placed), [base, placed]);
  const { heights, altOf, doers } = frame;
  // The drone writing the card's file, where it flies: the card keeps clear of it.
  const cardPath = lens?.find((one) => one.card)?.path;
  const writer = cardPath === undefined ? undefined : drones.find((one) => one.ghost !== true && one.doing === "write" && one.file === cardPath);
  const writerAt = writer === undefined ? undefined : frame.flying(writer);
  const droneBy = new Map(drones.map((one) => [one.key, one] as const));

  // The whole field at rest, found from the projection itself: every territory with its base and flag and every drone with its altitude.
  const whole = useMemo<Cam>(
    () => homeOf(size, plane, [...marksOf(layout.territories, [...layout.at].map(([key, at]) => ({ at, call: calls.has(key) })), big, reachOf(FOOT, MARGIN, big)), ...outpostMarks(world, outposts.length)]),
    [layout, plane.width, plane.height, size, big, calls, outposts.length],
  );
  // Every map rests on the whole field, a Job's Field too: its drones are seen where they are in the repository, and the lens's card
  // stands over the map without moving the camera.
  const linksKey = links.map((one) => `${one.from}>${one.to}`).join("|");
  const home = whole;
  const limits = limitsOf(size, plane, world.width, whole);
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

  const [hot, setHot] = useState<string>();
  const [hotTile, setHotTile] = useState<string>();
  const [hotPart, setHotPart] = useState<string>();
  // What the pointer is on is held here, apart from this component's state: a card that comes and goes redraws the card and nothing else, and
  // the drone or tile it lights moves only once the card has settled (`tip-hover.ts`).
  const [tips] = useState(() => steadyTips(undefined, SHARED_WAITS));
  tips.onSettle((shown) => {
    setHot(shown?.hot);
    setHotTile(shown?.tile);
    setHotPart(shown?.part);
  });
  useEffect(() => tips.cancel, [tips]);
  const locateRef = useRef<() => void>(() => undefined);
  const view = useCamera({
    host,
    size,
    plane,
    limits,
    home,
    pull,
    arriving: arrival.length > 0,
    keep: own === undefined,
    locate: () => locateRef.current(),
    onHand: () => tips.clear(),
  });
  const { eff, latest, go, zoomMid, homeAgain } = view;

  const lock = selected === undefined ? undefined : layout.at.get(selected);
  const toSelection = () => (lock === undefined ? homeAgain() : go(lookingAt(lock, Math.max(latest.current.eye, LOCK_EYE)), "ease"));
  locateRef.current = toSelection;
  // Lock-on: the camera eases to the drone picked, closer and lower. Not at the mount while the camera was left somewhere.
  const first = useRef(own === undefined && kept.current !== undefined);
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
    const key = own === undefined ? aimAt.key : undefined;
    const at = key === undefined ? undefined : layout.at.get(key);
    if (at === undefined) return;
    aimAt.key = undefined;
    first.current = false;
    go(lookingAt(at, Math.max(latest.current.eye, LOCK_EYE)), "ease");
  }, [layout]);

  // Another repository drawn: the camera lets go of where it was and eases to the fit of the new field.
  const was = useRef(scope);
  useEffect(() => {
    if (was.current === scope) return;
    was.current = scope;
    homeAgain();
  }, [scope]);

  // An own camera is at the whole field for each `own` and each new size of the pane, and stays where he leaves it between those.
  useEffect(() => {
    if (own !== undefined) homeAgain();
  }, [own, fresh]);

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

  // A drone off the code floor that moves (the dots of one thinking, the lights of one on the pad) counts toward the same few as an action,
  // and so does the card typing and each arc's packets, which take theirs first.
  const most = MOST_LOOPS - (typing ? 1 : 0);
  const linkLoops = useMemo(() => new Set(links.slice(0, most).map((one) => one.key)), [linksKey, most]);
  const loops = loopsOf([...doers, ...drones.filter((one) => one.ghost !== true && (one.off === "think" || one.off === "pad"))], { focus, selected, hot }, most - linkLoops.size);
  const showTip = (next: Tip | undefined, target?: Element, at?: { x: number; y: number }, lit?: { hot?: string; tile?: string; part?: string }) => {
    if (next === undefined || target === undefined) return tips.leave();
    tips.show({ tip: next, anchor: { target, ...(at === undefined ? {} : { at }) }, ...lit });
  };
  const [stage, setStage] = useState<Stage>();
  // What the canvas map's pictures do when they are pressed or hovered: this page's, read when wanted so the layer is not drawn again for a new one.
  const hitActs = useRef<HitActs>(null as never);
  hitActs.current = {
    select: onSelect,
    pad: onPad,
    goal: onGoal,
    pr: onPr,
    tip: showTip,
  };
  // Esc puts the callout away; it comes back when the pointer or focus comes to something again.
  useEffect(() => {
    const away = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      tips.clear();
    };
    window.addEventListener("keydown", away, true);
    return () => window.removeEventListener("keydown", away, true);
  }, []);

  // The Session he is looking at, if it is building something: its card stays open beside it until the pointer is on something else.
  const building = selected === undefined ? undefined : facts.get(selected)?.build;
  const lockedDrone = selected === undefined ? undefined : droneBy.get(selected);
  // The drone is an element over the canvas, which is in the page only once the layer has drawn: it is looked for after each draw.
  const [stood, setStood] = useState<Element | null>(null);
  useLayoutEffect(() => {
    const found = building === undefined || host.current === null ? null : host.current.querySelector(`[data-key="${selected}"][data-hit=drone]`);
    setStood((was) => (was === found ? was : found));
  });
  const lockedTip = useMemo(() => (building === undefined || lockedDrone === undefined || selected === undefined ? undefined : droneTip(lockedDrone, facts.get(selected), ShieldCheck)), [building, lockedDrone, facts, selected]);
  const lockedAnchor = useMemo(() => (stood === null ? undefined : { target: stood }), [stood]);
  const locked = selected !== undefined && lock !== undefined;
  // The plots: a name for those the selected drone is over and those a calling drone is over, an amber name where Fleet says two are writing.
  const clashAt = new Map(layout.clashes.map((one) => [one.path, one] as const));
  const named = new Set<string>();
  for (const drone of drones) {
    if (!drone.shown) continue;
    for (const { tile } of layout.over.get(drone.key) ?? []) if (drone.key === selected || drone.ping) named.add(tile.path);
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
  const nameable = useMemo(() => drones.filter((one) => one.ghost !== true || one.linked === true), [drones]);
  const { labels, taken } = labelsOf({ drones: nameable, place, bodies, big, readable, selected, hot });
  // A plot's name is written on it where it has room and meets no body or name; the pointer on the plot always names it.
  const plotShown = (tile: Tile) => {
    const room = Math.max(4, Math.floor((tile.rect.w - 12) / 6.3));
    const there = (heights.get(tile.path) ?? 0) * SIN;
    const box = { l: tile.rect.x + 5, r: tile.rect.x + 9 + Math.min(room, tile.path.length) * 6.4, t: tile.rect.y - there - 16, b: tile.rect.y - there };
    return { room, show: plotNamed({ box, wanted: named.has(tile.path), readable, hot: hotTile === tile.path, bodies, taken }) };
  };
  // Fleet's clashes are read by a hook, one drone at a time; the canvas is handed the counts.
  const [clashed, setClashed] = useState<ReadonlyMap<string, number>>(new Map());
  const countClashes = useCallback((key: string, n: number) => setClashed((was) => ((was.get(key) ?? 0) === n ? was : new Map(was).set(key, n))), []);
  // The sets are rebuilt on every render of the camera; the scene is cut again only when what is in one changes.
  const loopKey = [...loops].sort().join("|");
  const labelKey = [...labels].sort().join("|");
  const heldLoops = useMemo(() => loops, [loopKey]);
  const heldLabels = useMemo(() => labels, [labelKey]);
  // The names written on the plots, where a plot is wider than a name can start in.
  const names = layout.territories.flatMap((one) => one.tiles.flatMap((tile) => { const { room, show } = plotShown(tile); return show && tile.rect.w > 44 ? [{ tile, text: short(tile.path, room), clash: clashAt.has(tile.path) }] : []; }));
  const nameKey = names.map((one) => `${one.tile.path}:${one.text}:${one.clash}`).join("|");
  const heldNames = useMemo(() => names, [nameKey, layout]);
  const pixiScene = useMemo(
    () => pixiSceneOf({ layout, world, big, frame, drones, pads, goals, prs, plays, outposts, readable, builds: placed, selected, hot, hotTile, hotPart, names: heldNames, focus, labels: heldLabels, clashed, calls, callOf, loops: heldLoops, locked, trails, padAt, ...(planned === undefined ? {} : { planned }), ...(touched === undefined ? {} : { touched }), ...(glow === undefined ? {} : { glow }), ...(route === undefined ? {} : { route }), ...(lights === undefined ? {} : { lights }), ...(walk === undefined ? {} : { walk }), ...(lens === undefined ? {} : { lens }), links, linkLoops }),
    [layout, world, big, frame, drones, pads, goals, prs, plays, outposts, readable, placed, selected, hot, hotTile, hotPart, heldNames, focus, heldLabels, clashed, calls, gone, heldLoops, locked, trails, planned, touched, glow, route, lights, walk, lens, links, linkLoops],
  );
  const locateWord = lock !== undefined ? "Locate the selection" : framing !== undefined ? "Back to this Job" : "Back to the whole field";

  return (
    <div ref={host} className="bf" tabIndex={-1} data-grab={view.grab || undefined} {...view.handlers}>
      <svg className="bf-grain" aria-hidden="true" width="100%" height="100%">
        <filter id="bf-noise" x="0" y="0" width="100%" height="100%">
          <feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="1" seed="5" />
          <feColorMatrix values="0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  0 0 0 0.5 0" />
        </filter>
        <rect width="100%" height="100%" filter="url(#bf-noise)" />
      </svg>
      {drones.map((one) => (one.ghost === true ? null : <ClashCount key={one.key} drone={one} report={countClashes} />))}
      <PixiMap
        size={size}
        cam={eff}
        live={view.live}
        scene={pixiScene}
        onGround={(event) => {
          if ((event.target as Element).closest("[data-hit]") !== null) return;
          onClear?.();
        }}
        onStage={setStage}
        hidden={folded}
      >
        {stage === undefined ? null : <Hits stage={stage} scene={pixiScene} facts={facts} acts={hitActs} />}
      </PixiMap>
      <div className="bf-haze" aria-hidden="true" />
      {lens === undefined ? null : <LensWires stage={stage} specs={pixiScene.lens} card={lensCard} writer={writerAt} />}
      {/* A drone that holds a call and is outside the pane: its badge at the border, an arrow toward it. Pressing it eases the camera there. */}
      {[...new Map([...gone, ...calls])].map(([key, tone]) => {
        const drone = droneBy.get(key);
        const at = layout.at.get(key);
        if (drone === undefined || at === undefined) return null;
        const edge = edgeOf(size, eff, { x: at.x, y: at.y * FLAT, z: altOf(drone) + 40 * big });
        if (edge === undefined) return null;
        const leaving = !calls.has(key);
        const tipped: Tip = { label: "Job", title: drone.title, hue: tone, lines: ["Needs you"] };
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
            <i className="bf-edge__arrow" style={{ transform: `rotate(${edge.angle.toFixed(1)}deg) translateX(24px)` }} />
          </button>
        );
      })}
      {host.current === null ? null : <TipLayer tips={tips} host={host.current} view={eff} {...(lockedTip === undefined || lockedAnchor === undefined ? {} : { locked: { tip: lockedTip, anchor: lockedAnchor, key: selected } })} />}
      <div className="bf-zoom">
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
      </div>
    </div>
  );
}

/** Reports how many write clashes Fleet has a drone in. It draws nothing: the map's canvas draws the marks, and the hook that knows is a component's. */
function ClashCount({ drone, report }: { drone: DroneIn; report: (key: string, n: number) => void }) {
  const n = useClashesOf(drone.kind, drone.key.slice(drone.kind.length + 1)).length;
  useEffect(() => report(drone.key, n), [drone.key, n]);
  return null;
}
