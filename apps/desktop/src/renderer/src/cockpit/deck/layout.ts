// The flight deck's battlefield, as pure readings. Each repository is a territory with a base at its near edge and its
// main as the objective at the far edge; each Session is a drone placed inside its territory and kept where it
// was put. Nothing here draws or moves anything.
//
// **A drone is only ever put where something observed puts it.** The areas it has edited (`touches.ts`) place it over
// the plots of those areas that the repository draws; with none, or none drawn, it rests at the pad of the bay it holds,
// or at its repository's base when it holds no bay. Never over a directory it was guessed to be in. Fleet reports no
// directory tree yet, so a territory is drawn today as its pads and its main flag with no plots. #2301.

import type { Check, Doing } from "./doing";
import type { Areas } from "./areas";
import type { Play } from "./plays";
import { gridOf, pillRect, type PillGrid } from "./pills";
import type { Pr, Subject } from "./ship";
import type { Clash, Touch } from "./touches";

export type Rect = { x: number; y: number; w: number; h: number };
export type Size = { width: number; height: number };
export type Point = { x: number; y: number };

// ---- A drone, read off the Session it is --------------------------------------------------------

/** Grey is the baseline; amber is owed to him or stuck, red is a Check failing now. */
export type Hue = "calm" | "ask" | "bad";

export type DroneIn = {
  key: string;
  kind: "session";
  title: string;
  root: string;
  /** Its state, for the tooltip. */
  said: string;
  hue: Hue;
  /** Rotors turn only while it is working. */
  spin: boolean;
  /** Hovering on its beam, or landed on the ground. */
  hover: boolean;
  /** Holds a call: a beacon pings from it. */
  ping: boolean;
  /** Working and silent: its rotors stall and an amber ring breathes. */
  stuck: boolean;
  /** A Check is failing. */
  flare: boolean;
  /** At work: a Session whose turn is running, stuck or failing ones included. Idle and waiting drones are not. */
  working: boolean;
  /** What a working drone's latest tool event says it is doing, over its plot. Set from the timeline, not here. */
  doing?: Doing;
  /** How its Check stands, while it is running tools. */
  check?: Check;
  /** The command or path of that tool event. */
  via?: string;
  /** The service its latest call went to, while that call is fresh: the outpost a beam runs to. Set from the timeline, not here. */
  calling?: string;
  /** The file it is over, where the repository draws files as bars: it hovers at the bar and not at the middle of the plot. */
  file?: string;
  /** The directories it is changing, and how much of its change is in each. */
  touches: readonly Touch[];
  /** The bay it holds. */
  slot?: string;
  /** Inside the filters. Outside, it stays where it is and dims. */
  shown: boolean;
  /** Another Job's, drawn faded on a Job's own Field: no beam, label or action, and it raises no plot. */
  ghost?: true;
};

export function droneOf(subject: Subject, owed: boolean, extra: { slot?: string | undefined; file?: string | undefined; shown: boolean; touches?: readonly Touch[] }): DroneIn {
  const stuck = subject.kind === "session" && subject.phase === "held" && subject.alarm;
  const failing = !stuck && (subject.alarm || subject.mark.tone === "bad");
  const ask = owed || subject.needs;
  const working = subject.phase === "working";
  return {
    key: subject.key,
    kind: "session",
    title: subject.title,
    root: subject.root,
    said: subject.mark.said,
    hue: stuck ? "ask" : failing ? "bad" : ask ? "ask" : "calm",
    spin: working && !stuck && !failing,
    hover: working || stuck,
    ping: ask || failing,
    stuck,
    flare: failing,
    working,
    ...(extra.slot === undefined ? {} : { slot: extra.slot }),
    ...(extra.file === undefined ? {} : { file: extra.file }),
    shown: extra.shown,
    touches: extra.touches ?? [],
  };
}

// ---- Layout --------------------------------------------------------------------------------------------

const MARGIN = 14;

type Box = { l: number; t: number; r: number; b: number };
/** A margin kept round what a drone must stay clear of. */
const EXCLUDE = 16;
const boxOf = (l: number, t: number, r: number, b: number, pad: number): Box => ({ l: l - pad, t: t - pad, r: r + pad, b: b + pad });
/** How far above its ground point a drone is drawn, and the height and arc a pull request flies at: where each lands on the screen. */
const DRONE_UP = 50;
const ROUTE_UP = 52;
const ROUTE_ARC = 34;

