// Each served repository's worktree pool on Cleanup, as a grid of bays, and
// the acts a person takes on it: add a slot, remove one, close or reopen one,
// and rescue a stranded one.
//
// What Fleet answers is said on the bay it was about, and kept there until the
// next act on that bay: the pool itself is read again by main after every act,
// so a refusal is the one thing the read cannot carry.

import { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle, PoolSlots } from "@armada/components";
import { useAtFloor } from "@armada/shell";
import type {
  ChangeSlotPool,
  Outcome,
  RescueAct,
  RescueSlot,
  SlotAct,
  SlotRescued,
  WorktreeSlot,
} from "@armada/protocol";

import { said } from "./copy";
import { sitting } from "./held";
import type { RescueOutcome } from "./slot-rescue";

export type SlotPoolsProps = {
  slots: readonly WorktreeSlot[];
  now: number;
  onOpenJob: (jobId: string) => void;
  /** Change one repository's pool. Absent draws the pool with no acts. */
  onChange?: (manifestId: string, change: ChangeSlotPool) => Promise<Outcome>;
  /** Rescue a stranded slot of one repository's pool. Absent draws none of its acts. */
  onRescue?: (manifestId: string, rescue: RescueSlot) => Promise<RescueOutcome>;
};

/** What a refused act failed to do, leading what Fleet said. */
const NOT: Record<SlotAct, string> = {
  add: "Not added",
  remove: "Not removed",
  close: "Not closed",
  open: "Not reopened",
};

const NOT_RESCUED: Record<RescueAct, string> = {
  start: "Not started",
  stop: "Not stopped",
  scrap: "Not scrapped",
  stash: "Not stashed",
};

function refusal(lead: string, outcome: Outcome): string {
  const why = !outcome.ok && outcome.why === "refused" ? outcome.error.message : said(outcome);
  return `${lead}: ${why}`;
}

/** What a Scrap or a Stash did, as bare facts. The branch a Scrap kept; the commit a Stash made. */
function receipt(act: RescueAct, got: SlotRescued): string | undefined {
  if (act === "scrap") return got.branch_kept === true && got.branch !== undefined ? `${got.branch} kept` : undefined;
  if (act !== "stash") return undefined;
  const on = got.branch === undefined ? "" : ` on ${got.branch}`;
  return got.committed === undefined ? undefined : `${got.committed.slice(0, 7)}${on}`;
}

/** A bay's key, or a repository's add tile's where `slot` is absent. */
const keyOf = (manifestId: string, slot?: number) => `${manifestId}/${slot ?? "add"}`;

export function SlotPools({ slots, now, onOpenJob, onChange, onRescue }: SlotPoolsProps) {
  /** What Fleet refused, by bay. */
  const [refused, setRefused] = useState<Record<string, string>>({});
  /** What a Scrap or a Stash did, by bay, until the next act on it. */
  const [receipts, setReceipts] = useState<Record<string, string>>({});
  /** Bays with an act out, so a second press is not sent. */
  const [acting, setActing] = useState<ReadonlySet<string>>(new Set());
  const floor = useAtFloor();

  const pools = new Map<string, WorktreeSlot[]>();
  for (const slot of slots) pools.set(slot.manifest_id, [...(pools.get(slot.manifest_id) ?? []), slot]);

  async function act(manifestId: string, act: SlotAct, slot?: number): Promise<void> {
    if (onChange === undefined) return;
    const key = keyOf(manifestId, slot);
    if (acting.has(key)) return;
    setActing((was) => new Set(was).add(key));
    setRefused(({ [key]: _, ...rest }) => rest);
    const outcome = await onChange(manifestId, slot === undefined ? { act } : { act, slot });
    if (!outcome.ok) setRefused((was) => ({ ...was, [key]: refusal(NOT[act], outcome) }));
    setActing((was) => {
      const next = new Set(was);
      next.delete(key);
      return next;
    });
  }

  async function rescue(manifestId: string, act: RescueAct, slot: number): Promise<void> {
    if (onRescue === undefined) return;
    const key = keyOf(manifestId, slot);
    if (acting.has(key)) return;
    setActing((was) => new Set(was).add(key));
    setRefused(({ [key]: _, ...rest }) => rest);
    setReceipts(({ [key]: _, ...rest }) => rest);
    const outcome = await onRescue(manifestId, { act, slot });
    if (!outcome.ok) setRefused((was) => ({ ...was, [key]: refusal(NOT_RESCUED[act], outcome) }));
    else {
      const said = receipt(act, outcome.rescued);
      if (said !== undefined) setReceipts((was) => ({ ...was, [key]: said }));
    }
    setActing((was) => {
      const next = new Set(was);
      next.delete(key);
      return next;
    });
  }

  return (
    <>
      {[...pools].map(([manifestId, pool]) => (
        <Card key={manifestId}>
          <CardHeader>
            {/* Named for its repository only where there is more than one to tell apart. */}
            <CardTitle>{pools.size > 1 ? `Worktree slots in ${manifestId}` : "Worktree slots"}</CardTitle>
          </CardHeader>
          <CardContent>
            <PoolSlots
              floor={floor}
              rows={pool.map((slot) => {
                const key = keyOf(manifestId, slot.slot);
                const heldFor = slot.since === undefined ? null : sitting(slot.since, now);
                return {
                  slot,
                  ...(heldFor === null ? {} : { heldFor }),
                  ...(refused[key] === undefined ? {} : { refused: refused[key] }),
                  ...(receipts[key] === undefined ? {} : { said: receipts[key] }),
                  ...(acting.has(key) ? { acting: true } : {}),
                };
              })}
              onOpenJob={onOpenJob}
              {...(onRescue === undefined
                ? {}
                : { onRescue: (one: RescueAct, slot: number) => void rescue(manifestId, one, slot) })}
              {...(onChange === undefined
                ? {}
                : { onAct: (one: SlotAct, slot?: number) => void act(manifestId, one, slot) })}
              {...(refused[keyOf(manifestId)] === undefined ? {} : { addRefused: refused[keyOf(manifestId)] })}
              adding={acting.has(keyOf(manifestId))}
            />
          </CardContent>
        </Card>
      ))}
    </>
  );
}
