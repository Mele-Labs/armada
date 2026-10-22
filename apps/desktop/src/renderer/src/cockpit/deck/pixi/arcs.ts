// The arc through the air between this Job's drone and another Job's while one messages the other, read from `session.messaged`: a line in
// the accent, small packets travelling it from the sender to the receiver, and the message quoted at its middle. The packets move only
// while the arc is one of the few loops the map runs (`loopsOf`), and stand still under reduced motion, a tail behind each still saying
// which way it goes. The quote's box stands with the other upright pictures (`stands.ts`). `docs/concepts/cockpit.md`, *A Job open*.

import type { Graphics as G } from "pixi.js";

import { alongOf } from "../layout";
import { turn, type Clock } from "./clock";
import { hex, mix, type Palette, type Rgba } from "./palette";
import { project, type View } from "./project";
import type { ArcSpec, PixiSceneIn } from "./scene";

/** How many packets ride an arc, and how long one takes from end to end. */
const PACKETS = 3;
const TRIP_MS = 2400;
/** How much of the arc a packet's tail covers behind it. */
const TAIL = 0.07;

const colour = (c: Rgba) => ({ color: hex(c), alpha: c[3] });

/** Whether an arc's packets move. */
export const arcsMoving = (s: PixiSceneIn, clock: Clock): boolean => clock.pulse > 0 && s.arcs.some((one) => one.loop);

/** Where along the arc each packet is: travelling where it moves, else spread evenly along it. */
export const packetsAt = (arc: Pick<ArcSpec, "loop">, clock: Pick<Clock, "now" | "pulse">): number[] => {
  const start = arc.loop && clock.pulse > 0 ? turn(clock.now, TRIP_MS) : 0.5 / PACKETS;
  return Array.from({ length: PACKETS }, (_, k) => (start + k / PACKETS) % 1);
};

/** Each arc and its packets, in screen space on the effects' graphic. */
export function drawArcs(g: G, s: PixiSceneIn, p: Palette, view: View, clock: Clock): void {
  for (const arc of s.arcs) {
    const n = 28;
    const points = Array.from({ length: n + 1 }, (_, i) => project(view, alongOf(arc.from, arc.to, arc.lift, i / n)));
    if (points.some((one) => one.scale <= 0)) continue;
    const u = view.eye * points[n >> 1]!.scale;
    const path = (width: number, alpha: number) => {
      g.moveTo(points[0]!.x, points[0]!.y);
      for (const one of points.slice(1)) g.lineTo(one.x, one.y);
      g.stroke({ color: hex(p.accent), alpha, width: width * u, cap: "round", join: "round" });
    };
    path(5, 0.12);
    path(1.4, 0.75);
    for (const t of packetsAt(arc, clock)) {
      const head = project(view, alongOf(arc.from, arc.to, arc.lift, t));
      const tail = project(view, alongOf(arc.from, arc.to, arc.lift, Math.max(0, t - TAIL)));
      g.moveTo(tail.x, tail.y).lineTo(head.x, head.y).stroke({ color: hex(p.accent), alpha: 0.45, width: 2.4 * u, cap: "round" });
      g.roundRect(head.x - 4.5 * u, head.y - 3 * u, 9 * u, 6 * u, 1.5 * u).fill(colour(mix(p.accent, p.fg, 0.25)));
    }
  }
}
