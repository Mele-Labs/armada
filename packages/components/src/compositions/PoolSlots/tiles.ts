// What Cleanup's grid draws a tile from. A tile is a bay of the pool; the screen decides which
// acts apply, because those rules are its own and are unit-tested there.

import type { WorktreeSlot } from "@armada/protocol";

export type TileRow = {
  /** Unique across the grid. Absent is the bay's `manifest/slot`. */
  key?: string;
  /** Absent is `slot-4`. */
  name?: string;
  slot: WorktreeSlot;
  /** How long its holder has had the slot, formatted by the caller's clock. */
  heldFor?: string;
  /** Why the last act on this tile was refused, said in its panel. */
  refused?: string;
  /** The Session whose work made the last act refuse: Open Session sits beside the refusal. */
  openSession?: string;
  /** What the last rescue act did, as a bare fact: the branch a Scrap kept, the commit a Stash made. */
  said?: string;
  /** An act on this tile is out, so its acts wait. */
  acting?: boolean;
};

export const keyOf = (row: TileRow): string => row.key ?? `${row.slot.manifest_id}/${row.slot.slot}`;

export const nameOf = (row: TileRow): string => row.name ?? `slot-${row.slot.slot}`;
