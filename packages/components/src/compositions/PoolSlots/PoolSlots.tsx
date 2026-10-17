import { FolderPlus } from "lucide-react";
import { useRef, useState } from "react";
import type { RescueAct, SlotAct } from "@armada/protocol";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { useTileGrid } from "../../tile-grid";
import { PoolTile } from "./PoolTile";
import { TileSheet } from "./TileSheet";
import { CURSOR, focusTile } from "./keys";
import { keyOf } from "./tiles";
import type { TileRow } from "./tiles";

export type { TileRow } from "./tiles";
export { ClearSaves } from "./ClearSaves";
export { TileSheet } from "./TileSheet";
export { useTileManage } from "./TileManage";
export type { TileManageProps } from "./TileManage";
export { ReleaseConfirm } from "./ReleaseConfirm";

/**
 * Cleanup's grid, one tile per bay and each styled by what holds it. Held is a
 * filled card under a band in the leased hue, free an open dashed outline,
 * stranded hatched in the warning hue, and a slot not made a faint ghost. A
 * slot a person closed is shuttered in the closed hue; a held one closed keeps
 * its holder and takes the closed mark. `armada worktree --status` is the same
 * reading. Marks are group `Worktree slot` in `packages/icons/icons/`.
 *
 * **A tile says its state and nothing else.** A press on it opens its panel in
 * the trailing sheet, where every act is: Close, Reopen, Remove, Release and
 * Rescue with its Finding.
 */
export type PoolSlotsProps = {
  /** The bays, in the order they are drawn. */
  rows: readonly TileRow[];
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
  /** Release a slot an agent session holds, for the holder it showed. Sent only from its confirm. */
  onRelease?: (slot: number, holder: string) => void;
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
  onAct,
  onRescue,
  onRelease,
  onCopied,
  addRefused,
  adding,
  floor = false,
}: PoolSlotsProps) {
  /** The tile whose panel is open. */
  const [opened, setOpened] = useState<string | null>(null);
  const view = useRef<HTMLDivElement>(null);
  // One Tab stop for the pool, the arrows and j and k across it; o opens a tile's panel, as a press on its name does.
  useTileGrid(view, {
    tile: ".armada-bay",
    cursor: CURSOR,
    onOpen: (tile) => {
      const name = tile.querySelector<HTMLElement>(".armada-bay__name");
      if (name === null) return false;
      name.click();
    },
  });
  // A tile that went away, a slot removed, has no panel left.
  const open = rows.find((row) => keyOf(row) === opened);
  const pooled = rows.length > 0 || onAct !== undefined;
  const tile = (row: TileRow) => <PoolTile key={keyOf(row)} row={row} open={keyOf(row) === opened} onOpen={() => setOpened(keyOf(row))} />;
  return (
    <>
      {/* The cockpit's viewscreen: a band over a ruled glass the tiles stand on. A band repeats its list's name, so it is unread. */}
      <div className="armada-pool-slots__views" ref={view}>
        {pooled ? (
          <div className="armada-pool-slots__view">
            <div className="armada-pool-slots__band" aria-hidden>
              Worktree slots
            </div>
            <ul className="armada-pool-slots" aria-label="Worktree slots">
              {rows.map(tile)}
              {onAct === undefined ? null : <AddTile onAct={onAct} {...(addRefused === undefined ? {} : { refused: addRefused })} {...(adding === undefined ? {} : { adding })} />}
            </ul>
          </div>
        ) : null}
      </div>
      {open === undefined ? null : (
        <TileSheet
          key={opened}
          row={open}
          floor={floor}
          {...(onAct === undefined ? {} : { onAct })}
          {...(onRescue === undefined ? {} : { onRescue })}
          {...(onRelease === undefined ? {} : { onRelease })}
          {...(onCopied === undefined ? {} : { onCopied })}
          onClose={() => {
            // The sheet takes focus when it opens, so closing it gives focus back to the tile it was opened from.
            if (view.current !== null && opened !== null) focusTile(view.current, opened);
            setOpened(null);
          }}
        />
      )}
    </>
  );
}
