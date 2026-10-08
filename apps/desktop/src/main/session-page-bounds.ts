// Where the page view is placed: the rect the renderer reported, kept inside the window.
//
// **The renderer's number is not trusted**: a missing, negative or infinite one would place a
// view over the whole of Bridge or off it, so each is rounded and held to the window's content.

import type { PageBounds } from "@armada/screens/src/draft/sessions";

/** Whether a key event in the page's view is Esc going down, which the panel answers by closing. */
export const isEscape = (input: { type: string; key: string }): boolean => input.type === "keyDown" && input.key === "Escape";

const whole = (value: unknown): number => (typeof value === "number" && Number.isFinite(value) ? Math.round(value) : 0);

export function boundsOf(value: unknown, content: { width: number; height: number }): PageBounds {
  const box = (typeof value === "object" && value !== null ? value : {}) as Record<string, unknown>;
  const x = Math.min(Math.max(whole(box["x"]), 0), content.width);
  const y = Math.min(Math.max(whole(box["y"]), 0), content.height);
  return {
    x,
    y,
    width: Math.min(Math.max(whole(box["width"]), 0), content.width - x),
    height: Math.min(Math.max(whole(box["height"]), 0), content.height - y),
  };
}
