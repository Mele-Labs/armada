// The flight deck's battlefield, as pure readings. Each repository is a territory with a base at its near edge
// and its main as the objective at the far edge; each Job and Session is a drone placed loosely inside its
// territory and kept where it was put. The fleet's events become scene plays: what happened, to which drone,
// and which motion tells it. Nothing here draws or moves anything. Mock only. proto/cockpit-flight-deck.

import type { TimelineEvent, TimelineKind } from "@armada/protocol";

import type { Check, Doing } from "./deck-doing";
import type { Pr, Subject } from "./ship";

export type Rect = { x: number; y: number; w: number; h: number };
export type Size = { width: number; height: number };
export type Point = { x: number; y: number };

// ---- A drone, read off the Job or Session it is --------------------------------------------------------

/** Grey is the baseline; amber is owed to him or stuck, red is a Check failing now. */
export type Hue = "calm" | "ask" | "bad";

export type DroneIn = {
  key: string;
  kind: "job" | "session";
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
  /** At work: a Job or Session whose turn is running, stuck or failing ones included. Idle and waiting drones are not. */
  working: boolean;
  /** What a working drone's latest tool event says it is doing, over its plot. Set from the timeline, not here. */
  doing?: Doing;
  /** How its Check stands, while it is running tools. */
  check?: Check;
  /** The command or path of that tool event. */
  via?: string;
  /** The Job it belongs to or was dispatched by. */
  parent?: string;
  /** The directories it is changing, and how much of its change is in each. */
  touches: readonly { path: string; w: number }[];
  /** The bay it holds. */
  slot?: string;
  /** Inside the filters. Outside, it stays where it is and dims. */
  shown: boolean;
};

