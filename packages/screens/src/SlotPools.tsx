// Cleanup's grid: each served repository's worktree pool as bays, then the
// Jobs' worktrees that stand outside it, one tile each. A press on a tile opens
// its panel, where every act is: change the pool, rescue a stranded slot, and
// give a worktree back.
//
// What Fleet answers is said in the panel of the tile it was about, and kept
// there until the next act on that tile: the pool is read again by main after
// every act, so a refusal is the one thing the read cannot carry.

import { useState } from "react";
import { PoolSlots } from "@armada/components";
import type { TileRow } from "@armada/components";
import { useAtFloor } from "@armada/shell";
import type {
  ChangeSlotPool,
  JobSummary,
  Outcome,
  RescueAct,
  RescueSlot,
  SlotAct,
  SlotRescued,
  WorktreeHeld,
  WorktreeSlot,
} from "@armada/protocol";

import { said } from "./copy";
import { branchDeletedSaid, costOf, namedByHandle, offeredActs, reclaimedSaid, sitting } from "./held";
import type { RescueOutcome } from "./slot-rescue";
import { joined } from "./tiles";

export type SlotPoolsProps = {
  slots: readonly WorktreeSlot[];
  /** Every worktree Fleet holds for a Job: a bay's own, or one outside the pool. */
  worktrees: readonly WorktreeHeld[];
  /** The board's Jobs, read for the handle that names a worktree outside the pool. */
  jobs?: readonly JobSummary[];
  now: number;
  onOpenJob: (jobId: string) => void;
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
  onCopied?: (value: string) => void;
};

/** What a refused act failed to do, leading what Fleet said. */
const NOT: Record<SlotAct, string> = {
  add: "Slot not added",
  remove: "Slot not removed",
  close: "Slot not closed",
  open: "Slot not reopened",
};

const NOT_RESCUED: Record<RescueAct, string> = {
  start: "Scout not started",
  stop: "Scout not stopped",
  scrap: "Slot not scrapped",
  stash: "Changes not stashed",
  pick_up: "Work not picked up",
};

function refusal(lead: string, outcome: Outcome): string {
  const why = !outcome.ok && outcome.why === "refused" ? outcome.error.message : said(outcome);
  return `${lead}: ${why}`;
}

/** What a Scrap, a Stash or a Pick up did, in git's words. The branch a Scrap kept; the commit a Stash or a Pick up made. */
function receipt(act: RescueAct, got: SlotRescued): string | undefined {
  if (act === "scrap") return got.branch_kept === true && got.branch !== undefined ? `Branch ${got.branch} kept` : undefined;
  if (act !== "stash" && act !== "pick_up") return undefined;
  const on = got.branch === undefined ? "" : ` on ${got.branch}`;
  return got.committed === undefined ? undefined : `Committed ${got.committed.slice(0, 7)}${on}`;
}

/** A bay's key, or a repository's add tile's where `slot` is absent. */
const keyOf = (manifestId: string, slot?: number) => `${manifestId}/${slot ?? "add"}`;

