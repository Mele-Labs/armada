// A Job's mission: the battlefield's world cut down to one Job. Its workflow is a dotted route of waypoints across a small plane, a
// pin for each step, the plots it changes as slabs beside the route, the Job's drone over the step at work, its Sessions and Drones
// near the steps they serve, a base pad with the held bay at the start and the pull request waiting at the end. Pure readings:
// the steps come from the same run the Job page's canvas draws (`workflowRunOf`), the plots from the areas the drones' observed
// edits name (`touches.ts`), so a Job with none observed has a route and no plots, and nothing here draws. #2301.

import type { WorkflowStepCardProps } from "@armada/components";
import type { JobDetail } from "@armada/protocol";
import { stepThatWorksTheGroups, workflowRunOf } from "@armada/jobs";
import type { DroneView } from "@armada/jobs/draft/drone";
import { taskGroupsOf, type GroupState } from "@armada/jobs/draft/group";

import type { DroneIn, Layout, Placed, Point, Size, Territory, Tile } from "./layout";
import { FLAT } from "./layout";
import type { Clash } from "./touches";

/** Where a waypoint stands: behind the Job, at it, ahead of it, broken, or waiting on a person. */
export type PinState = "done" | "live" | "next" | "failed" | "ask";

export type Waypoint = {
  id: string;
  label: string;
  state: PinState;
  /** The state in words, for the tooltip. */
  said: string;
  /** The Job is at it: its drone hovers over the pin. */
  here: boolean;
  /** The card the Workflow tab draws for it, for the callout a press opens. */
  card: WorkflowStepCardProps;
};

export type Mission = {
  spine: readonly Waypoint[];
  /** The groups a step fans out into, by that step's id: they fork from it and rejoin at the next. */
  fans: ReadonlyMap<string, readonly Waypoint[]>;
  /** A step whose gate sends the work back to an earlier one. */
  returns: readonly { from: string; to: string }[];
  /** The waypoint the Job's drone hovers over. */
  live: string | undefined;
};

const stateOf = (card: WorkflowStepCardProps): PinState => {
  if (card.activity === "stopped" || card.activity === "killed" || card.activity === "failed" || (card.needs ?? []).some((one) => one.tone === "failed")) return "failed";
  if (card.activity === "awaiting_human" || (card.needs ?? []).some((one) => one.tone === "waiting")) return "ask";
  if (card.activity === "advanced") return "done";
  return card.current === true || card.activity === "running" || card.activity === "retrying" ? "live" : "next";
};

const GROUP_ACTIVITY: Record<GroupState, WorkflowStepCardProps["activity"]> = {
  pending: "not_started",
  running: "running",
  joining: "running",
  checking: "running",
  retrying: "running",
  passed: "advanced",
  landed: "advanced",
  failed: "failed",
};

/** The Job's run as waypoints, from the workflow it froze and the groups of its plan. `begun` false is the Job before its first step: every pin ahead. */
export function missionOf(whole: JobDetail, drones: readonly DroneView[], begun: boolean): Mission {
  const groups = taskGroupsOf(whole);
  const run = workflowRunOf({ whole, groups, drones });
  const waypoint = (id: string, card: WorkflowStepCardProps): Waypoint => {
    const state = begun ? stateOf(card) : "next";
    return { id, label: card.name, state, said: begun ? card.said : "Not started", here: begun && id === run.running, card: begun ? card : { ...card, activity: "not_started", current: false, needs: [] } };
  };
  const spine = run.nodes.filter((node) => node.id.startsWith("step:")).map((node) => waypoint(node.id, node.card));
  const fans = new Map<string, Waypoint[]>();
  const works = stepThatWorksTheGroups(whole);
  if (works !== undefined && groups.length >= 2) {
    fans.set(
      `step:${works}`,
      groups.map((group) => waypoint(`group:${group.id}`, { kind: "group", name: `Group ${group.ordinal}`, activity: GROUP_ACTIVITY[group.state], said: group.state, ...(group.state === "failed" ? { needs: [{ says: "Failed", tone: "failed" as const }] } : {}) })),
    );
  }
  const returns = run.edges.filter((edge) => edge.kind === "returns").map((edge) => ({ from: edge.source, to: edge.target }));
  return { spine, fans, returns, live: spine.find((one) => one.here)?.id };
}

