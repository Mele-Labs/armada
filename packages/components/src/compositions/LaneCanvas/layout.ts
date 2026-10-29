// Where the lanes sit, at the scale they are read: one canvas unit is one pixel. A node's column is shared by every lane so time reads across them; a lane is a row, and the
// rows share the canvas's height. A rule's chip sits in the gutter on a lane's edge, where no node is, so it can never cover one.

import type { LaneView, RuleView } from "./model";

export const GEO = {
  /** The lane head, down the left. */
  head: 144,
  pad: 16,
  colW: 196,
  nodeW: 168,
  /** A step's node holds its name and what ends it, two lines; a state's holds its name and who it is assigned to. */
  stepH: 68,
  stateH: 56,
  /** A lane is never shorter than a node and a chip's height either side of it. */
  laneMin: 128,
  chipH: 28,
} as const;

/** Where the canvas opens and Fit returns to: the top left, at the scale the lanes are drawn at. */
export const HOME = { x: 0, y: 0, zoom: 1 } as const;

export type Rect = { x: number; y: number; w: number; h: number };

export type Placement = {
  /** Each lane's band, full width. */
  bands: Map<string, Rect>;
  /** Each node, by its id. */
  items: Map<string, Rect>;
  laneH: number;
  width: number;
  height: number;
};

/** The lanes fill `height`, and stand at the least the lane height tall each when it is short. */
export function place(lanes: readonly LaneView[], height: number): Placement {
  const last = Math.max(0, ...lanes.flatMap((lane) => lane.nodes.map((node) => node.col)));
  const width = GEO.head + GEO.pad * 2 + last * GEO.colW + GEO.nodeW;
  const laneH = Math.max(GEO.laneMin, Math.floor(height / Math.max(1, lanes.length)));
  const bands = new Map<string, Rect>();
  const items = new Map<string, Rect>();
  lanes.forEach((lane, at) => {
    const y = at * laneH;
    const h = lane.kind === "workflow" ? GEO.stepH : GEO.stateH;
    bands.set(lane.id, { x: 0, y, w: width, h: laneH });
    for (const node of lane.nodes) {
      items.set(node.id, { x: GEO.head + GEO.pad + node.col * GEO.colW, y: y + (laneH - h) / 2, w: GEO.nodeW, h });
    }
  });
  return { bands, items, laneH, width, height: lanes.length * laneH };
}

// ---- Rule chips ----------------------------------------------------------------------------------------------------------

/** The chip's size as the words will draw it: a width worked out from the characters, since the canvas places before anything is measured. */
export function chipSize(rule: RuleView): { w: number; h: number } {
  if (rule.quiet === true) return { w: GEO.chipH, h: GEO.chipH };
  const words = rule.label.length * 6.6 + (rule.meta === undefined ? 0 : 12 + rule.meta.length * 7.9);
  return { w: Math.ceil(words + 36), h: GEO.chipH };
}

/** The shape of the curve React Flow draws between a bottom and a top handle: its height and width as fractions of the way along, both from the one parameter. */
const rise = (t: number): number => 1.5 * t * (1 - t) + t * t * t;
const slide = (t: number): number => 3 * t * t - 2 * t * t * t;

/** Where along its curve an edge crosses the height `y`, as the x it is at there. */
function crossing(from: { x: number; y: number }, to: { x: number; y: number }, y: number): number {
  const along = (y - from.y) / (to.y - from.y);
  let low = 0;
  let high = 1;
  for (let step = 0; step < 24; step += 1) {
    const mid = (low + high) / 2;
    if (rise(mid) < along) low = mid;
    else high = mid;
  }
  return from.x + (to.x - from.x) * slide((low + high) / 2);
}

const touches = (a: Rect, b: Rect, margin: number): boolean => a.x < b.x + b.w + margin && b.x < a.x + a.w + margin && a.y < b.y + b.h + margin && b.y < a.y + a.h + margin;

/**
 * Where each rule's chip sits, by its edge's id. **On the edge, in the gutter on the line between two lanes**, at the one it crosses nearest the node it leaves; a rule
 * that crosses several lanes has several gutters to choose from, and takes the next when a chip is already there. A chip that finds none free slides along the gutter.
 * The ones with words are placed first, so a chip with only its mark is the one that moves.
 */