/** A fixed number in [0, 1) from a key, so a tile or a drone is put the same way every time. */
export function rand(key: string, salt: number): number {
  let hash = 2166136261 ^ Math.imul(salt + 1, 40503);
  for (let i = 0; i < key.length; i += 1) hash = Math.imul(hash ^ key.charCodeAt(i), 16777619);
  hash ^= hash >>> 15;
  hash = Math.imul(hash, 2246822507);
  hash ^= hash >>> 13;
  return (hash >>> 0) / 4294967296;
}

/** Split a rect among weighted parts, along its longer side, so each part's area follows its weight. */
function slice(parts: readonly { id: string; weight: number }[], rect: Rect, into: Map<string, Rect>, gap: number): void {
  if (parts.length === 0) return;
  if (parts.length === 1) {
    into.set(parts[0]!.id, rect);
    return;
  }
  const sorted = [...parts].sort((a, b) => b.weight - a.weight || a.id.localeCompare(b.id));
  const total = sorted.reduce((sum, one) => sum + one.weight, 0);
  let take = 0;
  let sum = 0;
  while (take < sorted.length - 1 && (take === 0 || sum + sorted[take]!.weight / 2 <= total / 2)) sum += sorted[take++]!.weight;
  const first = sorted.slice(0, take);
  const rest = sorted.slice(take);
  const share = sum / total;
  if (rect.w >= rect.h) {
    const left = (rect.w - gap) * share;
    slice(first, { x: rect.x, y: rect.y, w: left, h: rect.h }, into, gap);
    slice(rest, { x: rect.x + left + gap, y: rect.y, w: rect.w - left - gap, h: rect.h }, into, gap);
  } else {
    const top = (rect.h - gap) * share;
    slice(first, { x: rect.x, y: rect.y, w: rect.w, h: top }, into, gap);
    slice(rest, { x: rect.x, y: rect.y + top + gap, w: rect.w, h: rect.h - top - gap }, into, gap);
  }
}

export type Pad = { key: string; x: number; y: number };

/** A plot of the battlefield: one directory, drawn to the area its files take. */
/** A plot, and the pills that fill it: one for each file its directory holds, where Fleet counted them. */
export type Tile = { path: string; top: string; rect: Rect; cx: number; cy: number; pills?: PillGrid };

export type Territory = {
  root: string;
  name: string;
  rect: Rect;
  /** The top directories, each the rect of its tiles, for the border that groups them. */
  groups: { dir: string; rect: Rect }[];
  tiles: Tile[];
  /** The base: the centre of its compound, and how wide it is. */
  base: { x: number; y: number; w: number };
  pads: Pad[];
  /** The objective, where this repository has a main. */
  goal: Point | undefined;
};

export type Placed = { x: number; y: number; s: number; root: string };

/** A pull request as the battlefield draws it: where it stands and which drone, if any, it belongs to. */
export type PrIn = { key: string; number: number; stage: Pr["stage"]; root: string; owner?: string };

/** What is being done to a file: made, changed or taken away. None is a file at rest. */
export type FileAct = "write" | "edit" | "remove";

/** A file of a repository, drawn as a small bar on the plot of its directory. */
export type FileIn = { path: string; act?: FileAct };

/** A file's bar as laid out: where it lies on its plot, in the layout's own units. A file with a pill is lit on it, and `pill` is which. */
export type Bar = { path: string; act: FileAct | undefined; rect: Rect; cx: number; cy: number; tile: Tile; pill?: number };

export type RepoIn = { root: string; name: string; pads: readonly string[]; goal: boolean; areas: Areas; files?: readonly FileIn[] };

/** The files a top directory is drawn for when it lists no directories inside it and Fleet has not counted. */
const BARE_FILES = 8;

/** The least a lit pill is drawn, so a file in a crowded plot can still be seen. */
const LIT = { w: 7, h: 2.6 };

