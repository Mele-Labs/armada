// What the canvas map is handed to draw: the battlefield's own readings (the layout, the plots' heights, the pull requests),
// cut to what the canvas needs and with the few placements the CSS map made in its render worked out here, once. Pure. Nothing here
// draws, and nothing is read off the page.

import type { Frame } from "../frame";
import type { CallTone } from "../field";
import { ROUTE } from "../geometry";
import { ALONG, type Play } from "../plays";
import { FRESH_LAND_MS } from "../builds";
import { askedOf, seatOf } from "../build";
import { CAM, FLAT, alongOf, type DroneIn, type Layout, type P3, type Point, type PrIn, type Size, type Tile } from "../layout";
import type { GoalIn, PadIn } from "../tip";
import type { placeBuilds } from "../builds";
import { onPlane } from "../bf-parts";
import type { Outpost } from "../outposts";
import type { Glow, Stop } from "../afterglow";
import type { Light, LinkIn } from "../off-floor";
import type { Row, Slab } from "../lens";

/** A construction placed on the field: its ring and where it lies. */
export type Placed = ReturnType<typeof placeBuilds>[number];

/** A pull request placed on its route to the objective, in the plane's own units. */
export type Crate = {
  pr: PrIn;
  /** Where its route begins and ends, in the air. */
  start: P3;
  end: P3;
  /** Where the crate itself is: along the route by how far it has got, or at the objective once it has landed. */
  at: P3;
  /** Its owner is outside the filters: the route and the crate dim. */
  dim: boolean;
  /** It is on its way: a hop along an arc from where it was, and how long it waits to start. Landing hops, then goes. */
  hop: { id: string; from: P3; delay: number; landing: boolean } | undefined;
};

/** A service outpost at the far edge of the field: where it stands on the plane, and whether a Session is calling it now. */
export type OutpostSpec = Outpost & { at: Point; lit: boolean };

/** A plot's name, written on it: the directory shortened to what the plot has room for. */
export type PlotName = { tile: Tile; text: string; /** Two drones are writing it: the name is amber. */ clash: boolean };

/** A subagent that hovers at a fixed place on the ring of the Session that sent it: by the pieces it wrote, else where its place among them puts it. */
export type WorkerSpec = {
  key: string;
  /** What it was asked. */
  detail: string;
  /** The ring it circles, on the plane, and the plot under that. */
  anchor: Point;
  tile: string | undefined;
  /** Its place among the Session's subagents: the further out and higher the later it is. */
  i: number;
  radius: number;
  /** How far round the ring it hovers, in turns from the top of the ring's first piece. */
  phase: number;
};

/** A plot a play lifts: the play that does it, how long after it begins, and how far. */
export type Bump = { id: string; wait: number; amp: number };

/** Where a working drone's action lands: the plot, the point on it nearest the drone and how wide the action is there, on the plane. */
export type Aim = { tile: string; at: Point; wide: number; deep: number };

/** A drone and everything the canvas needs to draw it, read once: where it flies, how it stands and what it is doing over its plot. */
export type DroneSpec = {
  drone: DroneIn;
  /** Where it stands on the plane, the plot under it, and how high it flies. */
  at: Point;
  tile: string | undefined;
  alt: number;
  /** Where it flies in the air, which a line to it ends at. */
  flying: P3;
  selected: boolean;
  hot: boolean;
  labelled: boolean;
  muted: boolean;
  away: boolean;
  focused: boolean;
  dim: boolean;
  call: CallTone | undefined;
  /** The call was opened or answered a moment ago: its marker is going. */
  leaving: boolean;
  /** How many write clashes Fleet has it in, a small amber mark each. */
  clashes: number;
  /** Its action turns, rather than showing as one still frame: one of the most the map animates at once. */
  loop: boolean;
  aim: Aim | undefined;
  /** The service its fresh call went to: the outpost a beam runs to. */
  calling: string | undefined;
  /** Its launch: the play, and where it lifts from. */
  launch: { id: string; delay: number; from: P3 } | undefined;
  /** The bay it holds, where its tether is tied. */
  pad: Point | undefined;
  /** Where its contrail begins: its bay, or the base where it holds none. */
  origin: P3;
  /** How long ago it was dispatched, for the contrail it left. */
  trail: number | undefined;
  /** The plots it touches, the biggest first. */
  touches: readonly { tile: string; at: Point }[];
  /** Another Job's on a Job's own Field: drawn faint, with no beam, label or action. */
  ghost: boolean;
  /** A subagent: drawn smaller than the drone that sent it. */
  small: boolean;
  /** Where the drone that sent it flies, which a faint line joins it to. */
  link: P3 | undefined;
  /** It has finished: it flies back into the drone that sent it, from where it was, and is gone. */
  dock: { id: string; delay: number; to: P3 } | undefined;
};

