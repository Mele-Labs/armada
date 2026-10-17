// What hangs in the air over the ground: a pull request's route from the drone that owns it, or from the base, up and over to the
// objective. A real arc through the scene, not a line shifted to look as if it were lifted, so it stands right at any pitch. Its shadow
// lies on the ground beneath. The lines that belong to drones (a trail, a tether, a beam) are the drones' to draw.

import { alongOf, type P3 } from "../layout";
import { PlaneBuilder } from "./plane";
import { fade, type Palette } from "./palette";
import type { PixiSceneIn } from "./scene";

/** The route's arc between two points, rising 34 at the middle. */
export const arcOf = (a: P3, b: P3, n = 18): P3[] => Array.from({ length: n + 1 }, (_, i) => alongOf(a, b, 34, i / n));

export function drawAir(b: PlaneBuilder, s: PixiSceneIn, p: Palette): void {
  for (const crate of s.crates) {
    if (crate.pr.stage === "landed") continue;
    const red = crate.pr.stage === "red";
    const arc = arcOf(crate.start, crate.end);
    const tone = red ? fade(p.bad, 0.7) : fade(p.muted, 0.6);
    const hold = crate.dim ? 0.12 : 1;
    b.stroke(arc.map((q) => ({ ...q, z: 0.3 })), 0.8, fade(red ? p.bad : p.subtle, 0.14 * hold), { dash: [2, 7] });
    b.stroke(arc, 0.8, fade(tone, hold), { dash: [2.4, 5.6] });
  }
}