export type Layout = {
  territories: Territory[];
  at: Map<string, Placed>;
  /** The files drawn as bars, by path. */
  bars: Map<string, Bar>;
  /** The plots each drone touches, the biggest share first. */
  over: Map<string, { tile: Tile; w: number }[]>;
  clashes: Clash[];
};

/** How large a drone is drawn in a field of this size: larger where there is room, smaller where there is not. */
export const bigOf = (size: Size): number => Math.min(1.55, Math.max(1.05, Math.min(size.width / 1000, size.height / 700) * 1.3));

/**
 * The territories across the whole field, sized by what they hold. Each is the repository's own directories as a
 * treemap, two levels, a plot for each directory with its area from its file count. A drone hovers over the centre
 * of the plots it touches, weighted by how much it touches each, and is held apart from the others; one with nothing
 * touched yet stays near its base pad.
 */
export function layoutOf(
  repos: readonly RepoIn[],
  drones: readonly DroneIn[],
  size: Size,
  big: number,
  prs: readonly { root: string; owner?: string | undefined; stage: Pr["stage"] }[] = [],
  clashed: readonly Clash[] = [],
): Layout {
  const rects = new Map<string, Rect>();
  const filesOf = (repo: RepoIn) => repo.areas.reduce((sum, area) => sum + (area.kids.length === 0 ? (area.files ?? BARE_FILES) : area.kids.reduce((s, k) => s + k[1], 0)), 0);
  const weights = repos.map((repo) => ({ id: repo.root, weight: 3 + drones.filter((one) => one.root === repo.root).length + 0.35 * Math.sqrt(filesOf(repo)) }));
  slice(weights, { x: MARGIN, y: MARGIN, w: Math.max(120, size.width - MARGIN * 2), h: Math.max(120, size.height - MARGIN * 2) }, rects, 14);

  const territories: Territory[] = [];
  const at = new Map<string, Placed>();
  const over = new Map<string, { tile: Tile; w: number }[]>();
  const clashes: Clash[] = [];
  const bars = new Map<string, Bar>();
  const want = new Map<string, Point>();
  const bounds = new Map<string, Rect>();

  for (const repo of repos) {
    const rect = rects.get(repo.root)!;
    const baseH = Math.min(104, rect.h * 0.2);
    const spacing = Math.max(24, Math.min(42, (rect.w - 190) / Math.max(1, repo.pads.length)));
    const rowW = spacing * Math.max(0, repo.pads.length - 1);
    const hangar = 64;
    const baseW = hangar + 24 + rowW + 40;
    const bx = rect.x + rect.w / 2;
    const by = rect.y + rect.h - baseH * 0.52;
    const left = bx - baseW / 2;
    const pads = repo.pads.map((key, i) => ({ key, x: left + hangar + 24 + i * spacing, y: by + 2 }));
    // The strip above the base where a drone with nothing touched stands, and the flag's band above the plots.
    const strip = 64 + 22 * big;
    const plots: Rect = { x: rect.x + 8, y: rect.y + 98, w: rect.w - 16, h: Math.max(60, by - strip - (rect.y + 98)) };
    const goal = repo.goal ? { x: rect.x + rect.w / 2, y: rect.y + 84 } : undefined;

    // Top directories by their files, then the directories inside each.
    const sizeOf = (area: RepoIn["areas"][number]) => (area.kids.length === 0 ? (area.files ?? BARE_FILES) : area.kids.reduce((sum, k) => sum + k[1], 0));
    const tops = new Map<string, Rect>();
    slice(repo.areas.map((area) => ({ id: area.dir, weight: sizeOf(area) })), plots, tops, 6);
    const tiles: Tile[] = [];
    const groups: Territory["groups"] = [];
    // A plot, with a pill for each file it holds: the named ones first, then those past the names Fleet gave.
    const plot = (path: string, top: string, rect: Rect, files: number, names: readonly string[]): Tile => {
      const grid = gridOf(rect, Math.max(files, names.length), names, FLAT);
      return { path, top, rect, cx: rect.x + rect.w / 2, cy: rect.y + rect.h / 2, ...(grid === undefined ? {} : { pills: grid }) };
    };
    for (const area of repo.areas) {
      const box = tops.get(area.dir);
      if (box === undefined) continue;
      groups.push({ dir: area.dir, rect: box });
      if (area.kids.length === 0) {
        tiles.push(plot(area.dir, area.dir, box, area.names === undefined ? 0 : (area.files ?? area.names.length), area.names ?? []));
        continue;
      }
      const inner = new Map<string, Rect>();
      slice(area.kids.map(([name, files]) => ({ id: name, weight: files })), { x: box.x + 3, y: box.y + 3, w: box.w - 6, h: box.h - 6 }, inner, 3);
      for (const [name, files, names] of area.kids) {
        const r = inner.get(name);
        if (r !== undefined) tiles.push(plot(`${area.dir}/${name}`, area.dir, r, files, names ?? []));
      }
    }
    // A touched path belongs to the plot with the longest path that begins it.
    const tileOf = (path: string) => tiles.filter((t) => path === t.path || path.startsWith(`${t.path}/`)).sort((a, b) => b.path.length - a.path.length)[0];

    // A file a plot has a pill for is lit on it. Any other is a bar in rows across the plot, in the order the repository lists them, each as wide as the plot allows.
    for (const tile of tiles) {
      const touched = (repo.files ?? []).filter((one) => tileOf(one.path) === tile);
      if (touched.length === 0) continue;
      const grid = tile.pills;
      const place = new Map<string, number>(grid === undefined ? [] : grid.names.map((name, i): [string, number] => [`${tile.path}/${name}`, i]));
      const mine: FileIn[] = [];
      for (const one of touched) {
        const i = place.get(one.path);
        if (grid === undefined || i === undefined) {
          mine.push(one);
          continue;
        }
        // Lit larger than a fine pill so it can be seen, about the pill's middle.
        const r = pillRect(grid, i);
        const [w, h] = [Math.max(r.w, LIT.w), Math.max(r.h, LIT.h)];
        const [cx, cy] = [r.x + r.w / 2, r.y + r.h / 2];
        bars.set(one.path, { path: one.path, act: one.act, rect: { x: cx - w / 2, y: cy - h / 2, w, h }, cx, cy, tile, pill: i });
      }
      if (mine.length === 0) continue;
      const bw = Math.max(8, Math.min(22, (tile.rect.w - 10) / 3.2));
      const bh = Math.max(3, bw * 0.34);
      const cols = Math.max(1, Math.floor((tile.rect.w - 10 + 3) / (bw + 3)));
      mine.forEach((one, i) => {
        const x = tile.rect.x + 5 + (i % cols) * (bw + 3);
        const y = tile.rect.y + 12 + Math.floor(i / cols) * (bh + 4);
        bars.set(one.path, { path: one.path, act: one.act, rect: { x, y, w: bw, h: bh }, cx: x + bw / 2, cy: y + bh / 2, tile });
      });
    }

    const own = drones.filter((one) => one.root === repo.root);
    own.forEach((one, order) => {
      const share = new Map<Tile, number>();
      for (const touch of one.touches) {
        const tile = tileOf(touch.path);
        if (tile !== undefined) share.set(tile, (share.get(tile) ?? 0) + touch.w);
      }
      const list = [...share].map(([tile, w]) => ({ tile, w })).sort((a, b) => b.w - a.w || a.tile.path.localeCompare(b.tile.path));
      over.set(one.key, list);
      const bar = one.file === undefined ? undefined : bars.get(one.file);
      if (bar !== undefined) {
        want.set(one.key, { x: bar.cx, y: bar.cy });
      } else if (list.length > 0) {
        const total = list.reduce((sum, one) => sum + one.w, 0);
        want.set(one.key, { x: list.reduce((sum, one) => sum + one.tile.cx * one.w, 0) / total, y: list.reduce((sum, one) => sum + one.tile.cy * one.w, 0) / total });
      } else {
        // Near its pad, or the base's middle, spread along the strip so two do not stand on each other.
        const pad = one.slot === undefined ? undefined : pads.find((p) => p.key === one.slot);
        want.set(one.key, { x: pad?.x ?? bx + (order % 5 - 2) * 56 * big, y: by - strip * 0.5 - 8 });
      }
      bounds.set(one.key, { x: rect.x + 30, y: plots.y + 14, w: rect.w - 60, h: by - 26 - strip * 0.2 - (plots.y + 14) });
    });
    // A clash is Fleet's, from git: it marks the plot of the area it names, where the repository draws one.
    for (const clash of clashed) if (clash.root === repo.root && tileOf(clash.path) !== undefined) clashes.push(clash);

    territories.push({ root: repo.root, name: repo.name, rect, groups, tiles, base: { x: bx, y: by, w: baseW }, pads, goal });
  }

  // Held apart by rule, not by hope: no two drones' hit boxes or label strips meet, and no drone stands over a flag, a pad, a crate or
  // a route that is not its own. Rects are in the layout's own units, a drone's lifted to where it is drawn.
  const keys = [...want.keys()].sort();
  const ghosts = new Set(drones.filter((one) => one.ghost === true).map((one) => one.key));
  const place = new Map(keys.map((key) => [key, { ...want.get(key)! }]));
  const goals = new Map(territories.flatMap((one) => (one.goal === undefined ? [] : [[one.root, one.goal] as const])));
  const homes = new Map(territories.map((one) => [one.root, { x: one.base.x - one.base.w / 2 + 36, y: one.base.y + 8 }] as const));
  const free = prs.filter((pr) => pr.stage !== "landed");
  const statics: Box[] = [
    ...territories.flatMap((one) => (one.goal === undefined ? [] : [boxOf(one.goal.x - 34, one.goal.y - 92, one.goal.x + 96, one.goal.y + 8, EXCLUDE)])),
    ...territories.flatMap((one) => one.pads.map((pad) => boxOf(pad.x - 17, pad.y - 17, pad.x + 17, pad.y + 17, EXCLUDE))),
    ...territories.map((one) => boxOf(one.base.x - one.base.w / 2, one.base.y - 30, one.base.x + one.base.w / 2, one.base.y + 40, 0)),
  ];
  /** Where a pull request's crate and route lie, for the drones that are not its own. */
  const routesOf = (): { by: string | undefined; box: Box }[] => {
    const out: { by: string | undefined; box: Box }[] = [];
    for (const pr of free) {
      const end = goals.get(pr.root);
      if (end === undefined) continue;
      const from = pr.owner === undefined ? homes.get(pr.root) : place.get(pr.owner);
      if (from === undefined) continue;
      const steps = Math.max(2, Math.round(Math.hypot(end.x - from.x, end.y - from.y) / 44));
      for (let i = 0; i <= steps; i += 1) {
        const t = i / steps;
        const x = from.x + (end.x - from.x) * t;
        const y = from.y + (end.y - from.y) * t - 0.81 * (ROUTE_UP + 4 * ROUTE_ARC * t * (1 - t));
        out.push({ by: pr.owner, box: boxOf(x - 14, y - 14, x + 14, y + 14, EXCLUDE) });
      }
    }
    return out;
  };
  const reach = (at: Point): Box => {
    const half = Math.max((20 + 12) * big, 80);
    // From the top of its name to the pool of light on the ground under it, which takes the pointer too.
    return boxOf(at.x - half, at.y - DRONE_UP - 34 * big - 46, at.x + half, at.y + 20 * big, 10);
  };
  const push = (a: Box, b: Box, mover: Point, other?: Point): void => {
    const px = Math.min(a.r - b.l, b.r - a.l);
    const py = Math.min(a.b - b.t, b.b - a.t);
    if (px <= 0 || py <= 0) return;
    const share = other === undefined ? 1 : 0.5;
    if (px < py) {
      const sign = (a.l + a.r) / 2 >= (b.l + b.r) / 2 ? 1 : -1;
      mover.x += sign * px * share;
      if (other !== undefined) other.x -= sign * px * share;
    } else {
      const sign = (a.t + a.b) / 2 >= (b.t + b.b) / 2 ? 1 : -1;
      mover.y += sign * py * share;
      if (other !== undefined) other.y -= sign * py * share;
    }
  };
  for (let pass = 0; pass < 160; pass += 1) {
    const routes = routesOf();
    for (let i = 0; i < keys.length; i += 1) {
      const a = place.get(keys[i]!)!;
      for (let j = i + 1; j < keys.length; j += 1) {
        const b = place.get(keys[j]!)!;
        if (Math.abs(a.x - b.x) < 1 && Math.abs(a.y - b.y) < 1) a.x += (rand(keys[i]! + keys[j]!, 3) - 0.5) * 6;
        // A faded drone gives way to a Job's own, which stays over its file.
        const [ga, gb] = [ghosts.has(keys[i]!), ghosts.has(keys[j]!)];
        if (ga === gb) push(reach(a), reach(b), a, b);
        else if (ga) push(reach(a), reach(b), a);
        else push(reach(b), reach(a), b);
      }
    }
    // What is fixed is settled last, so a drone is moved off a flag before it is moved against another drone.
    for (const key of keys) {
      const a = place.get(key)!;
      for (const box of statics) push(reach(a), box, a);
      for (const route of routes) if (route.by !== key) push(reach(a), route.box, a);
    }
    for (const key of keys) {
      const p = place.get(key)!;
      const box = bounds.get(key)!;
      p.x = Math.min(box.x + box.w, Math.max(box.x, p.x));
      p.y = Math.min(box.y + box.h, Math.max(box.y, p.y));
    }
  }
  for (const key of keys) {
    const p = place.get(key)!;
    const root = drones.find((one) => one.key === key)!.root;
    at.set(key, { x: p.x, y: p.y, s: 1, root });
  }
  return { territories, at, bars, over, clashes };
}

