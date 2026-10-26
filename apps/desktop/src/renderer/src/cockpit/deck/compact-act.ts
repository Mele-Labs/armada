// Compact as the context figure in the Job's head offers it (`ContextFigure.tsx`): what the Compact tile did, through the same
// route (`offeredOf`), with the reason it is off where the agent or the terminal has not listed `compact`. The owner's ask of 25 Oct 2026.

import type { ContextCompact } from "@armada/components";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { tilesOf, useFlightFile } from "./flight-controls";
import { COMPACT, offeredOf, plain } from "./session-tiles";

/** What the context figure offers: Compact, or why it is off. */
export function useCompactAct(session: Session, draft: SessionsDraft): ContextCompact {
  const words = tilesOf("job", useFlightFile()).find((one) => one.id === "compact")?.words ?? `/${COMPACT}`;
  const { off, compact } = offeredOf(session, draft, undefined);
  return {
    ...(off.compact === undefined ? {} : { why: off.compact }),
    onPress: () => {
      if (compact?.how === "terminal") draft.command?.(session.id, words);
      else void draft.send(session.id, plain(words));
    },
  };
}
