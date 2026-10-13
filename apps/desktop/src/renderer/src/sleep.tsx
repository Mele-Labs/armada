// Sleep mode: the title row's moon, and the decisions it made overnight, which the Cockpit draws as
// calls (`Dashboard`'s `useItems`). The state lives behind a source, the way Phone's does, so a Bridge
// with no source draws neither. The real one is `fleet-sleep.ts`, over Fleet's `/sleep` routes; the
// mock's is `mock/sleep.ts`.

import { createContext, useContext, useEffect, useSyncExternalStore } from "react";
import type { TheShellProps } from "@armada/components";

/** A question Sleep answered for the owner. `sessionId` is the Session it was decided in, `jobId` the Job, where it was one. */
export type MorningDecided = { id: string; who: string; asked: string; chose: string; corrected?: string; sessionId?: string; jobId?: string };

/** Something only the owner can settle. */
export type MorningBlocked = { id: string; who: string; text: string };

/** A pull request that merged during the night. */
export type MorningLanded = { id: string; who: string; title: string; pr: string };

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
  /** The owner is done with a decision: its call leaves the Cockpit. Renderer-side for now; not on the wire. */
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

/**
 * The title row's control. **Sleep turns itself off at the first use of Bridge** after it was turned
 * on: the first key or press anywhere in the window. The press on the moon itself is the owner's own
 * toggle, so it is left to do what it says.
 */
export function useSleepMode(): { sleep: TheShellProps["sleep"] } {
  const source = useSleepSource();
  const state = useSyncExternalStore((source ?? NONE).subscribe, (source ?? NONE).get);
  const on = state.on;
  const toggle = source?.toggle;
  useEffect(() => {
    if (!on || toggle === undefined) return;
    const used = (event: Event) => {
      if (event.target instanceof Element && event.target.closest('[aria-label="Sleep mode"]') !== null) return;
      stop();
      toggle();
    };
    const stop = () => {
      window.removeEventListener("keydown", used, true);
      window.removeEventListener("pointerdown", used, true);
    };
    window.addEventListener("keydown", used, true);
    window.addEventListener("pointerdown", used, true);
    return stop;
  }, [on, toggle]);
  if (source === null) return { sleep: undefined };
  return { sleep: { on, onToggle: source.toggle } };
}
