// Compact as the context figure in the Job's head offers it while a take is on (`flight-take.ts`, `ContextFigure.tsx`): what the Compact tile did, through the same
// route (`offeredOf`), with the reason it is off where the agent or the terminal has not listed `compact`. The owner's ask of 25 Oct 2026.

import type { ContextCompact } from "@armada/components";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { flightTake } from "./flight-take";
import { tilesOf, useFlightFile } from "./flight-controls";
import { COMPACT, offeredOf, plain } from "./session-tiles";

/** Nothing outside a take, where Compact is a tile. */
export function useCompactAct(session: Session, draft: SessionsDraft): ContextCompact | undefined {
  const words = tilesOf("job", useFlightFile()).find((one) => one.id === "compact")?.words ?? `/${COMPACT}`;
  if (flightTake() === undefined) return undefined;
  const { off, compact } = offeredOf(session, draft, undefined);
  return {
    ...(off.compact === undefined ? {} : { why: off.compact }),
    onPress: () => {
      if (compact?.how === "terminal") draft.command?.(session.id, words);
      else void draft.send(session.id, plain(words));
    },
  };
}
