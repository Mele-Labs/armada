// Ending a Job leaves at once: Fleet parks it in the background, and Bridge does not wait for that. Every End (the head's menu, the done
// band, a call card, a row) goes through `close` in the wired Sessions, which announces it here; the stage answers by playing the
// Job's pane out and showing the timeline with nothing selected.

import { useSyncExternalStore } from "react";

/** The exit lasts this long at most, whatever the animation says: a ghost that never ends would cover the timeline. */
export const EXIT_MS = 400;

/** True while a pane is leaving, for the stage to wear. */
let exiting = false;

const ends = new Set<(id: string) => void>();
const looks = new Set<() => void>();

/** Job `id` was ended. */
export const announceEnd = (id: string): void => {
  for (const listen of ends) listen(id);
};
/** Called by the stage that holds the Job's pane. */
export const onEnd = (listen: (id: string) => void): (() => void) => {
  ends.add(listen);
  return () => void ends.delete(listen);
};

export const setExiting = (next: boolean): void => {
  if (exiting === next) return;
  exiting = next;
  for (const look of looks) look();
};
export const useExiting = (): boolean =>
  useSyncExternalStore(
    (look) => (looks.add(look), () => void looks.delete(look)),
    () => exiting,
  );

export const reducedMotion = (): boolean => typeof window.matchMedia === "function" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/** A token's duration in ms, read where the stylesheet sets it, never past the cap. */
function tokenMs(name: string, fallback: number): number {
  const said = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  const value = parseFloat(said);
  if (Number.isNaN(value)) return fallback;
  return Math.min(said.endsWith("ms") ? value : value * 1000, EXIT_MS);
}

/**
 * The pane open on the stage leaves. **A copy of its DOM is laid over it at the measured rect, before anything changes**, so the
 * press shows the same picture; then `commit` clears the selection in the same tick (the timeline's mount is the heavy part), and
 * the copy's animation starts on the frame after, so its whole span plays instead of being spent on that mount. Web Animations drive
 * it, with the durations and easing the tokens carry, and its end removes the copy; a late timer is only the backstop.
 */
export function leaveFrom(key: string, commit: () => void): void {
  const pane = document.querySelector<HTMLElement>(".deck-stage > .deck-pane");
  const stage = pane?.parentElement;
  if (pane === null || pane === undefined || stage === null || stage === undefined) return commit();
  const at = pane.getBoundingClientRect();
  const from = stage.getBoundingClientRect();
  const row = document.querySelector<HTMLElement>(`[data-row="${CSS.escape(key)}"]`)?.getBoundingClientRect();
  // The pane folds into its row; with the row off screen it has nowhere to go and fades.

  const ghost = document.createElement("div");
  ghost.className = "deck-ghost";
  ghost.setAttribute("data-ghost", "");
  ghost.setAttribute("aria-hidden", "true");
  ghost.inert = true;
  Object.assign(ghost.style, { left: `${at.left - from.left}px`, top: `${at.top - from.top}px`, width: `${at.width}px`, height: `${at.height}px` });
  const copy = pane.cloneNode(true) as HTMLElement;
  ghost.append(copy);
  stage.append(ghost);
  // A copy starts every scroller at the top; the thread is where it was.
  const was = pane.querySelectorAll<HTMLElement>("*");
  const now = copy.querySelectorAll<HTMLElement>("*");
  was.forEach((one, i) => {
    if (one.scrollTop > 0) now[i]!.scrollTop = one.scrollTop;
  });

  const easing = getComputedStyle(document.documentElement).getPropertyValue("--ease").trim() || "ease";
  const span = row === undefined ? tokenMs("--duration-base", 180) : tokenMs("--duration-travel", 300);
  const frames: Keyframe[] =
    row === undefined
      ? [{ opacity: 1 }, { opacity: 0 }]
      : [
          { opacity: 1, transform: "none" },
          { opacity: 0.6, offset: 0.6 },
          { opacity: 0, transform: `translate(${row.left - at.left}px, ${row.top - at.top}px) scale(${row.width / at.width}, ${row.height / at.height})` },
        ];

  let finished = false;
  const finish = () => {
    if (finished) return;
    finished = true;
    ghost.remove();
    if (row !== undefined) landed(key);
  };
  // The timeline's own entrance outlasts a short exit, so the stage keeps its mark until that is over too.
  const rest = () => setExiting(false);
  window.setTimeout(() => (finish(), rest()), 1000);

  setExiting(true);
  commit();
  window.requestAnimationFrame(() => {
    if (finished) return;
    const run = ghost.animate(frames, { duration: span, easing, fill: "forwards" });
    window.setTimeout(rest, tokenMs("--duration-travel", 300) + 50);
    void run.finished.then(finish, finish);
  });
}

/** The row a pane folded into takes its mark for a moment. */
function landed(key: string): void {
  const row = document.querySelector<HTMLElement>(`[data-row="${CSS.escape(key)}"]`);
  if (row === null) return;
  row.setAttribute("data-landed", "");
  window.setTimeout(() => row.removeAttribute("data-landed"), EXIT_MS);
}
