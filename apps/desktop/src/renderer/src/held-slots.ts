// The slots a full pool is held by, as the call that offers to wait or add one carries them. Mock only: `WaitingItem` has no `holders`
// on the wire yet, so a real Fleet's call reads as it did, two choices and no rows.

import type { HeldSlotsProps } from "@armada/components";
import type { WaitingItem } from "@armada/protocol";

/** One held slot. `keeps` is why Free is off: the holder is running, or holds work no commit has. Fleet sends them longest quiet first. */
export type SlotHolder = { slot: number; job: { id: string; title: string }; quiet_since: string; keeps?: "running" | "uncommitted" };

/** A `slot_choice` item with the slots it was raised over. `creating_since` is when Fleet began the slot Create asked for, and `create_refused` why the last one failed. */
export type SlotCall = WaitingItem & { holders?: SlotHolder[]; creating_since?: string; create_refused?: string };

/** Create was pressed and Fleet is at it: nothing here needs the person now, though the call stays so its wait can be seen. */
export const creatingSlot = (item: WaitingItem): boolean => (item as SlotCall).creating_since !== undefined;

/** The rows a call draws under its choices: none for another item, a call with no holders, or where nothing serves Free. */
export function heldSlotsOf(item: WaitingItem, free: ((slot: number) => void | Promise<unknown>) | undefined, open: (jobId: string) => void, create?: () => void | Promise<unknown>): HeldSlotsProps | undefined {
  const { holders, creating_since: creatingSince, create_refused: createRefused } = item as SlotCall;
  if (item.source !== "slot_choice" || holders === undefined || holders.length === 0 || free === undefined) return undefined;
  return {
    rows: holders.map((one) => ({ slot: one.slot, job: one.job, quietSince: one.quiet_since, ...(one.keeps === undefined ? {} : { keeps: one.keeps }) })),
    onOpen: open,
    onFree: free,
    ...(create === undefined ? {} : { onCreate: create }),
    ...(creatingSince === undefined ? {} : { creatingSince }),
    ...(createRefused === undefined ? {} : { createRefused }),
  };
}