// ---- Geometry ------------------------------------------------------------------------------------------

export type PinIn = Waypoint & { at: Point; fan: boolean };
export type LegIn = { id: string; from: Point; to: Point; done: boolean; back: boolean };
export type RouteIn = {
  pins: readonly PinIn[];
  legs: readonly LegIn[];
  /** Where the pull request waits, and where the base pad and the flag are. */
  crate: Point;
  pad: Point;
  flag: Point;
  /** The live waypoint, which the camera opens on. */
  live: Point | undefined;
};

export type SceneIn = {
  mission: Mission;
  repo: { root: string; name: string };
  /** The Job's drone, its Sessions, and the Drones of its steps. */
  drones: readonly DroneIn[];
  job: string;
  /** The waypoint each of the others serves. */
  serves: ReadonlyMap<string, string>;
  /** The bay the Job holds. */
  pad: string | undefined;
  /** How large a drone is drawn, which this scene does not need, and whether the pane is wider than it is tall: the route runs across it, or up it. */
  big: number;
  wide: boolean;
  /** The write clashes Fleet reports, of which the ones on this scene's plots are drawn. */
  clashes?: readonly Clash[];
};

/** The distance between two steps of the route, in the layout's units: across a wide pane the route runs along it, up a tall one it climbs toward the far edge. */
const STEPS = { wide: { x: 200, y: 38 }, tall: { x: 118, y: 124 } };
/** How far the route sways to either side of its line. */
const SWAY = 24;

const tilePathOf = (path: string) => path.split("/").slice(0, 2).join("/");

/**
 * The scene as the battlefield's layout reads it: one territory holding only the plots the Job touches, a base at the route's start,
 * the flag at its end, and every drone placed. The route is placed in the plane's own units and then moved to the middle of a plane
 * sized round it, so the camera has the whole of it to look over.
 */
