/**
 * Bring `target` to the top (or middle) of the nearest ancestor that really scrolls, and move nothing else.
 * `scrollIntoView` also scrolls every `overflow: hidden` ancestor by whatever the real scroller could not
 * absorb, so a file near the end of a diff shifted the whole window's layout up.
 */
export function scrollWithin(target: HTMLElement, align: "start" | "center"): void {
  let box: HTMLElement | null = target.parentElement;
  while (box !== null && !/auto|scroll/.test(getComputedStyle(box).overflowY)) box = box.parentElement;
  const scroller = box ?? document.scrollingElement;
  if (scroller === null) return;
  const edge = box === null ? 0 : box.getBoundingClientRect().top;
  const at = target.getBoundingClientRect();
  const margin = parseFloat(getComputedStyle(target).scrollMarginTop) || 0;
  const room = box === null ? window.innerHeight : box.clientHeight;
  const offset = align === "center" ? (room - at.height) / 2 : margin;
  scroller.scrollTo({
    top: scroller.scrollTop + at.top - edge - offset,
  });
}
