// What a right press landed on, for the menus that open there: the annotation layer's and the app's.

/** Whether the target is the layer's own, or the mock's walk card, which a walk over the layer has to be able to use. */
export function inLayer(target: EventTarget | null): boolean {
  const element = target instanceof Element ? target : target instanceof Node ? target.parentElement : null;
  return element?.closest("[data-armada-annotate], [data-walk-ui]") != null;
}

/** What the pointer is over, taking an SVG as the whole glyph rather than one path in it. */
export function targetOf(target: EventTarget | null): Element | null {
  if (!(target instanceof Element)) return null;
  return target.closest("svg") ?? target;
}