export function droneOf(subject: Subject, owed: boolean, extra: { parent?: string | undefined; slot?: string | undefined; shown: boolean; touches?: readonly { path: string; w: number }[] }): DroneIn {
  const stuck = subject.kind === "session" && subject.phase === "held" && subject.alarm;
  const failing = !stuck && (subject.alarm || subject.mark.tone === "bad");
  const ask = owed || subject.needs;
  const working = subject.phase === "working";
  return {
    key: subject.key,
    kind: subject.kind === "job" ? "job" : "session",
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
    ...(extra.parent === undefined ? {} : { parent: extra.parent }),
    ...(extra.slot === undefined ? {} : { slot: extra.slot }),
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
export type Tile = { path: string; top: string; rect: Rect; cx: number; cy: number };

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

export type RepoIn = { root: string; name: string; pads: readonly string[]; goal: boolean; tree: readonly { dir: string; kids: readonly (readonly [string, number])[] }[]; bare: Readonly<Record<string, number>> };

/** Two or more drones changing the same plot: the write clash. */
export type Clash = { path: string; root: string; keys: string[] };

export type Layout = {
  territories: Territory[];
  at: Map<string, Placed>;
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
export function layoutOf(repos: readonly RepoIn[], drones: readonly DroneIn[], size: Size, big: number, prs: readonly { root: string; owner?: string | undefined; stage: Pr["stage"] }[] = []): Layout {
  const rects = new Map<string, Rect>();
  const filesOf = (repo: RepoIn) => repo.tree.reduce((sum, area) => sum + (area.kids.length === 0 ? (repo.bare[area.dir] ?? 8) : area.kids.reduce((s, k) => s + k[1], 0)), 0);
  const weights = repos.map((repo) => ({ id: repo.root, weight: 3 + drones.filter((one) => one.root === repo.root).length + 0.35 * Math.sqrt(filesOf(repo)) }));
  slice(weights, { x: MARGIN, y: MARGIN, w: Math.max(120, size.width - MARGIN * 2), h: Math.max(120, size.height - MARGIN * 2) }, rects, 14);

  const territories: Territory[] = [];
  const at = new Map<string, Placed>();
  const over = new Map<string, { tile: Tile; w: number }[]>();
  const clashes: Clash[] = [];
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
    const sizeOf = (area: RepoIn["tree"][number]) => (area.kids.length === 0 ? (repo.bare[area.dir] ?? 8) : area.kids.reduce((sum, k) => sum + k[1], 0));
    const tops = new Map<string, Rect>();
    slice(repo.tree.map((area) => ({ id: area.dir, weight: sizeOf(area) })), plots, tops, 6);
    const tiles: Tile[] = [];
    const groups: Territory["groups"] = [];
    for (const area of repo.tree) {
      const box = tops.get(area.dir);
      if (box === undefined) continue;
      groups.push({ dir: area.dir, rect: box });
      if (area.kids.length === 0) {
        tiles.push({ path: area.dir, top: area.dir, rect: box, cx: box.x + box.w / 2, cy: box.y + box.h / 2 });
        continue;
      }
      const inner = new Map<string, Rect>();
      slice(area.kids.map(([name, files]) => ({ id: name, weight: files })), { x: box.x + 3, y: box.y + 3, w: box.w - 6, h: box.h - 6 }, inner, 3);
      for (const [name] of area.kids) {
        const r = inner.get(name);
        if (r !== undefined) tiles.push({ path: `${area.dir}/${name}`, top: area.dir, rect: r, cx: r.x + r.w / 2, cy: r.y + r.h / 2 });
      }
    }
    // A touched path belongs to the plot with the longest path that begins it.
    const tileOf = (path: string) => tiles.filter((t) => path === t.path || path.startsWith(`${t.path}/`)).sort((a, b) => b.path.length - a.path.length)[0];

    const own = drones.filter((one) => one.root === repo.root);
    const byTile = new Map<string, string[]>();
    own.forEach((one, order) => {
      const share = new Map<Tile, number>();
      for (const touch of one.touches) {
        const tile = tileOf(touch.path);
        if (tile !== undefined) share.set(tile, (share.get(tile) ?? 0) + touch.w);
      }
      const list = [...share].map(([tile, w]) => ({ tile, w })).sort((a, b) => b.w - a.w || a.tile.path.localeCompare(b.tile.path));
      over.set(one.key, list);
      for (const { tile } of list) byTile.set(tile.path, [...(byTile.get(tile.path) ?? []), one.key]);
      if (list.length > 0) {
        const total = list.reduce((sum, one) => sum + one.w, 0);
        want.set(one.key, { x: list.reduce((sum, one) => sum + one.tile.cx * one.w, 0) / total, y: list.reduce((sum, one) => sum + one.tile.cy * one.w, 0) / total });
      } else {
        // Near its pad, or the base's middle, spread along the strip so two do not stand on each other.
        const pad = one.slot === undefined ? undefined : pads.find((p) => p.key === one.slot);
        want.set(one.key, { x: pad?.x ?? bx + (order % 5 - 2) * 56 * big, y: by - strip * 0.5 - 8 });
      }
      bounds.set(one.key, { x: rect.x + 30, y: plots.y + 14, w: rect.w - 60, h: by - 26 - strip * 0.2 - (plots.y + 14) });
    });
    for (const [path, keys] of byTile) if (keys.length > 1) clashes.push({ path, root: repo.root, keys });

    territories.push({ root: repo.root, name: repo.name, rect, groups, tiles, base: { x: bx, y: by, w: baseW }, pads, goal });
  }

  // Held apart by rule, not by hope: no two drones' hit boxes or label strips meet, and no drone stands over a flag, a pad, a crate or
  // a route that is not its own. Rects are in the layout's own units, a drone's lifted to where it is drawn.
  const keys = [...want.keys()].sort();
  const place = new Map(keys.map((key) => [key, { ...want.get(key)! }]));
  const owner = new Map(drones.map((one) => [one.key, one] as const));
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
  const reach = (key: string, at: Point): Box => {
    const job = owner.get(key)?.kind === "job";
    const half = Math.max(((job ? 26 : 20) + 12) * big, 80);
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
        push(reach(keys[i]!, a), reach(keys[j]!, b), a, b);
      }
    }
    // What is fixed is settled last, so a drone is moved off a flag before it is moved against another drone.
    for (const key of keys) {
      const a = place.get(key)!;
      for (const box of statics) push(reach(key, a), box, a);
      for (const route of routes) if (route.by !== key) push(reach(key, a), route.box, a);
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
  return { territories, at, over, clashes };
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
    if (!drone.shown) continue;
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

// ---- Pull requests as payloads -------------------------------------------------------------------------

/** How far along its route to the objective a pull request has got, by where it stands. */
export const ALONG: Record<Pr["stage"], number> = { checking: 0.22, red: 0.22, ready: 0.55, queued: 0.74, landed: 1 };

export const STAGE_SAID: Record<Pr["stage"], string> = { checking: "Checks running", red: "Checks failed", ready: "Ready", queued: "In the queue", landed: "Merged" };

// ---- The activity log as motion ------------------------------------------------------------------------

/**
 * What an event looks like on the field.
 *  launch   a drone lifts from its pad to its place, a contrail behind it
 *  sweep    a radar cone turns on the drone that is running a Check
 *  payload  a pull request leaves its drone for the objective
 *  capture  the objective flashes: a merge landed
 *  tracer   a short light runs between a Session and its Job
 *  blip     one ring goes out from the drone: something was said or decided
 *  flare    the objective's beacon flares: main went red or green
 */
export type PlayKind = "launch" | "sweep" | "payload" | "capture" | "tracer" | "blip" | "flare";

export type Play = {
  id: string;
  kind: PlayKind;
  /** The drone it happens to, where it is one. */
  key?: string;
  root?: string;
  /** The pull request, where it is one's. */
  number?: number;
  at: number;
  delay: number;
};

export const PLAYS: Partial<Record<TimelineKind, PlayKind>> = {
  "job.created": "launch",
  "session.started": "launch",
  "slot.leased": "launch",
  "job.check": "sweep",
  "job.judged": "sweep",
  "pr.checks": "sweep",
  "pr.opened": "payload",
  "pr.queued": "payload",
  "pr.merged": "capture",
  "job.landed": "capture",
  "session.turn": "tracer",
  "session.answered": "tracer",
  "session.asked": "blip",
  "job.asked": "blip",
  "job.answered": "blip",
  "job.step": "blip",
  "job.failed": "blip",
  "main.red": "flare",
  "main.green": "flare",
};

/** Events this new play when the field is drawn, the rest only leave their trail. */
export const RECENT_MS = 6 * 60_000;
/** How long a launch leaves its contrail on the ground. */
export const TRAIL_MS = 45 * 60_000;
/** The most that move at once. */
export const MOST_PLAYING = 12;

/** The key of the drone an event happened to, through the pull request's owner where it is one's. */
export function droneOfEvent(event: TimelineEvent, ownerOf: (number: number) => string | undefined): string | undefined {
  const { subject } = event;
  if (subject.kind === "job") return `job:${subject.id}`;
  if (subject.kind === "session") return `session:${subject.id}`;
  if (subject.kind === "pull_request") return ownerOf(subject.number);
  return undefined;
}

/** One event as the play it is, or nothing where it only leaves a trail. A replay and a focused row both play events this way. */
export function playOfEvent(event: TimelineEvent, id: string, ownerOf: (number: number) => string | undefined, rootOf: (manifest: string | undefined) => string | undefined): Play | undefined {
  const kind = PLAYS[event.kind];
  if (kind === undefined) return undefined;
  const key = droneOfEvent(event, ownerOf);
  const root = rootOf(event.manifest_id);
  const number = event.subject.kind === "pull_request" ? event.subject.number : undefined;
  return { id, kind, at: Date.now(), delay: 0, ...(key === undefined ? {} : { key }), ...(root === undefined ? {} : { root }), ...(number === undefined ? {} : { number }) };
}

/** The events recent enough to move, newest first, as plays. */
export function playsOf(events: readonly TimelineEvent[], now: number, ownerOf: (number: number) => string | undefined, rootOf: (manifest: string | undefined) => string | undefined): Play[] {
  return events
    .flatMap((event): Play[] => {
      const kind = PLAYS[event.kind];
      const at = Date.parse(event.at);
      if (kind === undefined || now - at > RECENT_MS || at - now > 60_000) return [];
      const key = droneOfEvent(event, ownerOf);
      const root = rootOf(event.manifest_id);
      const number = event.subject.kind === "pull_request" ? event.subject.number : undefined;
      return [{ id: `event:${event.id}`, kind, at, delay: 0, ...(key === undefined ? {} : { key }), ...(root === undefined ? {} : { root }), ...(number === undefined ? {} : { number }) }];
    })
    .sort((a, b) => b.at - a.at)
    .slice(0, MOST_PLAYING)
    .map((play, i) => ({ ...play, delay: i * 220 }));
}

/** When a drone was last dispatched, from the events, for the contrail it left. Absent where there is none or it is long gone. */
export function trailsOf(events: readonly TimelineEvent[], now: number, ownerOf: (number: number) => string | undefined): Map<string, number> {
  const trails = new Map<string, number>();
  for (const event of events) {
    if (event.kind !== "job.created" && event.kind !== "job.started" && event.kind !== "session.started" && event.kind !== "slot.leased") continue;
    const key = droneOfEvent(event, ownerOf);
    const age = now - Date.parse(event.at);
    if (key === undefined || age < 0 || age > TRAIL_MS) continue;
    trails.set(key, Math.min(trails.get(key) ?? Infinity, age));
  }
  return trails;
}
