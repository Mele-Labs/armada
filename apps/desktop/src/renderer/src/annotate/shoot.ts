// A picture of the app under the layer. Main's capture shoots the whole window, the layer's
// outline, pins, card and bar with it, so the layer is drawn transparent while it is taken.

/** The attribute `annotate.css` draws the layer transparent on. */
export const SHOOTING = "data-shooting";

const out = new WeakMap<Element, number>();

/** Two frames: the first runs before the paint that drops the layer, the second after it. */
const painted = (frame: (done: () => void) => unknown): Promise<void> =>
  new Promise((done) => frame(() => frame(() => done())));

/**
 * `take`, once a frame without the layer has been painted. Counted, so two overlapping shots
 * never put the layer back while the other is still out.
 */
export async function withoutLayer<T>(
  layer: Element | null,
  take: () => Promise<T>,
  frame: (done: () => void) => unknown = requestAnimationFrame,
): Promise<T> {
  if (layer === null) return take();
  out.set(layer, (out.get(layer) ?? 0) + 1);
  layer.setAttribute(SHOOTING, "");
  try {
    await painted(frame);
    return await take();
  } finally {
    const left = (out.get(layer) ?? 1) - 1;
    out.set(layer, left);
    if (left === 0) layer.removeAttribute(SHOOTING);
  }
}
