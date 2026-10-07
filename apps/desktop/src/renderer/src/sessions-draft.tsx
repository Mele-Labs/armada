// What a moment holds of Sessions, before Fleet can serve any.
//
// **Sessions have no read behind them**, so a mock scenario carries them and
// hands them down here, as `drafted.tsx` does for the Arc milestone's boards.
// The app's own mount provides none: `useSessionsDraft` is then `undefined`,
// the rail row is left off and no chip is owned. That is the whole seam, and
// it goes the day Fleet serves a Session.

import { createContext, useContext, useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

const Held = createContext<SessionsDraft | undefined>(undefined);

const NONE: readonly Session[] = [];
const NEVER = { get: () => NONE, subscribe: () => () => undefined };

/** Hand a moment's Sessions to the window. The mock's own mount is the one caller. */
export function SessionsFrom({ held, children }: { held: SessionsDraft | undefined; children: ReactNode }) {
  return <Held.Provider value={held}>{children}</Held.Provider>;
}

/** The Sessions this window was mounted holding and the acts on them, or `undefined` where none are served. */
export function useSessionsDraft(): SessionsDraft | undefined {
  return useContext(Held);
}

/** The Sessions as they stand, live. Empty where none are served. */
export function useSessions(): readonly Session[] {
  const held = useContext(Held) ?? NEVER;
  return useSyncExternalStore(held.subscribe, held.get);
}
