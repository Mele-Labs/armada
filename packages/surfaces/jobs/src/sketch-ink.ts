// What the owner drew on top of a sketch, kept for the window. **Mock only**: it lives as long as the
// page does and is keyed by the sketch it was drawn on, so coming back to that sketch (hovering an
// option again) finds the pen's lines where they were.

import type { SketchStroke } from "@armada/components";
import { useSyncExternalStore } from "react";

const kept = new Map<string, readonly SketchStroke[]>();
const listeners = new Set<() => void>();
const NONE: readonly SketchStroke[] = [];

/** A short key for the scene a sketch draws. */
export function inkKey(scene: unknown): string {
  const text = JSON.stringify(scene) ?? "";
  let hash = 0;
  for (let at = 0; at < text.length; at += 1) hash = (hash * 31 + text.charCodeAt(at)) | 0;
  return String(hash);
}

export function useInk(key: string | undefined): [readonly SketchStroke[], (next: readonly SketchStroke[]) => void] {
  const strokes = useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => (key === undefined ? NONE : (kept.get(key) ?? NONE)),
    () => NONE,
  );
  return [
    strokes,
    (next) => {
      if (key === undefined) return;
      kept.set(key, next);
      listeners.forEach((one) => one());
    },
  ];
}