/** A Job's pad on the plane: its middle and size, a light along its front edge for each check, and the line from a failing one to the file it blames. */
export type PadSpec = {
  at: Point;
  w: number;
  d: number;
  lights: readonly (Light & { at: Point })[];
  /** The light of the failing check, its name, the bar of the file Fleet read from its log and the line. None where it read no file the Field draws. */
  blame: readonly { from: Point; bar: string; check: string; line?: number }[];
};

/**
 * A file lifted off the floor by a drone writing it: its minimap on a slab enlarged over its bar, the bar's own rect left outlined on
 * the floor. On the plane: the bar's middle and rect; in pixels at any zoom: the slab's width and depth, and how far over the plot it rises.
 */
export type LensSpec = { path: string; card: boolean; rows: readonly Row[]; counts: string; tile: string; at: Point; mark: { x: number; y: number; w: number; h: number }; w: number; d: number; up: number };

/** A stop of a Job's route while the Field is scrubbed back: its number, the file's bar on the plane, and whether it was read or written. */
export type RouteSpot = { n: number; act: Stop["act"]; at: Point; tile: string };

/**
 * The arc through the air between two drones while one Job messages the other: from the sender's drone to the receiver's, rising `lift`
 * at its middle, where the quote stands. `other` is the other Job's drone, which a press opens. `loop` is whether its packets travel.
 */
export type ArcSpec = { key: string; other: string; title: string; quote: string; sent: boolean; from: P3; to: P3; lift: number; mid: P3; loop: boolean };

/** How a plot stands against a Session's plan, in the plan bar's own language: solid, outlined, amber, faint, or failed. */
export type PlotPiece = "solid" | "hollow" | "scaffold" | "ghost" | "failed";

export type PixiSceneIn = {
  layout: Layout;
  world: Size;
  /** The plane, whose depth is stretched by `FLAT`. */
  plane: Size;
  big: number;
  heights: ReadonlyMap<string, number>;
  /** The plot and the part (a pad, a flag or a crate, by `pad:<key>`, `goal:<root>` or `crate:<key>`) under the pointer or focus. */
  hotTile: string | undefined;
  hotPart: string | undefined;
  /** The names written on the plots. */
  names: readonly PlotName[];
  /** The subagents round the rings. */
  workers: readonly WorkerSpec[];
  /** The plots a working drone is over, and those Fleet says two are writing. */
  lit: ReadonlySet<string>;
  clashing: ReadonlySet<string>;
  pads: ReadonlyMap<string, PadIn>;
  goals: ReadonlyMap<string, GoalIn>;
  crates: readonly Crate[];
  /** The services Sessions have called, in a row past the field's far edge. */
  outposts: readonly OutpostSpec[];
  /** Zoomed in far enough that a name can be read. */
  readable: boolean;
  builds: readonly Placed[];
  drones: readonly DroneSpec[];
  plays: readonly Play[];
  /** The plots a play lifts, and when. */
  bumps: ReadonlyMap<string, Bump>;
  /** The plots a Session's plan names, by path, and what the plan says of each. A Session's Field only. */
  planned?: ReadonlyMap<string, { piece: PlotPiece; said: string }>;
  /** The plots a Job has touched, outlined in the accent. A Job's Field only. */
  touched?: ReadonlySet<string>;
  /** How each file's bar glows from what the Job last did to it, by path. A Job's Field only. */
  glow?: ReadonlyMap<string, Glow>;
  /** The Job's stops up to the slider, where it is scrubbed back. A Job's Field only. */
  route?: readonly RouteSpot[];
  /** The Job's pad and its lights. A Job's Field only. */
  pad?: PadSpec;
  /** The walk a drone of the Job is on: its name, in a window over the pad. A Job's Field only. */
  walk?: { name: string; at: P3 };
  /** The files lifted by the drones writing them, the card's last. A Job's Field only. */
  lens?: readonly LensSpec[];
  /** The arcs between this Job's drone and the other Jobs it is messaging or is messaged by. A Job's Field only. */
  arcs: readonly ArcSpec[];
  /** The radius of the ring a running drone's sweep turns on. */
  orbit: number;
  /** Two drones changing one area, each end where it flies. */
  clashLines: readonly { key: string; from: P3; to: P3 }[];
};

