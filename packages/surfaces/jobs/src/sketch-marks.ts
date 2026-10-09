// What the owner did to each sketch, kept for the window. **Mock only**: it lives as long as the page
// does and is keyed by the scene it was done on, so coming back to that sketch (hovering an option
// again) finds his lines, boxes, joins and strikes where they were.

import { NO_MARKS, hasMarks, type SceneMarks } from "@armada/components";
import { useMemo, useSyncExternalStore } from "react";

const kept = new Map<string, SceneMarks>();
const listeners = new Set<() => void>();
let version = 0;

/** A short key for the scene a sketch draws. */
export function marksKey(scene: unknown): string {
  const text = JSON.stringify(scene) ?? "";
  let hash = 0;
  for (let at = 0; at < text.length; at += 1) hash = (hash * 31 + text.charCodeAt(at)) | 0;
  return String(hash);
}

const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => listeners.delete(listener);
};

export function useMarks(key: string | undefined): [SceneMarks, (next: SceneMarks) => void] {
  useSyncExternalStore(subscribe, () => version, () => 0);
  return [
    key === undefined ? NO_MARKS : (kept.get(key) ?? NO_MARKS),
    (next) => {
      if (key === undefined) return;
      kept.set(key, next);
      version += 1;
      listeners.forEach((one) => one());
    },
  ];
}

/** Every sketch he marked, by key: what an answer carries beside his picks. */
export function useMarked(): Readonly<Record<string, SceneMarks>> {
  const seen = useSyncExternalStore(subscribe, () => version, () => 0);
  return useMemo(() => Object.fromEntries([...kept].filter(([, marks]) => hasMarks(marks))), [seen]);
}

/** Everything forgotten, as a fresh window would have it. For the tests, which share one page. */
export function forgetMarks(): void {
  kept.clear();
  version += 1;
  listeners.forEach((one) => one());
}