/** A worktree outside the pool is keyed by its Job. */
const outsideKey = (jobId: string) => `held/${jobId}`;

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
  onCopied,
}: SlotPoolsProps) {
  /** What Fleet refused, by tile. */
  const [refused, setRefused] = useState<Record<string, string>>({});
  /** What a Scrap or a Stash did, by tile, until the next act on it. */
  const [receipts, setReceipts] = useState<Record<string, string>>({});
  /** What a Clear or a Delete branch did, a fact to a line, by tile. */
  const [gaveBack, setGaveBack] = useState<Record<string, string[]>>({});
  /** Tiles with an act out, so a second press is not sent. */
  const [acting, setActing] = useState<ReadonlySet<string>>(new Set());
  const floor = useAtFloor();

  /** One act on one tile: refuse a second press, forget what the last one said, then run it. */
  async function run(key: string, task: () => Promise<void>): Promise<void> {
    if (acting.has(key)) return;
    setActing((was) => new Set(was).add(key));
    setRefused(({ [key]: _, ...rest }) => rest);
    setReceipts(({ [key]: _, ...rest }) => rest);
    setGaveBack(({ [key]: _, ...rest }) => rest);
    await task();
    setActing((was) => {
      const next = new Set(was);
      next.delete(key);
      return next;
    });
  }

  const refuse = (key: string, lead: string, outcome: Outcome) =>
    setRefused((was) => ({ ...was, [key]: refusal(lead, outcome) }));

  async function act(manifestId: string, act: SlotAct, slot?: number): Promise<void> {
    if (onChange === undefined) return;
    const key = keyOf(manifestId, slot);
    await run(key, async () => {
      const outcome = await onChange(manifestId, slot === undefined ? { act } : { act, slot });
      if (!outcome.ok) refuse(key, NOT[act], outcome);
    });
  }

  async function rescue(manifestId: string, act: RescueAct, slot: number): Promise<void> {
    if (onRescue === undefined) return;
    const key = keyOf(manifestId, slot);
    await run(key, async () => {
      const outcome = await onRescue(manifestId, { act, slot });
      if (!outcome.ok) return refuse(key, NOT_RESCUED[act], outcome);
      const got = receipt(act, outcome.rescued);
      if (got !== undefined) setReceipts((was) => ({ ...was, [key]: got }));
    });
  }

  /** Clear, Delete branch or Forget Job on the tile at `key`: what it did, or why not. */
  async function reclaim(
    key: string,
    jobId: string,
    what: "clear" | "branch" | "forget",
    pooled: boolean,
    tip?: string,
  ): Promise<void> {
    await run(key, async () => {
      if (what === "clear" && onReclaim !== undefined) {
        const outcome = await onReclaim(jobId);
        if (!outcome.ok) return refuse(key, pooled ? "Slot not released" : "Worktree not removed", outcome);
        if (outcome.reclaimed !== undefined) {
          setGaveBack((was) => ({ ...was, [key]: reclaimedSaid(outcome.reclaimed!, pooled) }));
        }
      } else if (what === "branch" && onDeleteBranch !== undefined && tip !== undefined) {
        const outcome = await onDeleteBranch(jobId, tip);
        if (!outcome.ok) return refuse(key, "Branch not deleted", outcome);
        if (outcome.branchDeleted !== undefined) {
          setGaveBack((was) => ({ ...was, [key]: branchDeletedSaid(outcome.branchDeleted!) }));
        }
      } else if (what === "forget" && onForget !== undefined) {
        const outcome = await onForget(jobId);
        if (!outcome.ok) refuse(key, "Record not deleted", outcome);
      }
    });
  }

  /** What a tile adds to its bay or its worktree: the figures, the acts the Job's reading offers, what happened. */
  const worked = (key: string, held: WorktreeHeld | undefined): Partial<TileRow> => {
    const job = held === undefined ? undefined : jobs.find((one) => one.id === held.job_id)?.handle;
    const named = held === undefined ? undefined : namedByHandle(held, jobs);
    const sat = held === undefined ? null : sitting(held.last_moved_at, now);
    return {
      ...(named === undefined ? {} : { held: named, offered: offeredActs(named), cost: costOf(named) }),
      ...(sat === null ? {} : { sat }),
      ...(job === undefined ? {} : { job }),
      ...(refused[key] === undefined ? {} : { refused: refused[key] }),
      ...(receipts[key] === undefined ? {} : { said: receipts[key] }),
      ...(gaveBack[key] === undefined ? {} : { receipt: gaveBack[key] }),
      ...(acting.has(key) ? { acting: true } : {}),
    };
  };

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
      ...worked(key, held),
    };
    pools.set(slot.manifest_id, [...(pools.get(slot.manifest_id) ?? []), row]);
  }
  const outsideRows = outside.map((held): TileRow => {
    const key = outsideKey(held.job_id);
    return { key, name: jobs.find((job) => job.id === held.job_id)?.handle ?? held.job_id, ...worked(key, held) };
  });

  const acts = (manifestId: string) => ({
    ...(onChange === undefined ? {} : { onAct: (one: SlotAct, slot?: number) => void act(manifestId, one, slot) }),
    ...(onRescue === undefined ? {} : { onRescue: (one: RescueAct, slot: number) => void rescue(manifestId, one, slot) }),
  });
  const reclaims = (key: (jobId: string) => string, pooled: (jobId: string) => boolean) => ({
    ...(onReclaim === undefined ? {} : { onClear: (jobId: string) => void reclaim(key(jobId), jobId, "clear", pooled(jobId)) }),
    ...(onDeleteBranch === undefined
      ? {}
      : { onDeleteBranch: (jobId: string, tip: string) => void reclaim(key(jobId), jobId, "branch", pooled(jobId), tip) }),
    ...(onForget === undefined ? {} : { onForget: (jobId: string) => void reclaim(key(jobId), jobId, "forget", pooled(jobId)) }),
    ...(onCopied === undefined ? {} : { onCopied }),
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
