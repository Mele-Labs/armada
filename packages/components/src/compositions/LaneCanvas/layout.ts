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
  /** A step's second line is a state or the Jobs it waits on by name, which wraps: about this many characters to a line, and each line past the two its node holds adds this much height. */
  subChars: 20,
  subLine: 16,
  /** A lane is never shorter than a node and a chip's height either side of it. */
  laneMin: 128,
  chipH: 28,
  /** The small circle where the workflow starts or ends, and how far it stands from the first or the last step. */
  dot: 24,
  dotGap: 16,
  /** Room before the first column, for the circle that starts the workflow. */
  lead: 48,
  /** The toolbar that stands over what is picked: its widest, with Edit and three acts, its height, and the gap between it and what it stands over. */
  toolbarW: 208,
  toolbarH: 44,
  toolbarGap: 10,
} as const;

/** Where the canvas opens and Fit returns to: the top left, at the scale the lanes are drawn at. */
export const HOME = { x: 0, y: 0, zoom: 1 } as const;

/** How far out the canvas goes: a node's words are the kit's body size, and 0.8 of it is the smallest they are still read at. A frame narrower than the lanes scrolls and never shrinks them further. */
export const MIN_ZOOM = 0.8;

export type Rect = { x: number; y: number; w: number; h: number };

/** A node or a chip with the room its toolbar takes, centred on it and above or below, so a pan that brings the one into view brings the other. */
export function withToolbar(rect: Rect, side: "above" | "below"): Rect {
  const w = Math.max(rect.w, GEO.toolbarW);
  const x = rect.x - (w - rect.w) / 2;
  const room = GEO.toolbarH + GEO.toolbarGap;
  return side === "above" ? { x, y: rect.y - room, w, h: rect.h + room } : { x, y: rect.y, w, h: rect.h + room };
}

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
  const nodes = lanes.flatMap((lane) => lane.nodes);
  const last = Math.max(0, ...nodes.map((node) => node.col));
  const origin = GEO.head + GEO.pad + GEO.lead;
  const tail = nodes.some((node) => node.kind === "end") ? GEO.dotGap + GEO.dot : 0;
  const width = origin + GEO.pad + last * GEO.colW + GEO.nodeW + tail;
  const laneH = Math.max(GEO.laneMin, Math.floor(height / Math.max(1, lanes.length)));
  const bands = new Map<string, Rect>();
  const items = new Map<string, Rect>();
  lanes.forEach((lane, at) => {
    const y = at * laneH;
    // A step that names what it waits on (several Jobs, by title) is as tall as its words need.
    const lines = lane.kind === "workflow" ? Math.max(0, ...lane.nodes.map((node) => (node.waiting ?? [node.sub ?? ""]).reduce((sum, said) => sum + Math.ceil(said.length / GEO.subChars), 0) - 2)) : 0;
    const h = (lane.kind === "workflow" ? GEO.stepH : GEO.stateH) + lines * GEO.subLine;
    bands.set(lane.id, { x: 0, y, w: width, h: laneH });
    for (const node of lane.nodes) {
      const x = origin + node.col * GEO.colW;
      // The circles that start and end the workflow stand just before the first step and just after the last.
      if (node.kind === "start") items.set(node.id, { x: x - GEO.dotGap - GEO.dot, y: y + (laneH - GEO.dot) / 2, w: GEO.dot, h: GEO.dot });
      else if (node.kind === "end") items.set(node.id, { x: x + GEO.nodeW + GEO.dotGap, y: y + (laneH - GEO.dot) / 2, w: GEO.dot, h: GEO.dot });
      else items.set(node.id, { x, y: y + (laneH - h) / 2, w: GEO.nodeW, h });
    }
  });
  return { bands, items, laneH, width, height: lanes.length * laneH };
}

// ---- Rule chips ----------------------------------------------------------------------------------------------------------

