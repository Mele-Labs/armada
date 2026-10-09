// Sleep mode: the title row's moon and the Morning review sheet. The state lives behind a source, the
// way Phone's does, so a Bridge with no source draws neither. The real one is `fleet-sleep.ts`, over
// Fleet's `/sleep` routes; the mock's is `mock/sleep.ts`.

import { createContext, useContext, useState, useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import { MorningReview } from "@armada/components";
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
};

const SleepContext = createContext<SleepSource | null>(null);
export const SleepSourceProvider = SleepContext.Provider;

const NONE: SleepSource = {
  get: () => ({ on: false, decided: [], blocked: [], landed: [], walks: [] }),
  subscribe: () => () => undefined,
  toggle: () => undefined,
  override: () => undefined,
};

/** Whether there is anything from the night to read. */
const left = (state: SleepState): boolean =>
  state.decided.length + state.blocked.length + state.landed.length + state.walks.length > 0;

/**
 * The title row's control and the sheet it opens. **Turning sleep off opens the review** when the
 * night left anything; the sunrise beside the moon reopens it, and is there only while there is
 * something to read.
 */
export function useSleepMode(): { sleep: TheShellProps["sleep"]; sheet: ReactNode } {
  const source = useContext(SleepContext);
  const here = source ?? NONE;
  const state = useSyncExternalStore(here.subscribe, here.get);
  const [open, setOpen] = useState(false);
  if (source === null) return { sleep: undefined, sheet: null };
  return {
    sleep: {
      on: state.on,
      onToggle: () => {
        if (state.on && left(state)) setOpen(true);
        source.toggle();
      },
      ...(left(state) ? { morning: { onOpen: () => setOpen(true) } } : {}),
    },
    sheet: (
      <MorningReview
        open={open}
        decided={state.decided}
        blocked={state.blocked}
        landed={state.landed}
        walks={state.walks.map((one) => ({ ...one, onOpen: () => setOpen(false) }))}
        onOverride={source.override}
        onClose={() => setOpen(false)}
      />
    ),
  };
}
