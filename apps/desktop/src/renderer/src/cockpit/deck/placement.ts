// Where the callout's card stands from the thing it is about, and where its dotted leader runs. Up and to the right by default,
// flipping left or down at the pane's edges so it never clips and never lies over what it is about; the leader ends at the card's
// near corner, or on its near side where the card has been pushed alongside. Pure: the card draws and follows the camera in
// `Callout.tsx`. Ported from proto/cockpit-flight-deck. #2301.

import type { Point } from "./layout";

/** What the card keeps clear of the pane's border, and how far off the thing it stands. */
export const EDGE = 10;
export const GAP = 44;

/** The thing's box on the pane: its centre and its size. A point (the pointer) is a small box. */
export type Anchored = { x: number; y: number; w: number; h: number };

export type Placed = {
  left: number;
  top: number;
  right: boolean;
  up: boolean;
  /** The leader: from just off the thing, to the card. */
  start: Point;
  end: Point;
};

const clamp = (n: number, low: number, high: number) => Math.min(Math.max(n, low), Math.max(low, high));

export function placeCallout(a: Anchored, card: { w: number; h: number }, pane: { width: number; height: number }): Placed {
  const [hw, hh] = [a.w / 2, a.h / 2];
  // Up and right unless that runs off the pane: then the side that has room.
  const right = a.x + hw + GAP + card.w <= pane.width - EDGE || (a.x - hw - GAP - card.w < EDGE && pane.width - a.x >= a.x);
  const up = a.y - hh - GAP - card.h >= EDGE || (a.y + hh + GAP + card.h > pane.height - EDGE && a.y >= pane.height - a.y);
  const left = clamp(right ? a.x + hw + GAP : a.x - hw - GAP - card.w, EDGE, pane.width - card.w - EDGE);
  const top = clamp(up ? a.y - hh - GAP - card.h : a.y + hh + GAP, EDGE, pane.height - card.h - EDGE);
  // The leader ends at the card's near corner, or on its near edge where the card has been pushed alongside.
  const level = a.y >= top && a.y <= top + card.h;
  const end = { x: right ? left : left + card.w, y: level ? clamp(a.y, top + 12, top + card.h - 12) : up ? top + card.h : top };
  const length = Math.hypot(end.x - a.x, end.y - a.y) || 1;
  const lift = Math.min(22, length / 3);
  const start = { x: a.x + ((end.x - a.x) / length) * lift, y: a.y + ((end.y - a.y) / length) * lift };
  return { left, top, right, up, start, end };
}
