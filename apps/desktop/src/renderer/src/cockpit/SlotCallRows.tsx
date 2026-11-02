// The slots a full pool is held by, under a `slot_choice` call's choices on the Cockpit's card.

import { HeldSlots } from "@armada/components";

import type { Hosts, Item } from "../Dashboard";
import { ADD_CHOICE, heldSlotsOf } from "../held-slots";
import { useSessionsDraft } from "../sessions-draft";

/** Nothing for any other call. Each Free serves the Job the call is on. */
export function SlotCallRows({ item, hosts }: { item: Item; hosts: Hosts }) {
  const draft = useSessionsDraft();
  const waiting = item.waiting;
  const rows =
    waiting === undefined
      ? undefined
      : heldSlotsOf(
          waiting.item,
          draft?.freeSlot === undefined
            ? undefined
            : (slot) => draft.freeSlot?.(waiting.sessionId, waiting.item.id, slot),
          hosts.onOpenSession,
          draft?.answerWaiting === undefined
            ? undefined
            : () =>
                draft.answerWaiting?.(waiting.sessionId, waiting.item.id, {
                  choice: ADD_CHOICE,
                }),
        );
  return rows === undefined ? null : <HeldSlots {...rows} />;
}
