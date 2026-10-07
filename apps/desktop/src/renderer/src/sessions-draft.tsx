// What the window holds of Sessions, and the acts on them.
//
// **The seam between the screens and whatever serves Sessions.** A mock scenario carries them as
// fixtures and hands them down here, as `drafted.tsx` does for the Arc milestone's boards; a real
// Fleet's are `sessions-wired.tsx`, built over what main publishes. Where nothing serves them,
// `useSessionsDraft` is `undefined`, the rail row is left off and no chip is owned.

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

/** What Fleet last refused a Session act, in words. Absent where nothing was refused, and in the mock. */
export function useSessionsSaid(): string | undefined {
  const held = useContext(Held);
  return useSyncExternalStore(held?.subscribe ?? NEVER.subscribe, () => held?.said?.());
}

/** The Sessions as they stand, live. Empty where none are served. */
export function useSessions(): readonly Session[] {
  const held = useContext(Held) ?? NEVER;
  return useSyncExternalStore(held.subscribe, held.get);
}
