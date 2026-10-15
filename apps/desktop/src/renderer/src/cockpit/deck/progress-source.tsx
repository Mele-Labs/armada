// Where the battlefield gets what a Session has built. **Fleet serves no task list and no walk-shown entry yet**, so the window's
// source is empty and the field draws no structure; the mock hands a stand-in. The same seam as the directories (`areas-source.tsx`).

import { createContext, useContext, useMemo, useSyncExternalStore, type ReactNode } from "react";

import { builtOf, type After, type Built, type ProgressFeed, type RoadAt, type Take } from "./progress";

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

/** What each Session and Job has built, read once, by drone key. Empty where Fleet reports none. */
export function useBuilds(): { by: ReadonlyMap<string, Built>; take: Take; after: After; road: RoadAt } {
  const feed = useProgress();
  return useMemo(() => ({ by: new Map((feed?.views ?? []).map((view) => [view.key, builtOf(view)] as const)), take: feed?.take ?? "tower", after: feed?.after ?? "clear", road: feed?.road ?? "outer" }), [feed]);
}
