// Where the walk's card rests. It sits in the trailing bottom corner and takes the trailing top
// one, under the app's top bar, when the step's ring is under it, so it never covers what it
// points at. Pure, so the rule is checked without drawing a window. `WalkPlayer.tsx` uses it.

export type Box = { left: number; right: number; top: number; bottom: number };

export const overlaps = (a: Box, b: Box): boolean => a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top;

/**
 * The edge the card rests on: the bottom one unless the ring is under it, then the top one unless
 * the ring is under that too, where it stays at the bottom. `gap` is the card's distance from the
 * window's edge and `top` how far down the top edge starts.
 */
export function placeCard(ring: Box | null, size: { width: number; height: number }, view: { width: number; height: number }, gap: number, top: number): "bottom" | "top" {
  if (ring === null) return "bottom";
  const left = view.width - gap - size.width;
  const bottom: Box = { left, right: view.width - gap, top: view.height - gap - size.height, bottom: view.height - gap };
  const above: Box = { left, right: view.width - gap, top, bottom: top + size.height };
  return overlaps(ring, bottom) && !overlaps(ring, above) ? "top" : "bottom";
}

/** A dragged spot kept inside the window, so a card is never lost off its edge. */
export function keepInside(spot: { x: number; y: number }, size: { width: number; height: number }, view: { width: number; height: number }): { x: number; y: number } {
  return { x: Math.min(Math.max(0, spot.x), Math.max(0, view.width - size.width)), y: Math.min(Math.max(0, spot.y), Math.max(0, view.height - size.height)) };
}