export function sceneOf(input: SceneIn): { layout: Layout; route: RouteIn; world: Size } {
  const { mission, drones, job } = input;
  const STEP = (input.wide ? STEPS.wide : STEPS.tall).x;
  const CLIMB = (input.wide ? STEPS.wide : STEPS.tall).y;
  const spine = mission.spine;
  const n = spine.length;
  // The spine, from the first step at the near end to the last far off, a gentle sway so it reads as a path and not a ruler.
  const at = new Map<string, Point>();
  spine.forEach((one, i) => at.set(one.id, { x: i * STEP, y: -i * CLIMB + (i % 2 === 0 ? SWAY : -SWAY) }));
  // Across the route: where plots stand to either side of it, and where a step's groups fan out.
  const length = Math.hypot(STEP, CLIMB);
  const perp: Point = { x: CLIMB / length, y: STEP / length };
  const fanAt = new Map<string, Point>();
  const fanReach = new Map<number, number>();
  spine.forEach((one, i) => {
    const group = mission.fans.get(one.id);
    if (group === undefined || i === n - 1) return;
    const [a, b] = [at.get(one.id)!, at.get(spine[i + 1]!.id)!];
    fanReach.set(i, ((group.length - 1) / 2) * 76 + 54);
    group.forEach((pin, j) => {
      const off = (j - (group.length - 1) / 2) * 76;
      fanAt.set(pin.id, { x: (a.x + b.x) / 2 + perp.x * off, y: (a.y + b.y) / 2 + perp.y * off });
    });
  });
  const first = at.get(spine[0]?.id ?? "") ?? { x: 0, y: 0 };
  const last = at.get(spine[n - 1]?.id ?? "") ?? first;
  const [along, up] = [{ x: STEP / length, y: -CLIMB / length }, input.wide ? 1 : 0];
  const pad: Point = { x: first.x - along.x * 170 + perp.x * 80 * (1 - up) , y: first.y - along.y * 170 + (up ? 74 : perp.y * 80) };
  const flag: Point = { x: last.x + along.x * STEP * 0.9, y: last.y + along.y * STEP * 0.9 - 34 };
  const crate: Point = { x: last.x + along.x * 80 + perp.x * 60, y: last.y + along.y * 80 + perp.y * 60 - 20 };

  // The plots: the Job's own, the biggest nearest the step at work, the rest along the route on both sides of it.
  const weight = new Map<string, number>();
  for (const one of drones) for (const touch of one.touches) weight.set(tilePathOf(touch.path), (weight.get(tilePathOf(touch.path)) ?? 0) + touch.w * (one.key === job ? 1 : 0.5));
  const ranked = [...weight].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
  const liveAt = Math.max(0, spine.findIndex((one) => one.id === mission.live));
  const slots: { i: number; side: -1 | 1 }[] = [];
  for (let d = 0; d <= n; d += 1) {
    for (const i of new Set([liveAt - d, liveAt + d])) {
      if (i < 0 || i >= n) continue;
      slots.push({ i, side: -1 }, { i, side: 1 });
    }
  }
  const tiles: Tile[] = [];
  ranked.forEach(([path, w], k) => {
    const slot = slots[k % Math.max(1, slots.length)];
    if (slot === undefined) return;
    const [wide, deep] = [Math.min(STEP - 24, 118 + 14 * w), Math.min(120, 68 + 9 * w)];
    const centre = at.get(spine[slot.i]!.id)!;
    const room = Math.max(fanReach.get(slot.i) ?? 0, fanReach.get(slot.i - 1) ?? 0, 56);
    const off = room + 34 + (deep / 2) * Math.abs(perp.y) + (wide / 2) * Math.abs(perp.x) + 90 * Math.floor(k / slots.length);
    const [cx, cy] = [centre.x + perp.x * slot.side * off, centre.y + perp.y * slot.side * off];
    tiles.push({ path, top: path.split("/")[0]!, rect: { x: cx - wide / 2, y: cy - deep / 2, w: wide, h: deep }, cx, cy });
  });
  const tileOf = (path: string) => tiles.find((one) => path === one.path || path.startsWith(`${one.path}/`));

  // The drones: the Job's over the step at work (or the pad before it starts, the flag's end when it is over), the others beside the step each serves.
  const place = new Map<string, Point>();
  const here = mission.live === undefined ? undefined : at.get(mission.live);
  const allDone = n > 0 && spine.every((one) => one.state === "done");
  place.set(job, here ?? (allDone ? { x: last.x, y: last.y } : { x: pad.x + 30, y: pad.y - 46 }));
  const beside = new Map<string, number>();
  for (const one of drones) {
    if (one.key === job) continue;
    const to = input.serves.get(one.key) ?? mission.live;
    const target = to === undefined ? undefined : (at.get(to) ?? fanAt.get(to));
    const count = beside.get(to ?? "") ?? 0;
    beside.set(to ?? "", count + 1);
    const base = target ?? place.get(job)!;
    place.set(one.key, { x: base.x + (count % 2 === 0 ? -1 : 1) * (66 + 16 * Math.floor(count / 2)), y: base.y + 58 + 12 * Math.floor(count / 2) });
  }
  const over = new Map<string, { tile: Tile; w: number }[]>();
  for (const one of drones) {
    const share = new Map<Tile, number>();
    for (const touch of one.touches) {
      const tile = tileOf(touch.path);
      if (tile !== undefined) share.set(tile, (share.get(tile) ?? 0) + touch.w);
    }
    const list = [...share].map(([tile, w]) => ({ tile, w })).sort((a, b) => b.w - a.w || a.tile.path.localeCompare(b.tile.path));
    over.set(one.key, list);
  }
  // A clash is Fleet's, from git: it marks the plot of the area it names, where this scene draws one.
  const clashes = (input.clashes ?? []).filter((clash) => clash.root === input.repo.root && tileOf(clash.path) !== undefined);

  // Everything, moved to the middle of a plane sized round it.
  const marks: Point[] = [pad, flag, crate, ...[...at.values()], ...[...fanAt.values()], ...[...place.values()], ...tiles.flatMap((t) => [{ x: t.rect.x, y: t.rect.y }, { x: t.rect.x + t.rect.w, y: t.rect.y + t.rect.h }])];
  const l = Math.min(...marks.map((m) => m.x)) - 120;
  const r = Math.max(...marks.map((m) => m.x)) + 140;
  const t = Math.min(...marks.map((m) => m.y)) - 150;
  const b = Math.max(...marks.map((m) => m.y)) + 130;
  const world: Size = { width: Math.round(r - l + 700), height: Math.round(b - t + 560) };
  const [dx, dy] = [world.width / 2 - (l + r) / 2, world.height / 2 - (t + b) / 2];
  const sh = (p: Point): Point => ({ x: p.x + dx, y: p.y + dy });
  const moved = (rect: Tile["rect"]) => ({ ...rect, x: rect.x + dx, y: rect.y + dy });

  const movedTiles: Tile[] = tiles.map((one) => ({ ...one, rect: moved(one.rect), cx: one.cx + dx, cy: one.cy + dy }));
  const swap = new Map(tiles.map((one, i) => [one, movedTiles[i]!] as const));
  const padAt = sh(pad);
  const baseW = 128;
  const territory: Territory = {
    root: input.repo.root,
    name: input.repo.name,
    rect: { x: l + dx, y: t + dy, w: r - l, h: b - t },
    groups: [],
    tiles: movedTiles,
    base: { x: padAt.x - 24, y: padAt.y - 2, w: baseW },
    pads: input.pad === undefined ? [] : [{ key: input.pad, x: padAt.x, y: padAt.y }],
    goal: sh(flag),
  };
  const layoutAt = new Map<string, Placed>(drones.map((one) => [one.key, { ...sh(place.get(one.key)!), s: 1, root: input.repo.root }]));
  const layoutOver = new Map([...over].map(([key, list]) => [key, list.map(({ tile, w }) => ({ tile: swap.get(tile)!, w }))] as const));
  const layout: Layout = { territories: [territory], at: layoutAt, over: layoutOver, clashes };

  const pins: PinIn[] = [
    ...spine.map((one): PinIn => ({ ...one, at: sh(at.get(one.id)!), fan: false })),
    ...[...mission.fans.values()].flatMap((group) => group.map((one): PinIn | undefined => (fanAt.has(one.id) ? { ...one, at: sh(fanAt.get(one.id)!), fan: true } : undefined)).filter((one): one is PinIn => one !== undefined)),
  ];
  const where = new Map(pins.map((one) => [one.id, one] as const));
  const legs: LegIn[] = [];
  const leg = (from: Point, to: Point, done: boolean, back = false) => legs.push({ id: `${legs.length}`, from, to, done, back });
  if (n > 0) leg(padAt, where.get(spine[0]!.id)!.at, spine[0]!.state !== "next");
  spine.forEach((one, i) => {
    const next = spine[i + 1];
    const group = mission.fans.get(one.id);
    const from = where.get(one.id)!;
    if (next === undefined) return void leg(from.at, sh(flag), one.state === "done");
    const to = where.get(next.id)!;
    if (group === undefined || !fanAt.has(group[0]!.id)) return void leg(from.at, to.at, one.state === "done");
    for (const pin of group) {
      const mid = where.get(pin.id)!;
      leg(from.at, mid.at, one.state === "done");
      leg(mid.at, to.at, pin.state === "done");
    }
  });
  for (const back of mission.returns) {
    const [a, c] = [where.get(back.from), where.get(back.to)];
    if (a !== undefined && c !== undefined) leg(a.at, c.at, false, true);
  }
  // The camera opens on the step at work; on a Job not begun, its first step; on one finished, its last.
  const focus = mission.live ?? (allDone ? spine[n - 1]?.id : spine[0]?.id);
  const live = focus === undefined ? undefined : where.get(focus)?.at;
  return { layout, route: { pins, legs, crate: sh(crate), pad: padAt, flag: sh(flag), live }, world };
}

/** Where a point of the route stands on the plane. */
export const planeOf = (p: Point): Point => ({ x: p.x, y: p.y * FLAT });
