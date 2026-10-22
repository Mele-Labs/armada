// The lines a drone leaves in the air and the beam it hangs from: the contrail of a launch, the tether to the bay it holds, the beams to the
// plots it changes, the line between two drones changing one area. Real lines through the scene,
// so they stand right at any pitch. The beam is a dashed vertical from the drone's shadow up to its foot, with a light along it.

import { FOOT } from "../geometry";
import { alongOf, type P3 } from "../layout";
import { TRAIL_MS } from "../plays";
import type { Clock } from "./clock";
import { arrival } from "./ground";
import { hueOf } from "./drones";
import { PlaneBuilder } from "./plane";
import { fade, type Palette } from "./palette";
import type { PixiSceneIn } from "./scene";

/** An arc between two points, rising `lift` at the middle, as far along as `to` of the way. */
const arc = (a: P3, b: P3, lift: number, to = 1, n = 18): P3[] => Array.from({ length: n + 1 }, (_, i) => alongOf(a, b, lift, (i / n) * to));

export function drawLifted(b: PlaneBuilder, glow: PlaneBuilder, s: PixiSceneIn, p: Palette, floor: (tile: string | undefined) => number, clock: Clock): void {
  for (const spec of s.drones) {
    const d = spec.drone;
    // Another Job's drone hangs from no beam and leaves no line.
    if (spec.ghost) continue;
    const hold = d.shown ? 1 : 0.12;
    const to = spec.flying;
    const launching = spec.launch !== undefined && clock.pulse > 0;
    if (spec.trail !== undefined) {
      const drawn = launching ? Math.min(1, Math.max(0, clock.since(spec.launch!.id, spec.launch!.delay) / (clock.pulse * 1.2))) : 1;
      if (drawn > 0) b.stroke(arc(spec.origin, to, 46, drawn), 0.65, fade(p.muted, Math.max(0.05, 0.2 * (1 - spec.trail / TRAIL_MS)) * hold));
    }
    if (spec.pad !== undefined) b.stroke(arc({ ...spec.pad, z: 2 }, to, 30), 0.55, fade(p.subtle, 0.55 * hold), { dash: [1, 4] });
    // Off the code floor it hangs from nothing: thinking over its file, or over the pad.
    if (d.off !== undefined) continue;
    spec.touches.forEach(({ tile, at }, i) => {
      const top: P3 = { ...at, z: s.heights.get(tile) ?? 0 };
      const tone = spec.selected ? fade(p.accent, 0.85) : fade(p.muted, i === 0 ? 0.62 : 0.34);
      const line = (c: typeof tone) => fade(c, c[3] * hold);
      b.stroke([to, top], 0.55, line(tone), { dash: [1.2, 4.5] });
      b.stroke(PlaneBuilder.arc(top.x, top.y, 8, 0, Math.PI * 2, top.z + 0.3, 20), 0.55, line(spec.selected ? fade(p.accent, 0.85) : fade(p.muted, 0.45)), { closed: true });
    });
    // The beam: from its shadow up to its foot, bright toward the drone and dim toward the ground.
    if (d.hover) {
      const [low, high] = [floor(spec.tile) + 1, Math.max(floor(spec.tile) + 1, spec.alt - FOOT)];
      const keep = (spec.away ? 0.12 : spec.dim ? 0.4 : 1) * arrival(spec, clock);
      if (high > low && keep > 0) {
        const hue = hueOf(d.hue, p);
        const column = [{ x: spec.at.x, y: spec.at.y, z: low }, { x: spec.at.x, y: spec.at.y, z: high }];
        const along = (q: P3) => (q.z - low) / (high - low);
        const up = { normal: { x: 1, y: 0 } };
        b.stroke(column, 0.5, hue, { ...up, dash: [3, 5], shade: (q) => fade(hue, (0.4 + 0.5 * along(q)) * keep) });
        glow.stroke(column, 4, hue, { ...up, shade: (q) => fade(hue, (0.04 + 0.2 * along(q)) * keep) });
      }
    }
  }
  for (const clash of s.clashLines) {
    const line = arc(clash.from, clash.to, 22);
    b.stroke(line, 0.9, fade(p.ask, 0.9), { dash: [5, 4] });
    glow.stroke(line, 3, fade(p.ask, 0.16));
  }
}
