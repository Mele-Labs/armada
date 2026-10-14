// The flight deck's battlefield, as pure readings. Each repository is a territory with a base at its near edge
// and its main as the objective at the far edge; each Job and Session is a drone placed loosely inside its
// territory and kept where it was put. The fleet's events become scene plays: what happened, to which drone,
// and which motion tells it. Nothing here draws or moves anything. Mock only. proto/cockpit-flight-deck.

import type { TimelineEvent, TimelineKind } from "@armada/protocol";

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
    ...(extra.parent === undefined ? {} : { parent: extra.parent }),
    ...(extra.slot === undefined ? {} : { slot: extra.slot }),
    shown: extra.shown,
    touches: extra.touches ?? [],
  };
}

// ---- Layout --------------------------------------------------------------------------------------------

const MARGIN = 14;

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
export function layoutOf(repos: readonly RepoIn[], drones: readonly DroneIn[], size: Size, big: number): Layout {
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
    const goal = repo.goal ? { x: rect.x + rect.w * (0.5 + (rand(repo.root, 7) - 0.5) * 0.3), y: rect.y + 84 } : undefined;

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

  // Held apart: drones over the same plot, or close, are pushed from each other, a few passes, the same way every time.
  const keys = [...want.keys()].sort();
  const place = new Map(keys.map((key) => [key, { ...want.get(key)! }]));
  const apart = 64 * big;
  for (let pass = 0; pass < 24; pass += 1) {
    for (let i = 0; i < keys.length; i += 1) {
      for (let j = i + 1; j < keys.length; j += 1) {
        const a = place.get(keys[i]!)!;
        const b = place.get(keys[j]!)!;
        const dx = (b.x - a.x) / 1.5;
        const dy = b.y - a.y;
        const gap = Math.hypot(dx, dy);
        if (gap >= apart) continue;
        const turn = rand(keys[i]! + keys[j]!, 3) * Math.PI * 2;
        const ux = gap < 1 ? Math.cos(turn) : dx / gap;
        const uy = gap < 1 ? Math.sin(turn) : dy / gap;
        const push = (apart - gap) / 2;
        a.x -= ux * push * 1.5;
        a.y -= uy * push;
        b.x += ux * push * 1.5;
        b.y += uy * push;
      }
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
    // Nearer the bottom of the field is nearer the viewer: a little larger.
    at.set(key, { x: p.x, y: p.y, s: 0.84 + 0.3 * Math.min(1, Math.max(0, p.y / size.height)), root });
  }
  return { territories, at, over, clashes };
}

/** A camera over the field: the translation that puts `aim` in the middle at zoom `z`, kept inside the field. */
export function cameraOf(size: Size, z: number, aim: Point): { x: number; y: number } {
  const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));
  return { x: clamp(size.width / 2 - aim.x * z, size.width - size.width * z, 0), y: clamp(size.height / 2 - aim.y * z, size.height - size.height * z, 0) };
}

/** The bend of a gentle arc between two points, for a line that is not to read as a ruler. */
function bendOf(a: Point, b: Point, bow: number): Point {
  return { x: (a.x + b.x) / 2 - (b.y - a.y) * bow, y: (a.y + b.y) / 2 + (b.x - a.x) * bow - Math.abs(b.x - a.x) * 0.08 };
}

export function arcOf(a: Point, b: Point, bow = 0.14): string {
  const c = bendOf(a, b, bow);
  return `M${a.x.toFixed(1)},${a.y.toFixed(1)} Q${c.x.toFixed(1)},${c.y.toFixed(1)} ${b.x.toFixed(1)},${b.y.toFixed(1)}`;
}

/** The point `t` of the way along `arcOf(a, b, bow)`. */
export function arcAt(a: Point, b: Point, t: number, bow = 0.14): Point {
  const c = bendOf(a, b, bow);
  const u = 1 - t;
  return { x: u * u * a.x + 2 * u * t * c.x + t * t * b.x, y: u * u * a.y + 2 * u * t * c.y + t * t * b.y };
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

const PLAYS: Partial<Record<TimelineKind, PlayKind>> = {
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
