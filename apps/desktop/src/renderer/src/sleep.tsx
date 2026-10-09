// Sleep mode: the title row's moon, and the decisions it made overnight, which the Cockpit draws as
// calls (`Dashboard`'s `useItems`). The state lives behind a source, the way Phone's does, so a Bridge
// with no source draws neither. The real one is `fleet-sleep.ts`, over Fleet's `/sleep` routes; the
// mock's is `mock/sleep.ts`.

import { createContext, useContext, useSyncExternalStore } from "react";
import type { MorningBlocked, MorningDecided, MorningLanded, TheShellProps } from "@armada/components";

export type SleepState = {
  on: boolean;
  decided: readonly MorningDecided[];
  blocked: readonly MorningBlocked[];
  landed: readonly MorningLanded[];
  walks: readonly { id: string; who: string; title: string }[];
};

export type SleepSource = {
  get(): SleepState;
  subscribe(on: () => void): () => void;
  toggle(): void;
  override(id: string, text: string): void;
  /** The owner kept a decision: its call leaves the Cockpit. Renderer-side for now; not on the wire. */
  review(id: string): void;
};

const SleepContext = createContext<SleepSource | null>(null);
export const SleepSourceProvider = SleepContext.Provider;

const NO_STATE: SleepState = { on: false, decided: [], blocked: [], landed: [], walks: [] };

const NONE: SleepSource = {
  get: () => NO_STATE,
  subscribe: () => () => undefined,
  toggle: () => undefined,
  override: () => undefined,
  review: () => undefined,
};

/** The source, where there is one. */
export const useSleepSource = (): SleepSource | null => useContext(SleepContext);

/** What Sleep decided for the owner and he has not yet kept or changed. */
export function useSleepDecided(): readonly MorningDecided[] {
  const here = useSleepSource() ?? NONE;
  return useSyncExternalStore(here.subscribe, here.get).decided;
}

/** The title row's control. */
export function useSleepMode(): { sleep: TheShellProps["sleep"] } {
  const source = useSleepSource();
  const state = useSyncExternalStore((source ?? NONE).subscribe, (source ?? NONE).get);
  if (source === null) return { sleep: undefined };
  return { sleep: { on: state.on, onToggle: source.toggle } };
}
