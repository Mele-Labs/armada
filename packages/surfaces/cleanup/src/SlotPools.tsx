// Cleanup's grid: each served repository's worktree pool as bays, one tile each. A press on a
// tile opens its panel, where every act is: change the pool and rescue a stranded slot.
//
// What Fleet answers is said in the panel of the tile it was about, and kept
// there until the next act on that tile (`tile-acts.ts`, which a bay's pane on
// the Cockpit uses too): the pool is read again by main after every act, so a
// refusal is the one thing the read cannot carry.

import { PoolSlots } from "@armada/components";
import type { TileRow } from "@armada/components";
import { useAtFloor } from "@armada/shell";
import type { ChangeSlotPool, Outcome, RescueSlot, WorktreeSlot } from "@armada/protocol";

import { sitting } from "@armada/screens/src/held";
import type { RescueOutcome } from "./slot-rescue";
import { keyOf, useTileActs } from "./tile-acts";

export type SlotPoolsProps = {
  slots: readonly WorktreeSlot[];
  now: number;
  /** Change one repository's pool. Absent draws the pool with no acts. */
  onChange?: (manifestId: string, change: ChangeSlotPool) => Promise<Outcome>;
  /** Rescue a stranded slot of one repository's pool. Absent draws none of its acts. */
  onRescue?: (manifestId: string, rescue: RescueSlot) => Promise<RescueOutcome>;
  onCopied?: (value: string) => void;
};

export function SlotPools({ slots, now, onChange, onRescue, onCopied }: SlotPoolsProps) {
  const floor = useAtFloor();
  const { worked, acts, acting, refused } = useTileActs({ now, onChange, onRescue, onCopied });

  const pools = new Map<string, TileRow[]>();
  for (const slot of slots) {
    const key = keyOf(slot.manifest_id, slot.slot);
    const heldFor = slot.since === undefined ? null : sitting(slot.since, now);
    const row: TileRow = {
      key,
      name: `slot-${slot.slot}`,
      slot,
      ...(heldFor === null ? {} : { heldFor }),
      ...worked(key),
    };
    pools.set(slot.manifest_id, [...(pools.get(slot.manifest_id) ?? []), row]);
  }

  return (
    <>
      {[...pools].map(([manifestId, pool]) => (
        <section key={manifestId} className="armada-pool-slots__pool" aria-label={pools.size > 1 ? manifestId : undefined}>
          {/* Named for its repository only where there is more than one to tell apart. */}
          {pools.size > 1 ? <h3 className="armada-pool-slots__repo">{manifestId}</h3> : null}
          <PoolSlots
            floor={floor}
            rows={pool}
            {...acts(manifestId)}
            {...(onCopied === undefined ? {} : { onCopied })}
            {...(refused[keyOf(manifestId)] === undefined ? {} : { addRefused: refused[keyOf(manifestId)] })}
            adding={acting.has(keyOf(manifestId))}
          />
        </section>
      ))}
    </>
  );
}
