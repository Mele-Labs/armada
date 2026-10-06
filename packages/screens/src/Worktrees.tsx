// What Fleet is holding disk for, as one grid of tiles.
//
// # One grid, because the pool and the Jobs are the same worktrees
//
// Each Job's worktree is a slot of the pool, so a list of held worktrees below
// the bays drew the same checkout twice with different acts. A tile is a bay,
// or a Job's worktree outside the pool, and a press on it opens the panel that
// manages it (`SlotPools.tsx`). The rule that Fleet reclaims what passes all
// five safety tests on its own stands, and its worktrees are tiles like the
// rest, so a worktree missing from the grid is one already given back.
//
// # Per tile, never all-or-nothing
//
// There is one bulk act in armada, `armada clean --everything`, and it is the
// one nobody should reach for from a screen, so Clear is per tile and its
// confirm names what that one checkout destroys. **The head's two sweeps are not
// that act**: Clear and Delete records reach every finished Job and confirm on
// their own, and sit here because the owner moved them off Overview's menu on
// 1 Oct 2026.
//
// What a tile offers and what its confirm says are in `held.ts`, which is
// unit-tested, because every line of it is read immediately before something
// is destroyed.

import { useEffect, type ReactNode } from "react";
import { Alert, Button } from "@armada/components";

import type { ChangeSlotPool, HeldWorktrees, JobSummary, Outcome, RescueSlot } from "@armada/protocol";
import { said } from "./copy";
import { SlotPools } from "./SlotPools";
import type { RescueOutcome } from "./slot-rescue";

export type WorktreesProps = {
  /**
   * Ask the host to open or close the read.
   *
   * **It has to be stable**, for `Reports`'s reason: this is depended on by an
   * effect, and a lambda rebuilt every render would open and close the read on
   * a loop — the read publishes state, so the loop would feed itself.
   */
  onWant: (want: boolean) => void;
  /** `GET /worktrees`, as main published it. */
  held: HeldWorktrees;
  /**
   * The board's own Jobs, read for their handles.
   *
   * **Only for naming**: a worktree outside the pool by its Job's handle, and a
   * `depended_on` reason's blocker. Defaults to none, which falls back to the
   * id — a screen with no board read yet still has something to show.
   */
  jobs?: readonly JobSummary[];
  /**
   * Give one worktree back, and answer with what the two halves did.
   *
   * **One id at a time, and a promise rather than a callback.** There is no
   * bulk route on the wire and there should not be: each is independent and the
   * receipt belongs to the press that asked for it — publishing it as app state
   * would make one person's gesture part of what every surface re-renders on.
   */
  onReclaim: (jobId: string) => Promise<Outcome>;
  /**
   * Delete one tile's branch, sending the tip the person confirmed.
   *
   * **The tip is what was confirmed, not what fleet answers with next.** Fleet
   * refuses with 409 where the tip has moved since — the safety net a stale
   * confirmation needs, not a fact this screen has to keep current itself.
   */
  onDeleteBranch: (jobId: string, tip: string) => Promise<Outcome>;
  /**
   * Delete one Job's whole record. **There is no undo.** Only ever offered once
   * the checkout and the branch are gone — `held.ts`'s `offeredActs` keeps that
   * true before this is called.
   */
  onForget: (jobId: string) => Promise<Outcome>;
  /**
   * Back to the Board. **At the top of every state**, not only the read
   * one — the page head this used to live in carried it whether or not the
   * read had come back yet.
   */
  onClose: () => void;
  /**
   * The clock every elapsed figure on this surface is drawn from.
   *
   * **The app's one `now`, not a `Date.now()` per row.** Two clocks on one
   * screen drift, and a test that could not fix the instant would be asserting
   * against the wall.
   */
  now: number;
  /** A clipboard write is silent, so the surface confirms it. */
  onCopied: (value: string) => void;
  /** Drawn in the head, across from the way out: the caller's bulk sweeps. */
  actions?: ReactNode;
  /** Open the Job holding a worktree. */
  onOpenJob: (jobId: string) => void;
  /** Add, remove, close or reopen one slot of a repository's pool. Absent draws no acts. */
  onChangeSlotPool?: (manifestId: string, change: ChangeSlotPool) => Promise<Outcome>;
  /** Start or stop a rescue of a stranded slot, or Scrap or Stash it. Absent draws none of its acts. */
  onRescueSlot?: (manifestId: string, rescue: RescueSlot) => Promise<RescueOutcome>;
};

/** How often the pool is read again while a Scout reads a slot, so its files arrive as it goes. */
const RESCUE_READ_MS = 1_000;

export function Worktrees({
  onWant,
  held,
  jobs = [],
  onReclaim,
  onDeleteBranch,
  onForget,
  now,
  onClose,
  onCopied,
  actions,
  onOpenJob,
  onChangeSlotPool,
  onRescueSlot,
}: WorktreesProps) {
  useEffect(() => {
    onWant(true);
    return () => onWant(false);
  }, []);

  const scouting =
    held.state === "read" && (held.held.slots ?? []).some((one) => one.rescue?.state === "reading");
  // Fleet sends nothing when a Scout reads another file, so the surface asks.
  useEffect(() => {
    if (!scouting) return;
    const timer = setInterval(() => onWant(true), RESCUE_READ_MS);
    return () => clearInterval(timer);
  }, [scouting]);

  /** The way out, at the top of every state — #1090 moved it here from the
   *  page head that used to carry it. */
  const back = (
    <div className="armada-screen__head-row">
      <Button variant="ghost" size="sm" onClick={onClose}>
        Back to the list
      </Button>
      {actions === undefined ? null : <div className="armada-screen__actions">{actions}</div>}
    </div>
  );

  if (held.state === "failed") {
    return (
      <div className="armada-screen__pane">
        {back}
        <Alert tone="escalated" title="What fleet is holding could not be read">
          {said(held.outcome)}
        </Alert>
      </div>
    );
  }
  // Before the read answers there is nothing yet to say, so only the way back is drawn.
  if (held.state !== "read") {
    return <div className="armada-screen__pane">{back}</div>;
  }

  return (
    <div className="armada-screen__pane">
      {back}
      <SlotPools
        slots={held.held.slots ?? []}
        worktrees={held.held.worktrees}
        jobs={jobs}
        now={now}
        onOpenJob={onOpenJob}
        onReclaim={onReclaim}
        onDeleteBranch={onDeleteBranch}
        onForget={onForget}
        onCopied={onCopied}
        {...(onChangeSlotPool === undefined ? {} : { onChange: onChangeSlotPool })}
        {...(onRescueSlot === undefined ? {} : { onRescue: onRescueSlot })}
      />
    </div>
  );
}
