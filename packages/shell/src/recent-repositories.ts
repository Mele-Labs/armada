// The repositories a person picked last, so a picker over many puts them first.
// Per window, in `localStorage` like the widths beside it: it is how this person
// reads Bridge, not something Fleet knows.

import { useCallback, useState } from "react";

export const RECENT_KEY = "armada.bridge.recent-repositories";

/** How many the picker leads with. More is a second list to search. */
export const RECENT_REPOSITORIES = 3;

function read(): string[] {
  try {
    const raw = JSON.parse(window.localStorage.getItem(RECENT_KEY) ?? "[]") as unknown;
    return Array.isArray(raw) ? raw.filter((one): one is string => typeof one === "string") : [];
  } catch {
    return [];
  }
}

/** `root` first and each root once, cut to the few the picker leads with. */
export function remembering(roots: readonly string[], root: string): string[] {
  return [root, ...roots.filter((one) => one !== root)].slice(0, RECENT_REPOSITORIES);
}

/** The last few roots picked, newest first, and the call that adds one. */
export function useRecentRepositories(): [readonly string[], (root: string) => void] {
  const [recent, setRecent] = useState<string[]>(read);
  const remember = useCallback((root: string) => {
    setRecent((now) => {
      const next = remembering(now, root);
      try {
        window.localStorage.setItem(RECENT_KEY, JSON.stringify(next));
      } catch {
        // Private window or blocked storage: the picker just forgets between launches.
      }
      return next;
    });
  }, []);
  return [recent, remember];
}
