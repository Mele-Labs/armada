// The small parts the battlefield and the mission view share: a picture standing upright in the scene, the plane's own mapping of a
// point, and the helpers that place one. Mock only. proto/cockpit-flight-deck.

import type { CSSProperties } from "react";

import { FLAT, type Point } from "./deck-field";

export const vars = (values: Record<string, string | number>) => values as CSSProperties;
export const px = (value: number) => `${value.toFixed(1)}px`;
/** A point of the layout on the plane, whose depth is stretched so that the tilted ground reads at its own height. */
export const onPlane = (p: Point): Point => ({ x: p.x, y: p.y * FLAT });
export const keyed = (event: { key: string; preventDefault: () => void }, act: () => void) => {
  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    act();
  }
};

/**
 * A flat picture standing in the scene, upright toward the viewer, its origin on the anchor. The billboard is sized round
 * the picture and turns about the anchor: a box of no size, or a picture drawn off its own origin, is clipped by the layer. The viewBox starts at zero so that a transform-origin of 0 0 in the CSS is the anchor.
 */
export function Sprite({ x, y, w, h, label, children }: { x: number; y: number; w: number; h: number; label?: boolean; children: React.ReactNode }) {
  return (
    <div className={label === undefined ? "bf-bb" : "bf-bb bf-tag"} style={{ left: x, top: y, width: w, height: h, transformOrigin: `${-x}px ${-y}px` }}>
      <svg className="bf-sprite" width={w} height={h} viewBox={`0 0 ${w} ${h}`} aria-hidden="true">
        <g transform={`translate(${-x} ${-y})`}>{children}</g>
      </svg>
    </div>
  );
}

