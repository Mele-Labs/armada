// Where a Job's Lanes open is the window's width to decide, never a second button. Wide, they are a view in the Job's strip and open in the spine under it as Artifacts and Checks do; narrower, they are
// the app's side panel over the spine. One button and one key (⌥⇧L) serve both, and a window resized across the line carries what is open to the other place.

import { useSyncExternalStore } from "react";

import { closePanel, lanesOpenFor, openInPanel, useLanesOpen } from "../shared-panel/store";
import { closeLayer, layerOver, showLayer, toggleLayer, useLayer } from "./layers";

/** The width from which Lanes are a view in the spine. The one number: nothing else in the app reads the window at this width. */
export const WIDE_WINDOW_PX = 1800;

const QUERY = `(min-width: ${WIDE_WINDOW_PX}px)`;

// A mock's switch: a walk cannot resize its window, so a scenario can say the window is wide (or narrow) for a step. Unset, the window decides.
let forced: boolean | undefined;
const hear = new Set<() => void>();

export function forceWideWindow(on: boolean | undefined): void {
  forced = on;
  for (const one of hear) one();
}

/** Whether the window is wide enough for Lanes to open in the spine. */
export function isWideWindow(): boolean {
  return forced ?? (typeof window.matchMedia === "function" && window.matchMedia(QUERY).matches);
}

/** `isWideWindow`, redrawn when the window crosses the line. */
export function useWideWindow(): boolean {
  return useSyncExternalStore(
    (listener) => {
      hear.add(listener);
      const query = typeof window.matchMedia === "function" ? window.matchMedia(QUERY) : undefined;
      query?.addEventListener("change", listener);
      return () => {
        hear.delete(listener);
        query?.removeEventListener("change", listener);
      };
    },
    isWideWindow,
  );
}

/** The Job's Lanes are open, in the spine or in the panel. */
export function lanesOpen(sessionId: string): boolean {
  return layerOver(sessionId) === "lanes" || lanesOpenFor(sessionId);
}

/** Opens the Job's Lanes where the window's width puts them, or shuts them where they are open. */
export function toggleLanes(sessionId: string): void {
  if (lanesOpen(sessionId)) {
    if (layerOver(sessionId) === "lanes") closeLayer();
    if (lanesOpenFor(sessionId)) closePanel();
    return;
  }
  if (isWideWindow()) toggleLayer(sessionId, "lanes");
  else openInPanel({ kind: "lanes", sessionId });
}

/** Carries open Lanes to the place the window's width now puts them. Answers nothing: a Job with Lanes shut is left as it is. */
export function placeLanes(sessionId: string): void {
  const wide = isWideWindow();
  if (wide && lanesOpenFor(sessionId)) {
    closePanel();
    showLayer(sessionId, "lanes");
  } else if (!wide && layerOver(sessionId) === "lanes") {
    closeLayer();
    openInPanel({ kind: "lanes", sessionId });
  }
}

/** Whether this Job's Lanes are open, for the strip's button to show it pressed. */
export function useLanesView(sessionId: string): boolean {
  const inPanel = useLanesOpen(sessionId);
  return useLayer(sessionId) === "lanes" || inPanel;
}