export type SceneInputs = {
  layout: Layout;
  world: Size;
  big: number;
  frame: Frame;
  drones: readonly DroneIn[];
  pads: ReadonlyMap<string, PadIn>;
  goals: ReadonlyMap<string, GoalIn>;
  prs: readonly PrIn[];
  plays: readonly Play[];
  /** The services Sessions have called. */
  outposts?: readonly Outpost[];
  readable: boolean;
  builds: readonly Placed[];
  selected: string | undefined;
  hot: string | undefined;
  hotTile: string | undefined;
  hotPart: string | undefined;
  names: readonly PlotName[];
  focus: string | undefined;
  /** The drones whose name is drawn. */
  labels: ReadonlySet<string>;
  /** How many write clashes Fleet has each drone in. */
  clashed: ReadonlyMap<string, number>;
  calls: ReadonlyMap<string, CallTone>;
  /** The call markers on a drone, whether live or still going out. */
  callOf: (key: string) => CallTone | undefined;
  loops: ReadonlySet<string>;
  /** The camera is locked on a drone, so the calm ones stand back. */
  locked: boolean;
  trails: ReadonlyMap<string, number>;
  /** Each bay's pad on the plane. */
  padAt: ReadonlyMap<string, Point>;
  planned?: PixiSceneIn["planned"];
  touched?: ReadonlySet<string>;
  glow?: ReadonlyMap<string, Glow>;
  route?: readonly Stop[];
  /** The Job's checks, as the lights along its pad. */
  lights?: readonly Light[];
  /** The walk a drone of the Job is on, by its name. */
  walk?: string;
  /** The files the Job's drones are writing, lifted. */
  lens?: readonly Slab[];
  /** The messages between this Job and others, each an arc between two drones. */
  links?: readonly LinkIn[];
  /** The links whose packets travel: one of the most the map animates at once. */
  linkLoops?: ReadonlySet<string>;
};

/** How far beyond the field's far edge the outposts stand, and how far apart, in the layout's own units. */
export const OUTPOST_OUT = 60;
export const OUTPOST_GAP = 170;
/** How large an outpost is drawn, and how high its badge's middle stands, in the picture's own units. */
export const OUTPOST_SCALE = 2.4;
export const OUTPOST_UP = 49 * OUTPOST_SCALE;

/** Where the `i`th of `n` outposts stands: in a row across the middle of the field, past its far edge. In layout units, before the plane stretches the depth. */
export const outpostAt = (world: Size, i: number, n: number): Point => ({ x: world.width / 2 + (i - (n - 1) / 2) * OUTPOST_GAP, y: -OUTPOST_OUT });

/** The base's hangar end, where a pull request with no drone of its own starts from. */
export function baseOf(layout: Layout, world: Size, root: string): P3 {
  const t = layout.territories.find((one) => one.root === root);
  const at = onPlane(t === undefined ? { x: world.width / 2, y: world.height } : { x: t.base.x - t.base.w / 2 + 36, y: t.base.y + 8 });
  return { ...at, z: 20 };
}

/** How far to the left of its drone a pull request's route begins, at the drone's height: clear of the drone, of its call badge, which hangs over
 * it, and of its write clash marks, which run to the badge's right. */
const DOCK = -80;

/** A point `DOCK` to the right of another on the screen (a negative one to its left), which the camera's fixed turn makes a fixed step of the plane. */
const docked = (at: P3): P3 => ({ x: at.x + DOCK * Math.cos((CAM.yaw * Math.PI) / 180), y: at.y - DOCK * Math.sin((CAM.yaw * Math.PI) / 180), z: at.z });

