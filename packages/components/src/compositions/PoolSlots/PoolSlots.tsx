import { FolderPlus } from "lucide-react";
import { useState } from "react";
import type { RescueAct, SlotAct } from "@armada/protocol";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { PoolTile } from "./PoolTile";
import { TileSheet } from "./TileSheet";
import { keyOf } from "./tiles";
import type { TileRow } from "./tiles";

export type { ClearCost, Offered, TileRow } from "./tiles";
export { ClearSaves } from "./ClearSaves";
export { TileSheet } from "./TileSheet";

/**
 * Cleanup's grid, one tile per worktree and each styled by what holds it. A
 * bay of the pool: held is a filled card under a band in the leased hue, free
 * an open dashed outline, stranded hatched in the warning hue, and a slot not
 * made a faint ghost. A Job's worktree outside the pool is drawn in the same
 * grid after the bays. A slot a person closed is shuttered in the closed hue;
 * a held one closed keeps its holder and takes the closed mark.
 * `armada worktree --status` is the same reading. Marks are group `Worktree
 * slot` in `packages/icons/icons/`.
 *
 * **A tile says its state and nothing else.** A press on it opens its panel in
 * the trailing sheet, where every act is: what it holds, Clear, Delete branch,
 * Forget Job, and for a bay Close, Reopen, Remove and Rescue with its Finding.
 */
export type PoolSlotsProps = {
  /** Bays first, then worktrees outside the pool, in the order they are drawn. */
  rows: readonly TileRow[];
  /** Open the Job holding a tile. */
  onOpenJob: (jobId: string) => void;
  /**
   * Change the pool: add a slot (no `slot`), or remove, close or reopen one.
   * Absent draws no slot acts and no add tile.
   */
  onAct?: (act: SlotAct, slot?: number) => void;
  /**
   * Rescue a stranded slot: start or stop its Scout, or Scrap, Stash or Pick up what
   * it holds. A Scrap is sent only from its confirm, in the panel. Absent draws none
   * of the acts.
   */
  onRescue?: (act: RescueAct, slot: number) => void;
  /** Give a worktree back. Sent only from Clear's confirm. Absent draws no Clear. */
  onClear?: (jobId: string) => void;
  /** Release a slot an agent session holds, for the holder it showed. Sent only from its confirm. */
  onRelease?: (slot: number, holder: string) => void;
  /** Delete a worktree's branch at the tip its confirm named. Absent draws no Delete branch. */
  onDeleteBranch?: (jobId: string, tip: string) => void;
  /** Delete a Job's record. Sent only from its confirm. Absent draws no Forget Job. */
  onForget?: (jobId: string) => void;
  /** Pause a Job that holds a bay. Sent only from its confirm. Absent draws no Pause. */
  onPause?: (jobId: string) => void;
  /** Resume a paused Job. Absent draws no Resume. */
  onResume?: (jobId: string) => void;
  /** A path or a branch is copied on a press; the surface confirms it. */
  onCopied?: (value: string) => void;
  /** Why the last add was refused, drawn on the add tile. */
  addRefused?: string;
  /** An add is out, so the tile waits. */
  adding?: boolean;
  /** The window is at `--window-floor`, where the panel is flush to both edges. */
  floor?: boolean;
};

/** The ghost tile after the last bay: one more slot, made by the next lease. */
function AddTile({ onAct, refused, adding }: { onAct: NonNullable<PoolSlotsProps["onAct"]>; refused?: string; adding?: boolean }) {
  return (
    <li className="armada-bay" data-bay="add" aria-busy={adding || undefined}>
      <Tooltip label="Add a slot">
        <button
          type="button"
          className="armada-bay__add"
          aria-label="Add a slot"
          aria-disabled={adding || undefined}
          onClick={adding ? undefined : () => onAct("add")}
        >
          <FolderPlus size={16} strokeWidth={2} aria-hidden />
        </button>
      </Tooltip>
      {refused === undefined ? null : (
        <p className="armada-bay__refused" role="alert">
          {refused}
        </p>
      )}
    </li>
  );
}

export function PoolSlots({
  rows,
  onOpenJob,
  onAct,
  onRescue,
  onClear,
  onRelease,
  onDeleteBranch,
  onForget,
  onPause,
  onResume,
  onCopied,
  addRefused,
  adding,
  floor = false,
}: PoolSlotsProps) {
  /** The tile whose panel is open. */
  const [opened, setOpened] = useState<string | null>(null);
  // A tile that went away, a slot removed or a record forgotten, has no panel left.
  const open = rows.find((row) => keyOf(row) === opened);
  const bays = rows.filter((row) => row.slot !== undefined);
  const outside = rows.filter((row) => row.slot === undefined);
  const tile = (row: TileRow) => (
    <PoolTile key={keyOf(row)} row={row} open={keyOf(row) === opened} onOpenJob={onOpenJob} onOpen={() => setOpened(keyOf(row))} />
  );
  return (
    <>
      <ul className="armada-pool-slots" aria-label="Worktree slots">
        {bays.map(tile)}
        {onAct === undefined ? null : <AddTile onAct={onAct} {...(addRefused === undefined ? {} : { refused: addRefused })} {...(adding === undefined ? {} : { adding })} />}
        {outside.map(tile)}
      </ul>
      {open === undefined ? null : (
        <TileSheet
          key={opened}
          row={open}
          floor={floor}
          onOpenJob={onOpenJob}
          {...(onAct === undefined ? {} : { onAct })}
          {...(onRescue === undefined ? {} : { onRescue })}
          {...(onClear === undefined ? {} : { onClear })}
          {...(onRelease === undefined ? {} : { onRelease })}
          {...(onDeleteBranch === undefined ? {} : { onDeleteBranch })}
          {...(onForget === undefined ? {} : { onForget })}
          {...(onPause === undefined ? {} : { onPause })}
          {...(onResume === undefined ? {} : { onResume })}
          {...(onCopied === undefined ? {} : { onCopied })}
          onClose={() => setOpened(null)}
        />
      )}
    </>
  );
}
