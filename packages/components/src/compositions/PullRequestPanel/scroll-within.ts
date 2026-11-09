/**
 * Bring `target` to the top (or middle) of the nearest ancestor that really scrolls, and move nothing else.
 * `scrollIntoView` also scrolls every `overflow: hidden` ancestor by whatever the real scroller could not
 * absorb, so a file near the end of a diff shifted the whole window's layout up.
 * A box with only `overflow-x: auto` computes `overflow-y: auto` too, so a box counts only when it has
 * rows to scroll; otherwise a line's own diff box was picked and the pane never moved.
 */
export function scrollWithin(target: HTMLElement, align: "start" | "center"): void {
  const box = scroller(target, true) ?? scroller(target, false);
  if (box === null) return;
  const at = target.getBoundingClientRect();
  const margin = parseFloat(getComputedStyle(target).scrollMarginTop) || 0;
  const offset = align === "center" ? (box.clientHeight - at.height) / 2 : margin;
  box.scrollTo({
    top: box.scrollTop + at.top - box.getBoundingClientRect().top - offset,
  });
}

/** The nearest ancestor that may scroll rows; with `overflowing`, only one that has rows to scroll now. */
function scroller(target: HTMLElement, overflowing: boolean): HTMLElement | null {
  for (let box = target.parentElement; box !== null; box = box.parentElement) {
    if (!/auto|scroll/.test(getComputedStyle(box).overflowY)) continue;
    if (!overflowing || box.scrollHeight > box.clientHeight) return box;
  }
  return null;
}
