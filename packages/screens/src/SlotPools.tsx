// Each served repository's worktree pool on Cleanup, as a grid of bays, and
// the acts a person takes on it: add a slot, remove one, close or reopen one.
//
// What Fleet answers is said on the bay it was about, and kept there until the
// next act on that bay: the pool itself is read again by main after every act,
// so a refusal is the one thing the read cannot carry.

import { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle, PoolSlots } from "@armada/components";
import type { ChangeSlotPool, Outcome, SlotAct, WorktreeSlot } from "@armada/protocol";

import { said } from "./copy";
import { sitting } from "./held";

export type SlotPoolsProps = {
  slots: readonly WorktreeSlot[];
  now: number;
  onOpenJob: (jobId: string) => void;
  /** Change one repository's pool. Absent draws the pool with no acts. */
  onChange?: (manifestId: string, change: ChangeSlotPool) => Promise<Outcome>;
};

/** What a refused act failed to do, leading what Fleet said. */
const NOT: Record<SlotAct, string> = {
  add: "Not added",
  remove: "Not removed",
  close: "Not closed",
  open: "Not reopened",
};

function refusal(act: SlotAct, outcome: Outcome): string {
  const why = !outcome.ok && outcome.why === "refused" ? outcome.error.message : said(outcome);
  return `${NOT[act]}: ${why}`;
}

/** A bay's key, or a repository's add tile's where `slot` is absent. */
const keyOf = (manifestId: string, slot?: number) => `${manifestId}/${slot ?? "add"}`;

export function SlotPools({ slots, now, onOpenJob, onChange }: SlotPoolsProps) {
  /** What Fleet refused, by bay. */
  const [refused, setRefused] = useState<Record<string, string>>({});
  /** Bays with an act out, so a second press is not sent. */
  const [acting, setActing] = useState<ReadonlySet<string>>(new Set());

  const pools = new Map<string, WorktreeSlot[]>();
  for (const slot of slots) pools.set(slot.manifest_id, [...(pools.get(slot.manifest_id) ?? []), slot]);

  async function act(manifestId: string, act: SlotAct, slot?: number): Promise<void> {
    if (onChange === undefined) return;
    const key = keyOf(manifestId, slot);
    if (acting.has(key)) return;
    setActing((was) => new Set(was).add(key));
    setRefused(({ [key]: _, ...rest }) => rest);
    const outcome = await onChange(manifestId, slot === undefined ? { act } : { act, slot });
    if (!outcome.ok) setRefused((was) => ({ ...was, [key]: refusal(act, outcome) }));
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
              rows={pool.map((slot) => {
                const key = keyOf(manifestId, slot.slot);
                const heldFor = slot.since === undefined ? null : sitting(slot.since, now);
                return {
                  slot,
                  ...(heldFor === null ? {} : { heldFor }),
                  ...(refused[key] === undefined ? {} : { refused: refused[key] }),
                  ...(acting.has(key) ? { acting: true } : {}),
                };
              })}
              onOpenJob={onOpenJob}
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