/** The chip's size as the words will draw it: a width worked out from the characters, since the canvas places before anything is measured. */
export function chipSize(rule: RuleView): { w: number; h: number } {
  if (rule.quiet === true) return { w: GEO.chipH, h: GEO.chipH };
  const words = rule.label.length * 6.6 + (rule.meta === undefined ? 0 : 12 + rule.meta.length * 7.9);
  return { w: Math.ceil(words + 44 + (rule.differs === undefined ? 0 : 18)), h: GEO.chipH };
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
export function placeChips(rules: readonly RuleView[], placement: Placement): Map<string, { x: number; y: number; w: number; h: number }> {
  const { items, laneH, width } = placement;
  const out = new Map<string, { x: number; y: number; w: number; h: number }>();
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
    out.set(rule.id, { x: chosen.x + chosen.w / 2, y: chosen.y + chosen.h / 2, w: chosen.w, h: chosen.h });
  }
  return out;
}

const range = (from: number, to: number): number[] => Array.from({ length: Math.max(0, to - from + 1) }, (_, at) => from + at);

// ---- Following ------------------------------------------------------------------------------------------------------------

export type View = { x: number; y: number; zoom: number };

/**
 * The view that brings `rects` into the frame, or `undefined` where they already are, at the zoom they are in. The lane heads stand over the left of the frame, so what is
 * brought in clears them. **A set wider than the frame keeps its newest**: the leftmost is let go until the rest fits. `follow` puts the set at the left where it fits, so the
 * earliest of what changed is the context the person reads from; without it the pan is the least that brings the set into view, as a step by key wants. `down` moves it up or down only.
 */
export function panTo(rects: readonly Rect[], view: View, frame: { width: number; height: number }, content: { width: number; height: number }, follow: boolean, down = false): View | undefined {
  if (rects.length === 0 || frame.width === 0 || frame.height === 0) return undefined;
  const { zoom } = view;
  const left = GEO.head + GEO.pad;
  const right = GEO.pad;
  const gap = GEO.pad / 2;
  const usable = frame.width - left - right;
  const sorted = [...rects].sort((a, b) => a.x - b.x);
  const box = (set: readonly Rect[]): Rect => {
    const x = Math.min(...set.map((one) => one.x));
    const y = Math.min(...set.map((one) => one.y));
    return { x, y, w: Math.max(...set.map((one) => one.x + one.w)) - x, h: Math.max(...set.map((one) => one.y + one.h)) - y };
  };
  let kept = sorted;
  while (kept.length > 1 && box(kept).w * zoom > usable) kept = kept.slice(1);
  const at = box(kept);
  let x = view.x;
  const from = at.x * zoom + view.x;
  const to = (at.x + at.w) * zoom + view.x;
  if (from < left || to > frame.width - right) {
    if (follow || from < left || at.w * zoom > usable) x = left - at.x * zoom;
    else x = frame.width - right - (at.x + at.w) * zoom;
  }
  let y = view.y;
  const top = at.y * zoom + view.y;
  const bottom = (at.y + at.h) * zoom + view.y;
  if (top < gap || bottom > frame.height - gap) y = top < gap || at.h * zoom > frame.height - gap * 2 ? gap - at.y * zoom : frame.height - gap - (at.y + at.h) * zoom;
  if (down) x = view.x;
  // Never past the lanes: nothing is left of the first column or right of the last.
  x = Math.min(0, content.width * zoom > frame.width ? Math.max(x, frame.width - content.width * zoom) : 0);
  y = Math.min(0, content.height * zoom > frame.height ? Math.max(y, frame.height - content.height * zoom) : 0);
  return Math.abs(x - view.x) < 1 && Math.abs(y - view.y) < 1 ? undefined : { x, y, zoom };
}

export type Arrow = "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown";
export const isArrow = (key: string): key is Arrow => key === "ArrowLeft" || key === "ArrowRight" || key === "ArrowUp" || key === "ArrowDown";

/**
 * The node an arrow reaches: the nearest whose centre lies that way, one off to the side costing twice what one straight on does.
 * Lanes are rows, so a step down lands in the lane below at the nearest column. `skip` are the nodes nothing reaches: the circles at the ends of the workflow.
 */
export function nearest(items: ReadonlyMap<string, Rect>, from: string, arrow: Arrow, skip?: ReadonlySet<string>): string | undefined {
  const here = items.get(from);
  if (here === undefined) return undefined;
  const cx = here.x + here.w / 2;
  const cy = here.y + here.h / 2;
  let best: { id: string; cost: number } | undefined;
  for (const [id, one] of items) {
    if (id === from || skip?.has(id) === true) continue;
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