/** How the CSS map placed a pull request, which stands on a route from its drone (or the base) to the flag, further along the nearer it is to merging. */
export function cratesOf(i: Pick<SceneInputs, "layout" | "world" | "frame" | "drones" | "prs" | "plays">): Crate[] {
  const droneBy = new Map(i.drones.map((one) => [one.key, one] as const));
  const landing = new Set(i.plays.filter((one) => one.kind === "capture" && one.number !== undefined).map((one) => one.number));
  const out: Crate[] = [];
  for (const pr of i.prs) {
    const territory = i.layout.territories.find((one) => one.root === pr.root);
    if (territory?.goal === undefined) continue;
    const owner = pr.owner === undefined ? undefined : droneBy.get(pr.owner);
    const flying = owner === undefined ? undefined : i.frame.flying(owner);
    const start: P3 = (flying === undefined ? undefined : docked(flying)) ?? { ...baseOf(i.layout, i.world, pr.root), z: ROUTE };
    const end: P3 = { ...onPlane(territory.goal), z: ROUTE };
    const landed = pr.stage === "landed";
    if (landed && !landing.has(pr.number)) continue;
    // One whose Checks have not passed waits at its drone's dock: a step along its route would put it under the badge or a clash mark.
    const waiting = flying !== undefined && (pr.stage === "checking" || pr.stage === "red");
    const rest = waiting ? start : alongOf(start, end, 34, landed ? ALONG.queued : ALONG[pr.stage]);
    const arrived = i.plays.find((one) => one.kind === "capture" && one.number === pr.number);
    const opened = i.plays.find((one) => one.kind === "payload" && one.number === pr.number);
    const hopping = landed ? arrived : opened;
    out.push({
      pr,
      start,
      end,
      at: landed ? end : rest,
      dim: owner !== undefined && !owner.shown,
      hop: hopping === undefined ? undefined : { id: hopping.id, from: landed ? rest : start, delay: hopping.delay, landing: landed },
    });
  }
  return out;
}

const tileUnder = (tiles: readonly Tile[], at: Point): Tile | undefined => tiles.find((t) => at.x >= t.rect.x && at.x <= t.rect.x + t.rect.w && at.y >= t.rect.y && at.y <= t.rect.y + t.rect.h);

/** A contrail's age to the nearest five minutes: it fades over three quarters of an hour, and a figure that moved every clock tick would redraw the map for nothing. */
const trailAge = (age: number | undefined): number | undefined => (age === undefined ? undefined : Math.floor(age / 300_000) * 300_000);

/** The drones as the canvas draws them. */
function dronesOf(i: SceneInputs): DroneSpec[] {
  const { layout, frame } = i;
  const out: DroneSpec[] = [];
  for (const drone of i.drones) {
    const placed = layout.at.get(drone.key);
    if (placed === undefined) continue;
    const at = onPlane(placed);
    const tile = tileUnder(frame.tiles, placed);
    const aimTile = frame.aimOf(drone);
    const aimed = aimTile === undefined ? undefined : onPlane(frame.aimPoint(drone, aimTile));
    const launch = i.plays.find((one) => one.key === drone.key && one.kind === "launch");
    const dock = i.plays.find((one) => one.key === drone.key && one.kind === "dock");
    const pad = drone.slot === undefined ? undefined : i.padAt.get(drone.slot);
    const sender = drone.parent === undefined ? undefined : droneBy(i, drone.parent);
    const link = sender === undefined ? undefined : frame.flying(sender);
    // A subagent lifts off the drone that sent it.
    const origin: P3 = link ?? (pad === undefined ? baseOf(layout, i.world, drone.root) : { ...pad, z: 4 });
    const on = i.selected === drone.key;
    out.push({
      drone,
      at,
      tile: tile?.path,
      alt: frame.altOf(drone),
      flying: frame.flying(drone)!,
      selected: on,
      hot: i.hot === drone.key,
      labelled: i.labels.has(drone.key),
      muted: drone.shown && i.locked && !on && drone.hue === "calm",
      away: !drone.shown,
      focused: i.focus === drone.key,
      dim: i.focus !== undefined && i.focus !== drone.key,
      call: i.callOf(drone.key),
      leaving: !i.calls.has(drone.key),
      clashes: i.clashed.get(drone.key) ?? 0,
      loop: i.loops.has(drone.key),
      calling: drone.calling,
      aim: aimTile === undefined || aimed === undefined ? undefined : { tile: aimTile.path, at: aimed, wide: Math.min(aimTile.rect.w, 150) * 0.86, deep: Math.min(aimTile.rect.h * FLAT, 100) * 0.86 },
      launch: launch === undefined ? undefined : { id: launch.id, delay: launch.delay, from: origin },
      pad,
      origin: pad === undefined ? baseOf(layout, i.world, drone.root) : { ...pad, z: 2 },
      trail: trailAge(i.trails.get(drone.key)),
      touches: drone.ghost === true ? [] : (layout.over.get(drone.key) ?? []).map(({ tile: one }) => ({ tile: one.path, at: onPlane({ x: one.cx, y: one.cy }) })),
      ghost: drone.ghost === true && drone.linked !== true,
      small: drone.parent !== undefined,
      link,
      dock: dock === undefined || link === undefined ? undefined : { id: dock.id, delay: dock.delay, to: link },
    });
  }
  return out;
}

