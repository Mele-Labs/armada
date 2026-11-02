// One clock for every loop that never ends. A running animation holds the window's frames at the
// display's rate and each frame redraws the blurred panels, so one faint pulse alone held the GPU at
// 52% (`docs/contracts/design-system/motion.md`). Steps on each loop's own schedule add up, so here the
// loops are paused and all moved together: ten of them cost one frame a tick, the same as one.

/** About 10 frames a second: a pulse still reads as a pulse, and a spinner as turning. */
const TICK_MS = 100;

function held(one: Animation): boolean {
  const effect = one.effect as KeyframeEffect | null;
  if (effect?.target == null) return false;
  return getComputedStyle(
    effect.target,
    effect.pseudoElement ?? undefined,
  ).animationPlayState.includes("paused");
}

/** Takes over each infinite animation as it appears, and keeps its phase. `styles/index.css` still decides what stands still. */
export function startLoopClock(): () => void {
  const root = document.documentElement;
  const offsets = new WeakMap<Animation, number>();
  const tick = (): void => {
    if (root.hasAttribute("data-still")) return;
    const now = Number(document.timeline.currentTime);
    const idle = root.hasAttribute("data-idle");
    for (const one of document.getAnimations()) {
      if (one.effect?.getComputedTiming().iterations !== Infinity) continue;
      let offset = offsets.get(one);
      if (offset === undefined) {
        offset = Number(one.currentTime) - now;
        offsets.set(one, offset);
        one.pause();
      }
      if (idle && held(one)) continue;
      one.currentTime = now + offset;
    }
  };
  const timer = setInterval(tick, TICK_MS);
  return () => clearInterval(timer);
}
