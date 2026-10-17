// What a tile of the worktree pool is acted on with, and what Fleet answered: the state and the handlers behind Cleanup's grid
// and behind a bay's pane on the Cockpit, so both send the same acts and say the same refusals and receipts.
//
// What Fleet answers is said on the tile it was about, and kept there until the next act on that tile: the pool is read again
// by main after every act, so a refusal is the one thing the read cannot carry.

import { useState } from "react";
import type { TileRow } from "@armada/components";
import type { ChangeSlotPool, Outcome, RescueAct, RescueSlot, SlotAct, SlotRescued } from "@armada/protocol";

import { said } from "@armada/screens/src/copy";
import { releaseRefusal } from "@armada/screens/src/releasing";
import type { RescueOutcome } from "./slot-rescue";

export type TileActsIn = {
  now: number;
  /** Change one repository's pool. Absent draws the pool with no acts. */
  onChange?: ((manifestId: string, change: ChangeSlotPool) => Promise<Outcome>) | undefined;
  /** Rescue a stranded slot of one repository's pool. Absent draws none of its acts. */
  onRescue?: ((manifestId: string, rescue: RescueSlot) => Promise<RescueOutcome>) | undefined;
  onCopied?: ((value: string) => void) | undefined;
};

/** What a refused act failed to do, leading what Fleet said. */
const NOT: Record<SlotAct, string> = {
  add: "Slot not added",
  remove: "Slot not removed",
  close: "Slot not closed",
  open: "Slot not reopened",
  release: "Slot not released",
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
export const keyOf = (manifestId: string, slot?: number) => `${manifestId}/${slot ?? "add"}`;

export function useTileActs({ onChange, onRescue }: TileActsIn) {
  /** What Fleet refused, by tile. */
  const [refused, setRefused] = useState<Record<string, string>>({});
  /** The Session whose work a release was refused for, by tile: its panel offers to open it. */
  const [workingSession, setWorkingSession] = useState<Record<string, string>>({});
  /** What a Scrap or a Stash did, by tile, until the next act on it. */
  const [receipts, setReceipts] = useState<Record<string, string>>({});
  /** Tiles with an act out, so a second press is not sent. */
  const [acting, setActing] = useState<ReadonlySet<string>>(new Set());

  /** One act on one tile: refuse a second press, forget what the last one said, then run it. */
  async function run(key: string, task: () => Promise<void>): Promise<void> {
    if (acting.has(key)) return;
    setActing((was) => new Set(was).add(key));
    setRefused(({ [key]: _, ...rest }) => rest);
    setReceipts(({ [key]: _, ...rest }) => rest);
    setWorkingSession(({ [key]: _, ...rest }) => rest);
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

  /** Release a session's slot for the holder the confirm showed, and say where its files went. A hosted Session's holder is its id. */
  async function release(manifestId: string, slot: number, holder: string): Promise<void> {
    if (onChange === undefined) return;
    const key = keyOf(manifestId, slot);
    await run(key, async () => {
      const outcome = await onChange(manifestId, { act: "release", slot, holder });
      if (!outcome.ok) {
        const refusal = releaseRefusal(outcome);
        if (refusal.working) setWorkingSession((was) => ({ ...was, [key]: holder }));
        return setRefused((was) => ({ ...was, [key]: refusal.said }));
      }
      const released = outcome.slotChanged?.released;
      const said = released?.saved == null ? "Slot released" : `Committed to ${released.branch}, slot released`;
      setReceipts((was) => ({ ...was, [key]: said }));
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

  /** What a tile adds to its bay: what happened. */
  const worked = (key: string): Partial<TileRow> => ({
    ...(refused[key] === undefined ? {} : { refused: refused[key] }),
    ...(workingSession[key] === undefined ? {} : { openSession: workingSession[key] }),
    ...(receipts[key] === undefined ? {} : { said: receipts[key] }),
    ...(acting.has(key) ? { acting: true } : {}),
  });

  const acts = (manifestId: string) => ({
    ...(onChange === undefined ? {} : { onAct: (one: SlotAct, slot?: number) => void act(manifestId, one, slot) }),
    ...(onChange === undefined ? {} : { onRelease: (slot: number, holder: string) => void release(manifestId, slot, holder) }),
    ...(onRescue === undefined ? {} : { onRescue: (one: RescueAct, slot: number) => void rescue(manifestId, one, slot) }),
  });
  return { worked, acts, acting, refused };
}
