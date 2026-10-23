// What the window holds of Sessions, and the acts on them.
//
// **The seam between the screens and whatever serves Sessions.** A mock scenario carries them as
// fixtures and hands them down here, as `drafted.tsx` does for the Arc milestone's boards; a real
// Fleet's are `sessions-wired.tsx`, built over what main publishes. Where nothing serves them,
// `useSessionsDraft` is `undefined`, the rail row is left off and no chip is owned.

import { createContext, useContext, useMemo, useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import type { QueuedIdea, Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { announceEnd } from "./cockpit/deck/exit";
import { withOutgoing } from "./outgoing";
import { withPressed } from "./pressed";
import { ViewerNotesOver } from "./viewer-notes";

const Held = createContext<SessionsDraft | undefined>(undefined);

/** The acts on a repository's own card, which a real Fleet serves whether or not the Sessions list has been read. */
export type RepositoryActs = Pick<SessionsDraft, "runInRepository" | "diffInRepository" | "actInRepository" | "refreshPullRequest" | "sendCheckoutAsPullRequest" | "checkoutSendFailed">;
const Acts = createContext<RepositoryActs | undefined>(undefined);

const NONE: readonly Session[] = [];
const NEVER = { get: () => NONE, subscribe: () => () => undefined };
const NO_IDEAS: readonly QueuedIdea[] = [];

/** End answers at once, over whichever serves it: the stage plays the Job out and shows the timeline, and Fleet parks the Job in the background (`cockpit/deck/exit.ts`). */
const withLeaving = (draft: SessionsDraft): SessionsDraft => {
  const { close } = draft;
  // Over the draft as its prototype, as `withPressed` is: a spread loses what it reads by getter, `commands` among them.
  return close === undefined ? draft : Object.assign(Object.create(draft) as SessionsDraft, { close: (id: string) => (announceEnd(id), close.call(draft, id)) });
};

/** Hand a moment's Sessions to the window. The mock's own mount is the one caller. */
export function SessionsFrom({ held, acts, children }: { held: SessionsDraft | undefined; acts?: RepositoryActs; children: ReactNode }) {
  // What the person sent is in the thread at once, and Pause, Auto land and Approve read as pressed at the press, over whichever serves it (`outgoing.ts`, `pressed.ts`).
  const sending = useMemo(() => (held === undefined ? undefined : withLeaving(withPressed(withOutgoing(held)))), [held]);
  return (
    <Acts.Provider value={acts}>
      <Held.Provider value={sending}>
        <ViewerNotesOver draft={sending}>{children}</ViewerNotesOver>
      </Held.Provider>
    </Acts.Provider>
  );
}

/**
 * What a repository's card may run and read. **Held apart from the Sessions**: a real Fleet answers these before the list of
 * Sessions has loaded, and a card that waited on the list showed no Run. The mock's draft carries them too.
 */
export function useRepositoryActs(): RepositoryActs {
  const acts = useContext(Acts);
  const held = useContext(Held);
  return {
    runInRepository: acts?.runInRepository ?? held?.runInRepository,
    diffInRepository: acts?.diffInRepository ?? held?.diffInRepository,
    actInRepository: acts?.actInRepository ?? held?.actInRepository,
    refreshPullRequest: acts?.refreshPullRequest ?? held?.refreshPullRequest,
    sendCheckoutAsPullRequest: acts?.sendCheckoutAsPullRequest ?? held?.sendCheckoutAsPullRequest,
    checkoutSendFailed: acts?.checkoutSendFailed ?? held?.checkoutSendFailed,
  };
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

/** The ideas saved to start later, live. Empty where none are served. */
export function useQueued(): readonly QueuedIdea[] {
  const held = useContext(Held);
  return useSyncExternalStore(held?.subscribe ?? NEVER.subscribe, held?.queued ?? (() => NO_IDEAS));
}
