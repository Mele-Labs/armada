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
  /** The bay it holds. */
  slot?: string;
  /** Inside the filters. Outside, it stays where it is and dims. */
  shown: boolean;
};

export function droneOf(subject: Subject, owed: boolean, extra: { parent?: string | undefined; slot?: string | undefined; shown: boolean }): DroneIn {
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
  };
}

// ---- Layout --------------------------------------------------------------------------------------------

const MARGIN = 14;
const GAP = 14;

/** A fixed number in [0, 1) from a key, so a drone is put the same way every time. */
export function rand(key: string, salt: number): number {
  let hash = 2166136261 ^ Math.imul(salt + 1, 40503);
  for (let i = 0; i < key.length; i += 1) hash = Math.imul(hash ^ key.charCodeAt(i), 16777619);
  hash ^= hash >>> 15;
  hash = Math.imul(hash, 2246822507);
  hash ^= hash >>> 13;
  return (hash >>> 0) / 4294967296;
}

/** Split a rect among weighted parts, along its longer side, so each part's area follows its weight. */
function slice(parts: readonly { id: string; weight: number }[], rect: Rect, into: Map<string, Rect>): void {
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
    const left = (rect.w - GAP) * share;
    slice(first, { x: rect.x, y: rect.y, w: left, h: rect.h }, into);
    slice(rest, { x: rect.x + left + GAP, y: rect.y, w: rect.w - left - GAP, h: rect.h }, into);
  } else {
    const top = (rect.h - GAP) * share;
    slice(first, { x: rect.x, y: rect.y, w: rect.w, h: top }, into);
    slice(rest, { x: rect.x, y: rect.y + top + GAP, w: rect.w, h: rect.h - top - GAP }, into);
  }
}

/** A closed smooth path through points: Catmull-Rom turned into Béziers. */
function smooth(points: readonly Point[]): string {
  const n = points.length;
  const at = (i: number) => points[(i + n) % n]!;
  let d = `M${at(0).x.toFixed(1)},${at(0).y.toFixed(1)}`;
  for (let i = 0; i < n; i += 1) {
    const p0 = at(i - 1);
    const p1 = at(i);
    const p2 = at(i + 1);
    const p3 = at(i + 2);
    d += ` C${(p1.x + (p2.x - p0.x) / 6).toFixed(1)},${(p1.y + (p2.y - p0.y) / 6).toFixed(1)} ${(p2.x - (p3.x - p1.x) / 6).toFixed(1)},${(p2.y - (p3.y - p1.y) / 6).toFixed(1)} ${p2.x.toFixed(1)},${p2.y.toFixed(1)}`;
  }
  return `${d}Z`;
}

/** A ground patch that fills a rect the way a country does: a rounded square with a ragged edge. */
function blobOf(rect: Rect, seed: string, wobble: number): string {
  const cx = rect.x + rect.w / 2;
  const cy = rect.y + rect.h / 2;
  const points = Array.from({ length: 18 }, (_, i) => {
    const turn = (i / 18) * Math.PI * 2;
    const c = Math.cos(turn);
    const s = Math.sin(turn);
    // A superellipse, so the corners are taken up, not left empty.
    const reach = 1 + (rand(seed, i) - 0.5) * 2 * wobble;
    return { x: cx + Math.sign(c) * Math.abs(c) ** (2 / 3.4) * (rect.w / 2) * reach, y: cy + Math.sign(s) * Math.abs(s) ** (2 / 3.4) * (rect.h / 2) * reach };
  });
  return smooth(points);
}

export type Pad = { key: string; x: number; y: number };

export type Territory = {
  root: string;
  name: string;
  rect: Rect;
  blob: string;
  contours: string[];
  /** Craters and scrub: the ground's own marks, as ellipses and one path of short ticks. */
  craters: { x: number; y: number; r: number }[];
  scrub: string;
  /** The base: the centre of its compound, and how wide it is. */
  base: { x: number; y: number; w: number };
  pads: Pad[];
  /** The objective, where this repository has a main. */
  goal: Point | undefined;
  /** Where a drone may be put, in ground coordinates. */
  area: Rect;
};

export type Placed = { x: number; y: number; s: number; root: string };

export type RepoIn = { root: string; name: string; pads: readonly string[]; goal: boolean };

/** How large a drone is drawn in a field of this size: larger where there is room, smaller where there is not. */
export const bigOf = (size: Size): number => Math.min(1.55, Math.max(1.05, Math.min(size.width / 1000, size.height / 700) * 1.3));

/** Unit positions inside a territory's area, kept by drone so it stays where it was put when others come and go. */
export type Memory = Map<string, { u: number; v: number }>;

export type Layout = { territories: Territory[]; at: Map<string, Placed> };

const DRONE_W = 1.25;

/**
 * The territories across the whole field, sized by what they hold, and every drone put loosely inside its own.
 * Each new drone takes the best of a handful of fixed candidates, the one furthest from the rest, and a Session
 * looks first near its Job. A drone already put keeps its place, so a later arrival moves nothing.
 */
