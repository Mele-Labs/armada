// Cleanup's grid: each served repository's worktree pool as bays, then the
// Jobs' worktrees that stand outside it, one tile each. A press on a tile opens
// its panel, where every act is: change the pool, rescue a stranded slot, and
// give a worktree back.
//
// What Fleet answers is said in the panel of the tile it was about, and kept
// there until the next act on that tile (`tile-acts.ts`, which a bay's pane on
// the Cockpit uses too): the pool is read again by main after every act, so a
// refusal is the one thing the read cannot carry.

import { PoolSlots } from "@armada/components";
import type { TileRow } from "@armada/components";
import { useAtFloor } from "@armada/shell";
import type { ChangeSlotPool, JobSummary, Outcome, RescueSlot, WorktreeHeld, WorktreeSlot } from "@armada/protocol";

import { sitting } from "@armada/screens/src/held";
import type { RescueOutcome } from "./slot-rescue";
import { keyOf, outsideKey, useTileActs } from "./tile-acts";
import { joined } from "./tiles";

export type SlotPoolsProps = {
  slots: readonly WorktreeSlot[];
  /** Every worktree Fleet holds for a Job: a bay's own, or one outside the pool. */
  worktrees: readonly WorktreeHeld[];
  /** The board's Jobs, read for the handle that names a worktree outside the pool. */
  jobs?: readonly JobSummary[];
  now: number;
  onOpenJob?: ((jobId: string) => void) | undefined;
  /** Change one repository's pool. Absent draws the pool with no acts. */
  onChange?: (manifestId: string, change: ChangeSlotPool) => Promise<Outcome>;
  /** Rescue a stranded slot of one repository's pool. Absent draws none of its acts. */
  onRescue?: (manifestId: string, rescue: RescueSlot) => Promise<RescueOutcome>;
  /** Give one worktree back. One id at a time: each is independent, and its receipt is its press's. */
  onReclaim?: (jobId: string) => Promise<Outcome>;
  /** Delete one worktree's branch at the tip the person confirmed; Fleet refuses where it has moved. */
  onDeleteBranch?: (jobId: string, tip: string) => Promise<Outcome>;
  /** Delete one Job's record. There is no undo, so it is sent only from its confirm. */
  onForget?: (jobId: string) => Promise<Outcome>;
  /** Pause one Job that holds a bay. Sent only from its confirm. */
  onPause?: (jobId: string) => Promise<Outcome>;
  /** Resume one paused Job. */
  onResume?: (jobId: string) => Promise<Outcome>;
  onCopied?: (value: string) => void;
};

export function SlotPools({
  slots,
  worktrees,
  jobs = [],
  now,
  onOpenJob,
  onChange,
  onRescue,
  onReclaim,
  onDeleteBranch,
  onForget,
  onPause,
  onResume,
  onCopied,
}: SlotPoolsProps) {
  const floor = useAtFloor();
  const { worked, acts, reclaims, acting, refused } = useTileActs({ jobs, now, onChange, onRescue, onReclaim, onDeleteBranch, onForget, onPause, onResume, onCopied });

  const { bays, outside } = joined(slots, worktrees);
  const pools = new Map<string, TileRow[]>();
  for (const { slot, held } of bays) {
    const key = keyOf(slot.manifest_id, slot.slot);
    const heldFor = slot.since === undefined ? null : sitting(slot.since, now);
    const row: TileRow = {
      key,
      name: `slot-${slot.slot}`,
      slot,
      ...(heldFor === null ? {} : { heldFor }),
      ...worked(key, held, slot),
    };
    pools.set(slot.manifest_id, [...(pools.get(slot.manifest_id) ?? []), row]);
  }
  const outsideRows = outside.map((held): TileRow => {
    const key = outsideKey(held.job_id);
    return { key, name: jobs.find((job) => job.id === held.job_id)?.handle ?? held.job_id, ...worked(key, held) };
  });

  // Alone, a pool takes the worktrees outside it into its own grid; beside others, they have a grid of their own.
  const lone = pools.size === 1;
  return (
    <>
      {[...pools].map(([manifestId, pool]) => (
        <section key={manifestId} className="armada-pool-slots__pool" aria-label={pools.size > 1 ? manifestId : undefined}>
          {/* Named for its repository only where there is more than one to tell apart. */}
          {pools.size > 1 ? <h3 className="armada-pool-slots__repo">{manifestId}</h3> : null}
          <PoolSlots
            floor={floor}
            rows={lone ? [...pool, ...outsideRows] : pool}
            onOpenJob={onOpenJob}
            {...acts(manifestId)}
            {...reclaims(
              (jobId) => pool.find((row) => row.held?.job_id === jobId)?.key ?? outsideKey(jobId),
              (jobId) => pool.some((row) => row.held?.job_id === jobId),
            )}
            {...(refused[keyOf(manifestId)] === undefined ? {} : { addRefused: refused[keyOf(manifestId)] })}
            adding={acting.has(keyOf(manifestId))}
          />
        </section>
      ))}
      {lone || outsideRows.length === 0 ? null : (
        <PoolSlots floor={floor} rows={outsideRows} onOpenJob={onOpenJob} {...reclaims(outsideKey, () => false)} />
      )}
    </>
  );
}
