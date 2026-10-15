// The numbers the battlefield's parts agree on: how high the lifted layer stands, the box a drone is drawn in and how far its foot
// hangs under its centre, and where a point at some height lands on the lifted layer at the camera's pitch. Pure. Ported from
// proto/cockpit-flight-deck. #2301.

import { CAM, SLAB, type P3, type Point } from "./layout";

/**
 * The height of the layer the lifted lines are drawn on, just above the tallest slab. And the picture of a drone: a box standing
 * upright round it. Two planes that cross are split and drawn in pieces, and a piece can lose to the wrong one, so a drone flies with
 * the foot of that box clear of every slab and ground beneath it (`FOOT`), plus a margin for a slab that rises when something touches it.
 */
export const LAYER = SLAB.high + 2;
export const BOX = { x: -68, y: -58, w: 136, h: 96 };
export const FOOT = 0.81 * (BOX.y + BOX.h);
export const MARGIN = 10;
/** The least height a pull request flies at: its picture clear of every slab. */
export const ROUTE = SLAB.high + 22;
export const SIN = Math.sin((CAM.wide * Math.PI) / 180);
export const COS = Math.cos((CAM.wide * Math.PI) / 180);

/** Where a point at some height is drawn on the lifted layer, so that it shows where it stands at the camera's pitch. */
export function liftOf(pitch: number): (p: P3) => Point {
  const tan = Math.tan((pitch * Math.PI) / 180);
  const sin = Math.sin((CAM.yaw * Math.PI) / 180);
  const cos = Math.cos((CAM.yaw * Math.PI) / 180);
  return (p) => {
    const a = (p.z - LAYER) * tan;
    return { x: p.x - a * sin, y: p.y - a * cos };
  };
}

/** Four corner brackets round a box: the lock-on reticle. */
export function bracketsOf(w: number, top: number, bottom: number): string {
  const leg = 9;
  return [
    [-1, top, 1],
    [1, top, 1],
    [1, bottom, -1],
    [-1, bottom, -1],
  ]
    .map(([sx, y, sy]) => `M${sx! * w},${y! + sy! * leg} V${y} H${sx! * (w - leg)}`)
    .join(" ");
}

export const short = (title: string, room: number) => (title.length > room ? `${title.slice(0, room - 1)}…` : title);
