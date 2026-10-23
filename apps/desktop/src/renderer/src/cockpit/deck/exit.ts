// Ending a Job leaves at once: Fleet parks it in the background, and Bridge does not wait for that. Every End (the head's menu, the done
// band, a call card, a row) goes through `close` in the wired Sessions, which announces it here; the stage answers by playing the
// Job's pane out and showing the timeline with nothing selected.

import { useSyncExternalStore } from "react";

/** How the pane leaves: it folds into its row, slides out to the side, or settles away. */
export type ExitTake = "fold" | "slide" | "settle";
export const EXIT_TAKES: readonly ExitTake[] = ["fold", "slide", "settle"];

/** The exit lasts this long at most, whatever the animation says: a ghost that never ends would cover the timeline. */
export const EXIT_MS = 400;

let take: ExitTake = "fold";
/** The take on screen while a pane is leaving, for the stage to wear. */
let exiting: ExitTake | undefined;

const ends = new Set<(id: string) => void>();
const looks = new Set<() => void>();

/** Chooses the take. The mock sets it from `?take=` so a walk can offer all three. */
export const setExitTake = (next: ExitTake) => {
  take = next;
};
export const exitTake = (): ExitTake => take;

/** Job `id` was ended. */
export const announceEnd = (id: string): void => {
  for (const listen of ends) listen(id);
};
/** Called by the stage that holds the Job's pane. */
export const onEnd = (listen: (id: string) => void): (() => void) => {
  ends.add(listen);
  return () => void ends.delete(listen);
};

export const setExiting = (next: ExitTake | undefined): void => {
  if (exiting === next) return;
  exiting = next;
  for (const look of looks) look();
};
export const useExiting = (): ExitTake | undefined =>
  useSyncExternalStore(
    (look) => (looks.add(look), () => void looks.delete(look)),
    () => exiting,
  );

export const reducedMotion = (): boolean => typeof window.matchMedia === "function" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
