import { Folder, GitBranch } from "lucide-react";
import type { RescueAct, SlotAct } from "@armada/protocol";

import { Sheet } from "../../primitives/Sheet/Sheet";
import { stateOf } from "./PoolTile";
import { useTileManage } from "./TileManage";
import { nameOf } from "./tiles";
import type { TileRow } from "./tiles";

/**
 * One tile's panel, in the trailing sheet the app opens everything else in:
 * the acts that fit the bay and, for a stranded one, the Scout's Finding, with
 * only the acts that apply. `floating`, as the
 * Record's and the Drones' sheets are, because a tile is one cell of a grid
 * and a sheet contained in the screen would cover its neighbours' column.
 *
 * **Every destructive act is sent from its confirm**, which names what would
 * go. What an act did, and what Fleet refused, is said here, under the acts.
 *
 * **Pick up is offered where there is work to continue**: a verdict of
 * Unfinished, or no verdict and the Scout's own words. Scraps has nothing to
 * continue, and a Finding with neither has nothing to carry on from.
 *
 * The acts and their confirms are `useTileManage`'s, which a bay's pane on the
 * Cockpit draws too.
 */
export type TileSheetProps = {
  row: TileRow;
  floor: boolean;
  /** Absent draws no slot acts. */
  onAct?: (act: SlotAct, slot?: number) => void;
  /** Absent draws none of the rescue acts. */
  onRescue?: (act: RescueAct, slot: number) => void;
  /** Release a slot an agent session holds, for the holder it showed. */
  onRelease?: (slot: number, holder: string) => void;
  /** A path or a branch is copied on a press; the surface confirms it. */
  onCopied?: (value: string) => void;
  onClose: () => void;
};

/** A value that gets pasted into a shell: a path, a branch. */
function Copied({ Glyph, said, value, onCopied }: { Glyph: typeof Folder; said: string; value: string; onCopied?: (value: string) => void }) {
  const inner = (
    <>
      <Glyph size={12} strokeWidth={2} aria-hidden />
      <span>{value}</span>
    </>
  );
  return onCopied === undefined ? (
    <span className="armada-tile-sheet__value">{inner}</span>
  ) : (
    <button
      type="button"
      className="armada-tile-sheet__value"
      aria-label={`Copy ${said}`}
      onClick={() => void navigator.clipboard.writeText(value).then(() => onCopied(value), () => onCopied(value))}
    >
      {inner}
    </button>
  );
}

export function TileSheet({ row, floor, onAct, onRescue, onRelease, onCopied, onClose }: TileSheetProps) {
  const { slot } = row;
  const state = stateOf(row);
  const { path, branch } = slot;
  const { body, foot } = useTileManage({ row, onAct, onRescue, onRelease });

  return (
    <Sheet
      kind="cleanup-tile"
      open
      floating
      floor={floor}
      title={nameOf(row)}
      subtitle={
        <span className="armada-tile-sheet__about">
          <span className="armada-bay__state" role="img" aria-label={state.said}>
            <state.Glyph size={12} strokeWidth={2} aria-hidden />
          </span>
        </span>
      }
      closeLabel="Close panel"
      closeBinding="Esc"
      {...(foot === undefined ? {} : { footer: foot })}
      onClose={onClose}
    >
      <div className="armada-tile-sheet__body">
        {body}
        <div className="armada-tile-sheet__where">
          <Copied Glyph={Folder} said="path" value={path} {...(onCopied === undefined ? {} : { onCopied })} />
          {branch === undefined ? null : <Copied Glyph={GitBranch} said="branch" value={branch} {...(onCopied === undefined ? {} : { onCopied })} />}
        </div>
      </div>
    </Sheet>
  );
}