/** The Job's pad: a light for each check spread along its front edge, and a line from each failing one to the file Fleet read from its log, where the Field draws it. */
export function padOf(layout: Layout, lights: readonly Light[]): PadSpec | undefined {
  const rect = layout.territories.find((one) => one.pad !== undefined)?.pad;
  if (rect === undefined) return undefined;
  const n = lights.length;
  const placed = lights.map((one, k) => ({ ...one, at: onPlane({ x: n === 1 ? rect.x + rect.w / 2 : rect.x + 12 + (k * (rect.w - 24)) / (n - 1), y: rect.y + rect.h - 5 }) }));
  return {
    at: onPlane({ x: rect.x + rect.w / 2, y: rect.y + rect.h / 2 }),
    w: rect.w,
    d: rect.h * FLAT,
    lights: placed,
    blame: placed.flatMap((one) => (one.state === "bad" && one.blame !== undefined && layout.bars.has(one.blame) ? [{ from: one.at, bar: one.blame, check: one.name, ...(one.line === undefined ? {} : { line: one.line }) }] : [])),
  };
}

/** The subagents round each ring. A drone stands still, so these do: each at its own place on the ring. */
function workersOf(i: SceneInputs): WorkerSpec[] {
  return i.builds.flatMap(({ build, anchor, tile }) => {
    // A landed ring is cleared a quarter hour on, and its subagents with it.
    if (build.landedAt !== undefined && build.now - build.landedAt > FRESH_LAND_MS) return [];
    return build.workers.map((label, k): WorkerSpec => {
      // One that wrote pieces sits by them; the others are spread round the ring by their place among the subagents.
      const seat = seatOf(build.tasks, label);
      return { key: `${build.key}/${label}`, detail: askedOf(label), anchor: onPlane(anchor), tile, i: k, radius: (34 + 9 * k) * i.big, phase: seat === undefined ? k * 0.27 : (seat / 360 + 1) % 1 };
    });
  });
}

/** The plots that rise when something touches them: a Check, a turn or a launch under a drone, and the wave a merge sends over its territory. */
function bumpsOf(i: SceneInputs): Map<string, Bump> {
  const bump = new Map<string, Bump>();
  for (const play of i.plays) {
    if ((play.kind === "launch" || play.kind === "sweep" || play.kind === "tracer") && play.key !== undefined) {
      for (const { tile } of i.layout.over.get(play.key) ?? []) if (!bump.has(tile.path)) bump.set(tile.path, { id: play.id, wait: play.delay + (play.kind === "launch" ? 900 : 250), amp: 8 });
    }
    if (play.kind === "capture") {
      const root = play.root ?? i.prs.find((pr) => pr.number === play.number)?.root;
      const at = root === undefined ? undefined : i.layout.territories.find((one) => one.root === root);
      if (at?.goal === undefined) continue;
      for (const tile of at.tiles) bump.set(tile.path, { id: play.id, wait: Math.round(play.delay + 900 + Math.hypot(tile.cx - at.goal.x, (tile.cy - at.goal.y) * FLAT) * 1.1), amp: 6 });
    }
  }
  return bump;
}

