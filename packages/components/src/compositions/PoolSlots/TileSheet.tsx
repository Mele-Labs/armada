import { Folder, GitBranch } from "lucide-react";
import type { RescueAct, SlotAct } from "@armada/protocol";

import { Sheet } from "../../primitives/Sheet/Sheet";
import { PausedMark } from "../PausedMark/PausedMark";
import { JobLink, jobOf, stateOf } from "./PoolTile";
import { useTileManage } from "./TileManage";
import { nameOf } from "./tiles";
import type { TileRow } from "./tiles";

/**
 * One tile's panel, in the trailing sheet the app opens everything else in:
 * what the worktree holds, the acts that fit it, and for a stranded bay the
 * Scout's Finding. **One panel for every kind of tile**, a bay or a Job's
 * worktree outside the pool, with only the acts that apply. `floating`, as the
 * Record's and the Drones' sheets are, because a tile is one cell of a grid
 * and a sheet contained in the screen would cover its neighbours' column.
 *
 * **Every destructive act is sent from its confirm**, which names what would
 * go. What an act did, and what Fleet refused, is said here, under the acts.
 *
 * **Pick up is offered where there is work to continue**: a verdict of
 * Unfinished, or no verdict and the Scout's own words. Scraps has nothing to
 * continue, and a Finding with neither has nothing to brief a Job with.
 *
 * The acts and their confirms are `useTileManage`'s, which a bay's pane on the
 * Cockpit draws too.
 */
export type TileSheetProps = {
  row: TileRow;
  floor: boolean;
  /** Absent draws the Job as a name. */
  onOpenJob?: ((jobId: string) => void) | undefined;
  /** Absent draws no slot acts. */
  onAct?: (act: SlotAct, slot?: number) => void;
  /** Absent draws none of the rescue acts. */
  onRescue?: (act: RescueAct, slot: number) => void;
  /** Absent draws no Clear. */
  onClear?: (jobId: string) => void;
  /** Release a slot an agent session holds, for the holder it showed. */
  onRelease?: (slot: number, holder: string) => void;
  /** Absent draws no Delete branch. Sends the tip the person confirmed. */
  onDeleteBranch?: (jobId: string, tip: string) => void;
  /** Absent draws no Forget Job. */
  onForget?: (jobId: string) => void;
  /** Absent draws no Pause. Sent from its confirm. */
  onPause?: (jobId: string) => void;
  /** Absent draws no Resume. */
  onResume?: (jobId: string) => void;
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

export function TileSheet({ row, floor, onOpenJob, onAct, onRescue, onClear, onRelease, onDeleteBranch, onForget, onPause, onResume, onCopied, onClose }: TileSheetProps) {
  const { slot, held } = row;
  const state = stateOf(row);
  const job = jobOf(row);
  const path = slot?.path ?? held?.path;
  const branch = slot?.branch ?? held?.branch;
  const { body, foot } = useTileManage({ row, onAct, onRescue, onClear, onRelease, onDeleteBranch, onForget, onPause, onResume });

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
          {job === null ? null : <JobLink jobId={job.jobId} title={job.title} onOpenJob={onOpenJob} />}
          {row.paused === undefined ? null : <PausedMark said={row.paused} />}
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
          {path === undefined ? null : <Copied Glyph={Folder} said="path" value={path} {...(onCopied === undefined ? {} : { onCopied })} />}
          {branch === undefined ? null : <Copied Glyph={GitBranch} said="branch" value={branch} {...(onCopied === undefined ? {} : { onCopied })} />}
        </div>
      </div>
    </Sheet>
  );
}
