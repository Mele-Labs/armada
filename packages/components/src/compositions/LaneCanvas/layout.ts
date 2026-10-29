// Where the lanes sit. A node's column is shared by every lane so time reads across them; a lane is a row of one height.

import type { LaneView } from "./model";

/** The canvas's own coordinates, not pixels: the zoom scales all of it. */
export const GEO = { head: 124, pad: 14, colW: 140, nodeW: 120, nodeH: 46, laneH: 100 } as const;

export type Rect = { x: number; y: number; w: number; h: number };

export type Placement = {
  /** Each lane's band, full width. */
  bands: Map<string, Rect>;
  /** Each node, by its id. */
  items: Map<string, Rect>;
  width: number;
  height: number;
};

export function place(lanes: readonly LaneView[]): Placement {
  const last = Math.max(0, ...lanes.flatMap((lane) => lane.nodes.map((node) => node.col)));
  const width = GEO.head + GEO.pad * 2 + last * GEO.colW + GEO.nodeW;
  const bands = new Map<string, Rect>();
  const items = new Map<string, Rect>();
  lanes.forEach((lane, at) => {
    const y = at * GEO.laneH;
    bands.set(lane.id, { x: 0, y, w: width, h: GEO.laneH });
    for (const node of lane.nodes) {
      items.set(node.id, { x: GEO.head + GEO.pad + node.col * GEO.colW, y: y + (GEO.laneH - GEO.nodeH) / 2, w: GEO.nodeW, h: GEO.nodeH });
    }
  });
  return { bands, items, width, height: lanes.length * GEO.laneH };
}

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