export function sceneOf(i: SceneInputs): PixiSceneIn {
  const lit = new Set<string>();
  for (const drone of i.drones) {
    if (!drone.shown || !drone.spin || drone.ghost === true) continue;
    for (const { tile } of i.layout.over.get(drone.key) ?? []) lit.add(tile.path);
  }
  return {
    layout: i.layout,
    world: i.world,
    hotTile: i.hotTile,
    hotPart: i.hotPart,
    names: i.names,
    workers: workersOf(i),
    plane: { width: i.world.width, height: i.world.height * FLAT },
    big: i.big,
    heights: i.frame.heights,
    lit,
    clashing: new Set(i.layout.clashes.map((one) => one.path)),
    pads: i.pads,
    goals: i.goals,
    crates: cratesOf(i),
    outposts: (i.outposts ?? []).map((one, k, all) => ({ ...one, at: onPlane(outpostAt(i.world, k, all.length)), lit: i.drones.some((drone) => drone.calling === one.service) })),
    readable: i.readable,
    builds: i.builds,
    drones: dronesOf(i),
    plays: i.plays,
    bumps: bumpsOf(i),
    orbit: i.frame.orbit,
    ...(i.planned === undefined ? {} : { planned: i.planned }),
    ...(i.touched === undefined ? {} : { touched: i.touched }),
    ...(i.glow === undefined ? {} : { glow: i.glow }),
    ...(i.route === undefined ? {} : { route: routeOf(i.layout, i.route) }),
    ...padAndWalk(i),
    ...(i.lens === undefined || i.lens.length === 0 ? {} : { lens: lensOf(i.layout, i.lens, i.big) }),
    arcs: arcsOf(i),
    clashLines: i.layout.clashes.flatMap((clash) => {
      const [a, b] = [clash.keys[0], clash.keys[1]].map((key) => (key === undefined ? undefined : droneBy(i, key)));
      const [from, to] = [a === undefined ? undefined : i.frame.flying(a), b === undefined ? undefined : i.frame.flying(b)];
      return from === undefined || to === undefined ? [] : [{ key: `${clash.root}:${clash.path}`, from, to }];
    }),
  };
}

/** How high an arc between two drones rises over the line between them: a third of how far apart they are, and never less than 60. */
export const arcLift = (from: P3, to: P3): number => Math.max(60, Math.hypot(to.x - from.x, to.y - from.y) * 0.3);

/** Each link whose two drones are both on the field, as an arc. A drone that is not here draws none. */
export function arcsOf(i: Pick<SceneInputs, "links" | "linkLoops" | "drones" | "frame">): ArcSpec[] {
  return (i.links ?? []).flatMap((link) => {
    const [a, b] = [droneBy(i, link.from), droneBy(i, link.to)];
    const [from, to] = [a === undefined ? undefined : i.frame.flying(a), b === undefined ? undefined : i.frame.flying(b)];
    if (from === undefined || to === undefined) return [];
    const lift = arcLift(from, to);
    return [{ key: link.key, other: link.other, title: link.title, quote: link.quote, sent: link.sent, from, to, lift, mid: alongOf(from, to, lift, 0.5), loop: i.linkLoops?.has(link.key) ?? false }];
  });
}

/** The pad, and the window over it while a drone of the Job walks the app: high over the pad, where the walking drone stands. */
function padAndWalk(i: SceneInputs): Pick<PixiSceneIn, "pad" | "walk"> {
  const pad = i.lights === undefined ? undefined : padOf(i.layout, i.lights);
  if (pad === undefined) return {};
  return { pad, ...(i.walk === undefined ? {} : { walk: { name: i.walk, at: { ...pad.at, z: WINDOW_UP } } }) };
}

/** How high over the pad the walk's window stands, clear of the drone under it. */
export const WINDOW_UP = 120;

/** The slab of the card's file, and of another writer's, in pixels before `big`: wide enough to read rows on at any zoom, however small the plot. */
export const SLAB_CARD = { w: 84, d: 120, up: 64 };
export const SLAB_SMALL = { w: 48, d: 68, up: 48 };

/** Each lifted file over its bar, the card's last so it is drawn over the others. A file with no bar on the floor lifts nothing. */
export function lensOf(layout: Layout, slabs: readonly Slab[], big: number): LensSpec[] {
  return [...slabs]
    .sort((a, b) => Number(a.card) - Number(b.card))
    .flatMap((slab) => {
      const bar = layout.bars.get(slab.path);
      if (bar === undefined) return [];
      const size = slab.card ? SLAB_CARD : SLAB_SMALL;
      const { rect } = bar;
      return [{ ...slab, tile: bar.tile.path, at: onPlane({ x: bar.cx, y: bar.cy }), mark: { x: rect.x, y: rect.y * FLAT, w: rect.w, h: rect.h * FLAT }, w: size.w * big, d: size.d * big, up: size.up * big }];
    });
}

/** Each stop at its file's bar. A stop whose file has no bar is left out, and the others keep their numbers. */
export function routeOf(layout: Layout, stops: readonly Stop[]): RouteSpot[] {
  return stops.flatMap((stop) => {
    const bar = layout.bars.get(stop.path);
    return bar === undefined ? [] : [{ n: stop.n, act: stop.act, at: onPlane({ x: bar.cx, y: bar.cy }), tile: bar.tile.path }];
  });
}

const droneBy = (i: Pick<SceneInputs, "drones">, key: string): DroneIn | undefined => i.drones.find((one) => one.key === key);
