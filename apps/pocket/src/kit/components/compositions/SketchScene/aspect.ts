// The shape a sketch's box takes, from the scene's own bounds.

import type { Scene } from "./scene";

/** A node with no size of its own is drawn about this big. */
const NODE_W = 160;
const NODE_H = 60;

/**
 * The shape a picture's box takes, width over height, from the scene's own bounds: a wide flow is a short box and a tall
 * rail a tall one, kept between a tall phone screen and a strip, so the fitted drawing fills it.
 */
export function aspectOf(scene: Scene): number {
  if (scene.nodes.length === 0) return 16 / 9;
  let left = Infinity;
  let top = Infinity;
  let right = -Infinity;
  let bottom = -Infinity;
  for (const node of scene.nodes) {
    const w = node.w ?? NODE_W;
    const h = node.h ?? (node.kind === "mock" ? w * 0.4 : NODE_H);
    left = Math.min(left, node.x);
    top = Math.min(top, node.y);
    right = Math.max(right, node.x + w);
    bottom = Math.max(bottom, node.y + h);
  }
  const pad = 48;
  return Math.min(3, Math.max(0.6, (right - left + pad) / (bottom - top + pad)));
}
