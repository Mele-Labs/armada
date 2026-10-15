// What a stranded bay offers on the deck: the acts the Bays page offers for it, in the order it offers them. A Scout is started
// to read what the holder left, stopped while it reads, and once it has read, the work is picked up, stashed or scrapped. **The
// acts are Fleet's** (`rescue_slot`); where the window has no way to send one, none is drawn. #2301.

import type { Outcome, RescueAct, WorktreeSlot } from "@armada/protocol";
import { said } from "@armada/screens/src/copy";
import { rescuable } from "@armada/components/src/compositions/PoolSlots/TileActs";

export type Offered = { start: boolean; stop: boolean; pickUp: boolean; stash: boolean; scrap: boolean };

const NONE: Offered = { start: false, stop: false, pickUp: false, stash: false, scrap: false };

/** Which rescue acts this slot offers now. Nothing is offered where the window cannot send one. */
export function offeredOf(slot: WorktreeSlot, canSend: boolean): Offered {
  if (!canSend || !rescuable(slot)) return NONE;
  const found = slot.rescue;
  if (found === undefined) return { ...NONE, start: true };
  if (found.state === "reading") return { ...NONE, stop: true };
  const toContinue = found.verdict === "unfinished" || (found.verdict === undefined && (found.summary !== undefined || (found.items ?? []).length > 0));
  return { ...NONE, pickUp: toContinue, stash: true, scrap: true };
}

/** What a refused act says, led by what was not done. */
const NOT_RESCUED: Record<RescueAct, string> = {
  start: "Scout not started",
  stop: "Scout not stopped",
  scrap: "Slot not scrapped",
  stash: "Changes not stashed",
  pick_up: "Work not picked up",
};

export function refusalOf(act: RescueAct, outcome: Outcome): string {
  const why = !outcome.ok && outcome.why === "refused" ? outcome.error.message : said(outcome);
  return `${NOT_RESCUED[act]}: ${why}`;
}
