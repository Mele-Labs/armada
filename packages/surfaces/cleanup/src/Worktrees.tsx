// What Fleet is holding disk for, as one grid of tiles: the bays of each repository's worktree
// pool. A press on a tile opens the panel that manages it (`SlotPools.tsx`). The rule that Fleet
// reclaims what passes all five safety tests on its own stands, so a worktree missing from the
// grid is one already given back.

import { useEffect, type ReactNode } from "react";
import { ArrowLeft } from "lucide-react";
import { Alert, Button, Card, CardContent, CardHeader, CardTitle, Skeleton, Tooltip } from "@armada/components";

import type { ChangeSlotPool, HeldWorktrees, Outcome, RescueSlot } from "@armada/protocol";
import { said } from "@armada/screens/src/copy";
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
  /** Add, remove, close or reopen one slot of a repository's pool. Absent draws no acts. */
  onChangeSlotPool?: (manifestId: string, change: ChangeSlotPool) => Promise<Outcome>;
  /** Start or stop a rescue of a stranded slot, or Scrap or Stash it. Absent draws none of its acts. */
  onRescueSlot?: (manifestId: string, rescue: RescueSlot) => Promise<RescueOutcome>;
};

const PENDING_BAYS = [0, 1, 2, 3, 4, 5];

/** How often the pool is read again while a Scout reads a slot, so its files arrive as it goes. */
const RESCUE_READ_MS = 1_000;

export function Worktrees({
  onWant,
  held,
  now,
  onClose,
  onCopied,
  actions,
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

  /** The way out, in the strip of the first panel in every state — #1090 moved it here from the
   *  page head that used to carry it. The caller's bulk sweeps sit beside it. */
  const back = (
    <>
      {actions === undefined ? null : <div className="armada-screen__actions">{actions}</div>}
      <Tooltip label="Back to the list">
        <Button variant="ghost" iconOnly aria-label="Back to the list" onClick={onClose}>
          <ArrowLeft size={16} aria-hidden="true" />
        </Button>
      </Tooltip>
    </>
  );
  /** Before the read answers, or when it failed, there is no pool to carry the strip, so the strip is a panel of its own. */
  const panel = (body?: ReactNode) => (
    <Card>
      <CardHeader>
        <CardTitle>Worktree slots</CardTitle>
        {back}
      </CardHeader>
      {body === undefined ? null : <CardContent>{body}</CardContent>}
    </Card>
  );

  if (held.state === "failed") {
    return (
      <div className="armada-screen__pane">
        {panel(
          <Alert tone="escalated" title="What fleet is holding could not be read">
            {said(held.outcome)}
          </Alert>,
        )}
      </div>
    );
  }
  // Before the read answers, bays stand where the pool's will: no text, so the slots land on them.
  if (held.state !== "read") {
    return (
      <div className="armada-screen__pane">
        <div className="armada-pool-slots__views" aria-busy>
          <div className="armada-pool-slots__view">
            <div className="armada-pool-slots__band">
              <span className="armada-pool-slots__label" aria-hidden>
                Worktree slots
              </span>
              {back}
            </div>
            <ul className="armada-pool-slots armada-pool-slots--pending" aria-hidden>
              {PENDING_BAYS.map((at) => (
                <li key={at} className="armada-bay" data-bay="pending">
                  <div className="armada-bay__band">
                    <Skeleton width="40%" />
                  </div>
                  <div className="armada-bay__body">
                    <Skeleton width="70%" />
                    <Skeleton width="85%" />
                  </div>
                </li>
              ))}
            </ul>
          </div>
        </div>
      </div>
    );
  }
  // Read, and no slot in it: an empty slot stays empty, so only the way back is drawn.
  if ((held.held.slots ?? []).length === 0) {
    return <div className="armada-screen__pane">{panel()}</div>;
  }

  return (
    <div className="armada-screen__pane">
      <SlotPools
        bandActs={back}
        slots={held.held.slots ?? []}
        now={now}
        onCopied={onCopied}
        {...(onChangeSlotPool === undefined ? {} : { onChange: onChangeSlotPool })}
        {...(onRescueSlot === undefined ? {} : { onRescue: onRescueSlot })}
      />
    </div>
  );
}
