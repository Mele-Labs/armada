// A hand-drawn line as an SVG path. Its own module so a picture of a sketch can draw the ink without React Flow.

import type { SketchPoint } from "./ink-types";

/**
 * The points as one path, curved through their midpoints rather than joined
 * corner to corner — a thinned freehand line drawn as straight segments reads
 * as a saw, which is not what the hand did.
 */
export function pathOf(points: readonly SketchPoint[]): string {
  const first = points[0];
  if (first === undefined) return "";
  let d = `M ${String(first.x)} ${String(first.y)}`;
  for (let at = 1; at < points.length - 1; at += 1) {
    const here = points[at]!;
    const next = points[at + 1]!;
    const mid = { x: (here.x + next.x) / 2, y: (here.y + next.y) / 2 };
    d += ` Q ${String(here.x)} ${String(here.y)} ${String(mid.x)} ${String(mid.y)}`;
  }
  const last = points[points.length - 1]!;
  return points.length === 1 ? d : `${d} L ${String(last.x)} ${String(last.y)}`;
}
