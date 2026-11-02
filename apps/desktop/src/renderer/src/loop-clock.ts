// One clock for every loop that never ends. A running animation holds the window's frames at the
// display's rate and each frame redraws the blurred panels, so one faint pulse alone held the GPU at
// 52% (`docs/contracts/design-system/motion.md`). Steps on each loop's own schedule add up, so here the
// loops are paused and all moved together: ten of them cost one frame a tick, the same as one.

/** 5 frames a second: a pulse still reads as a pulse, and a spinner as turning. */
const TICK_MS = 200;

/** Takes over each infinite animation as it appears, and keeps its phase. A window nobody is using moves none of them, and a loop out of sight moves nothing anyone would see, though its move would still cost a frame. `styles/index.css` still decides what stands still. */
export function startLoopClock(): () => void {
  const root = document.documentElement;
  const offsets = new WeakMap<Animation, number>();
  const unseen = new WeakSet<Element>();
  const watch = new IntersectionObserver((changes) => {
    for (const one of changes) {
      if (one.isIntersecting) unseen.delete(one.target);
      else unseen.add(one.target);
    }
  });
  const tick = (): void => {
    if (root.hasAttribute("data-still")) return;
    const now = Number(document.timeline.currentTime);
    const idle = root.hasAttribute("data-idle");
    for (const one of document.getAnimations()) {
      if (one.effect?.getComputedTiming().iterations !== Infinity) continue;
      const target = (one.effect as KeyframeEffect).target;
      let offset = offsets.get(one);
      if (offset === undefined) {
        offset = Number(one.currentTime) - now;
        offsets.set(one, offset);
        one.pause();
        if (target != null) watch.observe(target);
      }
      if (idle || (target != null && unseen.has(target))) continue;
      one.currentTime = now + offset;
    }
  };
  const timer = setInterval(tick, TICK_MS);
  return () => {
    clearInterval(timer);
    watch.disconnect();
  };
}
