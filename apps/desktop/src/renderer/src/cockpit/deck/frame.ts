// What the battlefield works out once for the frame it draws, from the layout and the drones: how high each plot stands, where a drone
// flies and what lies under it, which plot a working drone aims at and where its action lands. Pure. Ported from
// proto/cockpit-flight-deck. #2301.

import { FOOT, MARGIN } from "./geometry";
import { FLAT, heightsOf, type DroneIn, type Layout, type P3, type Placed, type Point, type Tile } from "./layout";
import type { Play } from "./plays";

export type Frame = {
  heights: Map<string, number>;
  tiles: readonly Tile[];
  /** The height of the plot a point stands on, where one is under it. */
  floorAt: (at: Point) => number;
  /** How high a drone flies: clear of the tallest slab in reach of its box, and a little higher while it works. */
  altOf: (drone: DroneIn) => number;
  /** Where a drone is in the air, on the plane. */
  flying: (drone: DroneIn) => P3 | undefined;
  /** The plot a working drone aims at: the biggest it touches. */
  aimOf: (drone: DroneIn) => Tile | undefined;
  /** Where on its plot a drone's action lands: the point nearest the drone, kept far enough in that the scan plate stays on the plot. */
  aimPoint: (drone: DroneIn, tile: Tile) => Point;
  /** The working drones that have something to aim at, and are placed. */
  doers: readonly DroneIn[];
  /** How far out a circling drone flies from the centre of its ring. */
  orbit: number;
};

/** How high a drone that has landed on the pad stands over it. */
export const PAD_UP = 14;

/** How much higher a thinking drone flies over its last file than one at work there. */
export const THINK_RISE = 26;

const clamp = (n: number, lo: number, hi: number) => (lo > hi ? (lo + hi) / 2 : Math.min(hi, Math.max(lo, n)));

export function frameOf(input: { layout: Layout; drones: readonly DroneIn[]; plays: readonly Play[]; big: number }): Frame {
  const { layout, drones, plays, big } = input;
  const heights = heightsOf(layout, drones, plays);
  const tiles = layout.territories.flatMap((one) => one.tiles);
  const floorAt = (at: Point) => {
    const under = tiles.find((t) => at.x >= t.rect.x && at.x <= t.rect.x + t.rect.w && at.y >= t.rect.y && at.y <= t.rect.y + t.rect.h);
    return under === undefined ? 0 : (heights.get(under.path) ?? 0);
  };
  const altOf = (drone: DroneIn) => {
    const at: Placed | undefined = layout.at.get(drone.key);
    if (at === undefined) return 0;
    // On the pad it has landed: it stands just over the ground, whatever plots stand near.
    if (drone.off === "pad" || drone.off === "walk") return Math.round(PAD_UP * big);
    const reach = tiles.filter((t) => t.rect.x < at.x + 170 && t.rect.x + t.rect.w > at.x - 170 && t.rect.y < at.y + 90 && t.rect.y + t.rect.h > at.y - 60);
    const clear = Math.max(floorAt(at), ...reach.map((t) => heights.get(t.path) ?? 0));
    // Reading, it dips toward the plot it is probing; thinking, it rises over its last file.
    return clear + FOOT + MARGIN + (drone.hover ? Math.round((drone.doing === "read" ? 6 : 18) * big) : 0) + (drone.off === "think" ? Math.round(THINK_RISE * big) : 0);
  };
  const flying = (drone: DroneIn): P3 | undefined => {
    const at = layout.at.get(drone.key);
    return at === undefined ? undefined : { x: at.x, y: at.y * FLAT, z: altOf(drone) };
  };
  const aimOf = (drone: DroneIn) => (drone.doing === undefined || !drone.shown ? undefined : layout.over.get(drone.key)?.[0]?.tile);
  const aimPoint = (drone: DroneIn, tile: Tile): Point => {
    const at = layout.at.get(drone.key)!;
    const hw = Math.min(tile.rect.w, 150) * 0.43;
    const hh = Math.min(tile.rect.h, 100 / FLAT) * 0.43;
    return { x: clamp(at.x, tile.rect.x + hw, tile.rect.x + tile.rect.w - hw), y: clamp(at.y, tile.rect.y + hh, tile.rect.y + tile.rect.h - hh) };
  };
  return { heights, tiles, floorAt, altOf, flying, aimOf, aimPoint, doers: drones.filter((one) => aimOf(one) !== undefined && layout.at.has(one.key)), orbit: 28 + 10 * big };
}
