// Where the battlefield gets what a Session has built. **Fleet serves no task list and no walk-shown entry yet**, so the window's
// source is empty and the field draws no structure; the mock hands a stand-in. The same seam as the directories (`areas-source.tsx`).

import { createContext, useContext, useSyncExternalStore, type ReactNode } from "react";

import type { ProgressFeed } from "./progress";

export type ProgressSource = { get: () => ProgressFeed | undefined; subscribe: (on: () => void) => () => void };

const NONE: ProgressSource = { get: () => undefined, subscribe: () => () => undefined };
const Source = createContext<ProgressSource>(NONE);

/** Hands the window a source of progress. Nothing handed, nothing is drawn. */
export function ProgressFrom({ source, children }: { source: ProgressSource | undefined; children: ReactNode }) {
  return source === undefined ? <>{children}</> : <Source.Provider value={source}>{children}</Source.Provider>;
}

export function useProgress(): ProgressFeed | undefined {
  const source = useContext(Source);
  return useSyncExternalStore(source.subscribe, source.get);
}