// ---- The camera, the ground's height and the curves above it --------------------------------------------

/** The camera in degrees: its pitch over the whole field and closer to a drone, and a slight yaw. `fit` trims how far the ground is stretched toward the viewer. */
export const CAM = { wide: 54, near: 60, yaw: 5, fit: 0.94 };

/** The layout is drawn as if seen from the viewer; the ground is tilted away, so it is stretched by this to read the same height. */
export const FLAT = CAM.fit / Math.cos((CAM.wide * Math.PI) / 180);

/** A point on the ground plane, and how high above it. */
export type P3 = { x: number; y: number; z: number };

/** The point `t` of the way from a to b, rising `lift` at the middle. */
export function alongOf(a: P3, b: P3, lift: number, t: number): P3 {
  return { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t, z: a.z + (b.z - a.z) * t + 4 * lift * t * (1 - t) };
}

/** How tall each plot stands: low where it is quiet, taller where drones work over it and where a Check or a turn has just touched it, the tallest where two clash. */
export const SLAB = { low: 3, high: 30 };

export function heightsOf(layout: Layout, drones: readonly DroneIn[], plays: readonly Play[]): Map<string, number> {
  const busy = new Map<string, number>();
  const add = (path: string, amount: number) => busy.set(path, (busy.get(path) ?? 0) + amount);
  for (const drone of drones) {
    if (!drone.shown || drone.ghost === true) continue;
    const list = layout.over.get(drone.key) ?? [];
    const total = list.reduce((sum, one) => sum + one.w, 0) || 1;
    const state = drone.spin ? 1 : drone.hover ? 0.6 : 0.2;
    for (const { tile, w } of list) add(tile.path, state * (0.4 + 0.6 * (w / total)));
    // The plot a drone is writing stands taller while it does.
    const writing = drone.doing === "write" ? list[0] : undefined;
    if (writing !== undefined) add(writing.tile.path, 0.5);
  }
  for (const play of plays) {
    if (play.key === undefined || (play.kind !== "sweep" && play.kind !== "tracer" && play.kind !== "launch")) continue;
    for (const { tile } of layout.over.get(play.key) ?? []) add(tile.path, 0.35);
  }
  const out = new Map<string, number>();
  let tallest = 0;
  for (const territory of layout.territories) {
    for (const tile of territory.tiles) {
      const height = SLAB.low + 12 * Math.min(busy.get(tile.path) ?? 0, 2.2);
      out.set(tile.path, height);
      tallest = Math.max(tallest, height);
    }
  }
  for (const clash of layout.clashes) out.set(clash.path, Math.min(SLAB.high, Math.max(tallest + 4, 24)));
  return out;
}

