// The small helpers the map and its hit layer share: the plane's own mapping of a point, and the press that acts on Enter or Space.
// Ported from proto/cockpit-flight-deck. #2301.

import { FLAT, type Point } from "./layout";

/** A point of the layout on the plane, whose depth is stretched so that the tilted ground reads at its own height. */
export const onPlane = (p: Point): Point => ({ x: p.x, y: p.y * FLAT });
export const keyed = (event: { key: string; preventDefault: () => void }, act: () => void) => {
  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    act();
  }
};
