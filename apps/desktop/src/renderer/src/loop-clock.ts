// Two tiers for every loop that never ends. A small loop, a spinner or a row's pulse, runs at the display's rate, since
// with no blur behind the panels its frames cost little. The Cockpit's map and console, and any loop large enough to
// redraw much of the window, stay on one clock that moves them all together: their frames are the dear ones, and ten
// loops on the clock cost one frame a tick, the same as one (`docs/contracts/design-system/motion.md`).

/** 5 frames a second: a pulse still reads as a pulse, and a drone as moving. */
const TICK_MS = 200;

/** An animation that ends takes the clock too if it runs this long, since nobody sees its steps: one changed row's 45-second fade held the window's frames for all 45. */
const SLOW_MS = 5_000;

/** A region whose loops take the clock however small they are: the Cockpit's map and console, where a frame redraws a canvas. */
const ON_CLOCK = "[data-loop-clock]";

/** A loop whose box covers more than this share of the window redraws enough of it each frame to take the clock. */
const LARGE = 0.1;

const followers = new Set<() => void>();

/** Draws on the clock's own tick, for a canvas's loop, so it costs no frame of its own. Not called in a window that is still or idle. Returns the way to stop. */
export function onLoopTick(follow: () => void): () => void {
  followers.add(follow);
  return () => followers.delete(follow);
}

/** Whether a loop moves on the clock rather than at the display's rate, decided once when the clock first meets it. */
function takesClock(target: Element | null, endless: boolean): boolean {
  if (!endless || target == null) return true;
  if (target.closest(ON_CLOCK) !== null) return true;
  const box = target.getBoundingClientRect();
  return box.width * box.height > LARGE * window.innerWidth * window.innerHeight;
}

/**
 * Meets each infinite or slow animation as it appears and puts it in its tier, keeping its phase. A window nobody is
 * using moves neither tier, and a loop out of sight moves nothing anyone would see, though its move would still cost a
 * frame. `styles/index.css` still decides what stands still.
 */
export function startLoopClock(): () => void {
  const root = document.documentElement;
  const offsets = new WeakMap<Animation, number>();
  const free = new WeakSet<Animation>();
  const held = new WeakSet<Animation>();
  const unseen = new WeakSet<Element>();
  const watch = new IntersectionObserver((changes) => {
    for (const one of changes) {
      if (one.isIntersecting) unseen.delete(one.target);
      else unseen.add(one.target);
    }
  });
  const tick = (): void => {
    const still = root.hasAttribute("data-still");
    const idle = root.hasAttribute("data-idle");
    if (!still && !idle) for (const follow of followers) follow();
    const now = Number(document.timeline.currentTime);
    for (const one of document.getAnimations()) {
      const timing = one.effect?.getComputedTiming();
      if (timing === undefined || one.playState === "finished") continue;
      const endless = timing.iterations === Infinity;
      if (!endless && !(Number(timing.duration) >= SLOW_MS)) continue;
      const target = (one.effect as KeyframeEffect).target;
      if (!free.has(one) && !offsets.has(one)) {
        if (target != null) watch.observe(target);
        if (takesClock(target, endless)) {
          offsets.set(one, Number(one.currentTime) - now);
          one.pause();
        } else free.add(one);
      }
      const hold = still || idle || (target != null && unseen.has(target));
      if (free.has(one)) {
        // A free loop runs on its own and is paused here only while it should hold. Once played from here it no longer
        // follows the still mark's CSS, so the still mark holds it here too.
        if (hold && one.playState === "running") {
          one.pause();
          held.add(one);
        } else if (!hold && held.has(one)) {
          one.play();
          held.delete(one);
        }
        continue;
      }
      if (hold) continue;
      const offset = offsets.get(one)!;
      if (!endless && now + offset >= Number(timing.endTime)) one.finish();
      else one.currentTime = now + offset;
    }
  };
  const timer = setInterval(tick, TICK_MS);
  return () => {
    clearInterval(timer);
    watch.disconnect();
  };
}
