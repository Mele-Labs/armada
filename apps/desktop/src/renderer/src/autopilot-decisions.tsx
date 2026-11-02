// What Autopilot answered while the owner was away, which the Cockpit lists in a HUD opened from the Autopilot switch
// (`cockpit/deck/AutopilotAnswers.tsx`). The list lives behind a source, the way Phone's does, so a Bridge with no
// source draws none. The real one is `fleet-autopilot-decisions.ts`, over Fleet's `/autopilot/decisions`
// routes; the mock's Fleet answers the same routes.

import { createContext, useContext, useSyncExternalStore } from "react";
import type { AutopilotDecided } from "@armada/protocol";

export type AutopilotDecisionsSource = {
  get(): readonly AutopilotDecided[];
  subscribe(on: () => void): () => void;
  /** Sends the Job the owner's words, and Fleet marks the answer corrected and reviewed. */
  override(id: string, text: string): void;
  /** The owner kept the answer: its call leaves the Cockpit and Fleet does not bring it back. */
  review(id: string): void;
};

const DecisionsContext = createContext<AutopilotDecisionsSource | null>(null);
export const AutopilotDecisionsSourceProvider = DecisionsContext.Provider;

const NO_DECISIONS: readonly AutopilotDecided[] = [];

const NONE: AutopilotDecisionsSource = {
  get: () => NO_DECISIONS,
  subscribe: () => () => undefined,
  override: () => undefined,
  review: () => undefined,
};

/** The source, where there is one. */
export const useAutopilotDecisionsSource = (): AutopilotDecisionsSource | null => useContext(DecisionsContext);

/** What Autopilot answered for the owner and he has not yet kept or changed. */
export function useAutopilotDecided(): readonly AutopilotDecided[] {
  const here = useAutopilotDecisionsSource() ?? NONE;
  return useSyncExternalStore(here.subscribe, here.get);
}

/**
 * Whether Fleet takes a correction of this answer. A row with no Job has nobody to tell, and a pull
 * request landed or a red main taken has no Job to ask again: only Agree is offered there.
 */
export const correctable = (one: AutopilotDecided): boolean => one.session_id !== undefined && one.kind !== "land" && one.kind !== "red";