export function placeChips(rules: readonly RuleView[], placement: Placement): Map<string, { x: number; y: number }> {
  const { items, laneH, width } = placement;
  const out = new Map<string, { x: number; y: number }>();
  const taken: Rect[] = [];
  const ordered = [...rules.filter((rule) => rule.quiet !== true), ...rules.filter((rule) => rule.quiet === true)];
  for (const rule of ordered) {
    const from = items.get(rule.from);
    const to = items.get(rule.to);
    if (from === undefined || to === undefined) continue;
    const down = to.y > from.y;
    const start = { x: from.x + from.w / 2, y: down ? from.y + from.h : from.y };
    const end = { x: to.x + to.w / 2, y: down ? to.y : to.y + to.h };
    const lanes = [Math.floor(from.y / laneH), Math.floor(to.y / laneH)] as const;
    // The lines between the two lanes, the one nearest the source first. A rule within one lane has the line above it.
    const lines = lanes[0] === lanes[1] ? [lanes[0] * laneH] : down ? range(lanes[0] + 1, lanes[1]).map((at) => at * laneH) : range(lanes[1] + 1, lanes[0]).reverse().map((at) => at * laneH);
    const size = chipSize(rule);
    const fit = (x: number, y: number): Rect => {
      const left = Math.min(Math.max(x - size.w / 2, GEO.head + GEO.pad / 2), Math.max(GEO.head + GEO.pad / 2, width - size.w - GEO.pad / 2));
      return { x: left, y: y - size.h / 2, w: size.w, h: size.h };
    };
    let chosen: Rect | undefined;
    for (const y of lines) {
      const sits = fit(lanes[0] === lanes[1] ? (start.x + end.x) / 2 : crossing(start, end, y), y);
      if (!taken.some((one) => touches(one, sits, 4))) {
        chosen = sits;
        break;
      }
    }
    if (chosen === undefined) {
      // Every gutter on its way is taken where it crosses: slide along the first until nothing is there.
      let sits = fit(crossing(start, end, lines[0]!), lines[0]!);
      for (let tries = 0; tries < 12; tries += 1) {
        const hit = taken.find((one) => touches(one, sits, 4));
        if (hit === undefined) break;
        sits = { ...sits, x: hit.x + hit.w + 6 };
      }
      chosen = sits;
    }
    taken.push(chosen);
    out.set(rule.id, { x: chosen.x + chosen.w / 2, y: chosen.y + chosen.h / 2 });
  }
  return out;
}

const range = (from: number, to: number): number[] => Array.from({ length: Math.max(0, to - from + 1) }, (_, at) => from + at);

export type Arrow = "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown";
export const isArrow = (key: string): key is Arrow => key === "ArrowLeft" || key === "ArrowRight" || key === "ArrowUp" || key === "ArrowDown";

/**
 * The node an arrow reaches: the nearest whose centre lies that way, one off to the side costing twice what one straight on does.
 * Lanes are rows, so a step down lands in the lane below at the nearest column.
 */
export function nearest(items: ReadonlyMap<string, Rect>, from: string, arrow: Arrow): string | undefined {
  const here = items.get(from);
  if (here === undefined) return undefined;
  const cx = here.x + here.w / 2;
  const cy = here.y + here.h / 2;
  let best: { id: string; cost: number } | undefined;
  for (const [id, one] of items) {
    if (id === from) continue;
    const dx = one.x + one.w / 2 - cx;
    const dy = one.y + one.h / 2 - cy;
    const along = arrow === "ArrowRight" ? dx : arrow === "ArrowLeft" ? -dx : arrow === "ArrowDown" ? dy : -dy;
    const across = arrow === "ArrowRight" || arrow === "ArrowLeft" ? Math.abs(dy) : Math.abs(dx);
    if (along <= 0 || across > along * 3) continue;
    const cost = along + across * 2;
    if (best === undefined || cost < best.cost) best = { id, cost };
  }
  return best?.id;
}