export function layoutOf(repos: readonly RepoIn[], drones: readonly DroneIn[], size: Size, memory: Memory, big: number): Layout {
  const rects = new Map<string, Rect>();
  const weights = repos.map((repo) => ({ id: repo.root, weight: drones.filter((one) => one.root === repo.root).length + 2.5 }));
  if (weights.length > 0) slice(weights, { x: MARGIN, y: MARGIN, w: Math.max(120, size.width - MARGIN * 2), h: Math.max(120, size.height - MARGIN * 2) }, rects);

  const territories: Territory[] = [];
  const at = new Map<string, Placed>();
  for (const repo of repos) {
    const rect = rects.get(repo.root)!;
    const objectiveH = Math.min(70, rect.h * 0.13);
    const baseH = Math.min(104, rect.h * 0.2);
    const spacing = Math.max(24, Math.min(42, (rect.w - 190) / Math.max(1, repo.pads.length)));
    const rowW = spacing * Math.max(0, repo.pads.length - 1);
    const hangar = 64;
    const baseW = hangar + 24 + rowW + 40;
    const bx = rect.x + rect.w / 2;
    const by = rect.y + rect.h - baseH * 0.52;
    const left = bx - baseW / 2;
    const pads = repo.pads.map((key, i) => ({ key, x: left + hangar + 24 + i * spacing, y: by + 2 }));
    // Ground a drone may stand on: room above for its body and beam, room below for its label, and the base clear.
    const top = rect.y + 34 + 56 * big;
    const bottom = by - 34 - 20 - 12 * big;
    const area: Rect = { x: rect.x + 28 + 22 * big, y: top, w: Math.max(60, rect.w - 56 - 44 * big), h: Math.max(60, bottom - top) };
    const goal = repo.goal ? { x: rect.x + rect.w * (0.5 + (rand(repo.root, 7) - 0.5) * 0.3), y: rect.y + objectiveH * 0.9 + 34 } : undefined;

    const own = drones.filter((one) => one.root === repo.root);
    const placed: { key: string; x: number; y: number }[] = goal === undefined ? [] : [{ key: "goal", x: goal.x, y: goal.y + 26 }];
    const px = (u: number, v: number): Point => ({ x: area.x + u * area.w, y: area.y + v * area.h });
    for (const one of own) {
      const known = memory.get(one.key);
      if (known !== undefined) placed.push({ key: one.key, ...px(known.u, known.v) });
    }
    // Jobs first, so a Session can look for room beside its own.
    const fresh = own.filter((one) => !memory.has(one.key)).sort((a, b) => (a.kind === b.kind ? 0 : a.kind === "job" ? -1 : 1));
    for (const one of fresh) {
      const parent = one.parent === undefined ? undefined : placed.find((p) => p.key === one.parent);
      const candidates: Point[] = Array.from({ length: 56 }, (_, i) => {
        if (parent !== undefined && i < 36) {
          const turn = rand(one.key, i * 2) * Math.PI * 2;
          const reach = (80 + rand(one.key, i * 2 + 1) * 60) * big;
          return { x: Math.min(area.x + area.w, Math.max(area.x, parent.x + Math.cos(turn) * reach * 1.15)), y: Math.min(area.y + area.h, Math.max(area.y, parent.y + Math.sin(turn) * reach * 0.62)) };
        }
        return px(rand(one.key, i * 2 + 100), rand(one.key, i * 2 + 101));
      });
      let best = candidates[0]!;
      let most = -Infinity;
      for (const candidate of candidates) {
        let room = Infinity;
        for (const other of placed) room = Math.min(room, Math.hypot((candidate.x - other.x) / DRONE_W, candidate.y - other.y));
        // Keep off the edges of the area a little, and keep near the parent when it has one.
        room = Math.min(room, 40 * big + Math.min(candidate.x - area.x, area.x + area.w - candidate.x, (candidate.y - area.y) * 1.6, (area.y + area.h - candidate.y) * 1.6));
        if (parent !== undefined) room -= Math.max(0, Math.hypot(candidate.x - parent.x, candidate.y - parent.y) - 150 * big) * 0.6;
        if (room > most) {
          most = room;
          best = candidate;
        }
      }
      memory.set(one.key, { u: (best.x - area.x) / area.w, v: (best.y - area.y) / area.h });
      placed.push({ key: one.key, ...best });
    }
    for (const one of own) {
      const known = memory.get(one.key)!;
      const point = px(known.u, known.v);
      // Nearer the bottom of the field is nearer the viewer: a little larger.
      at.set(one.key, { ...point, s: 0.84 + 0.3 * Math.min(1, Math.max(0, point.y / size.height)), root: repo.root });
    }

    const contours = [0.84, 0.68, 0.52].map((shrink, i) =>
      blobOf({ x: rect.x + rect.w * (1 - shrink) * (0.35 + rand(repo.root, 40 + i) * 0.5), y: rect.y + rect.h * (1 - shrink) * (0.3 + rand(repo.root, 50 + i) * 0.4), w: rect.w * shrink, h: rect.h * shrink }, `${repo.root}:${i}`, 0.07),
    );
    const craters = Array.from({ length: 6 }, (_, i) => ({ x: rect.x + rect.w * (0.12 + rand(repo.root, 60 + i) * 0.76), y: rect.y + rect.h * (0.16 + rand(repo.root, 70 + i) * 0.6), r: 9 + rand(repo.root, 80 + i) * 16 }));
    const scrub = Array.from({ length: 34 }, (_, i) => {
      const x = rect.x + rect.w * (0.08 + rand(repo.root, 200 + i) * 0.84);
      const y = rect.y + rect.h * (0.1 + rand(repo.root, 300 + i) * 0.74);
      return `M${x.toFixed(1)},${y.toFixed(1)} l${(3 + rand(repo.root, 400 + i) * 4).toFixed(1)},${(rand(repo.root, 500 + i) * 2 - 1).toFixed(1)}`;
    }).join(" ");
    territories.push({
      root: repo.root,
      name: repo.name,
      rect,
      blob: blobOf(rect, repo.root, 0.025),
      contours,
      craters,
      scrub,
      base: { x: bx, y: by, w: baseW },
      pads,
      goal,
      area,
    });
  }
  return { territories, at };
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
