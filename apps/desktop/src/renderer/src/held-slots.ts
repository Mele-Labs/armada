// The slots a full pool is held by, as the call that offers to wait or add one carries them. Mock only: `WaitingItem` has no `holders`
// on the wire yet, so a real Fleet's call reads as it did, two choices and no rows.

import type { HeldSlotsProps } from "@armada/components";
import type { WaitingItem } from "@armada/protocol";

/** One held slot. `keeps` is why Free is off: the holder is running, or holds work no commit has. Fleet sends them longest quiet first. */
export type SlotHolder = { slot: number; job: { id: string; title: string }; quiet_since: string; keeps?: "running" | "uncommitted" };

/** A `slot_choice` item with the slots it was raised over. */
export type SlotCall = WaitingItem & { holders?: SlotHolder[] };

/** The rows a call draws under its choices: none for another item, a call with no holders, or where nothing serves Free. */
export function heldSlotsOf(item: WaitingItem, free: ((slot: number) => void | Promise<unknown>) | undefined, open: (jobId: string) => void, create?: () => void | Promise<unknown>): HeldSlotsProps | undefined {
  const holders = (item as SlotCall).holders;
  if (item.source !== "slot_choice" || holders === undefined || holders.length === 0 || free === undefined) return undefined;
  return {
    rows: holders.map((one) => ({ slot: one.slot, job: one.job, quietSince: one.quiet_since, ...(one.keeps === undefined ? {} : { keeps: one.keeps }) })),
    onOpen: open,
    onFree: free,
    ...(create === undefined ? {} : { onCreate: create }),
  };
}
