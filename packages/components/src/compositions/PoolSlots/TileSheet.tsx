import { Folder, GitBranch } from "lucide-react";
import { useState } from "react";
import type { RescueAct, SlotAct } from "@armada/protocol";

import { Button } from "../../primitives/Button/Button";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { JobLink, jobOf, stateOf } from "./PoolTile";
import { ScrapConfirm, SlotFinding } from "./SlotFinding";
import { TileActs, rescuable } from "./TileActs";
import type { Confirming } from "./TileActs";
import { ReleaseConfirm } from "./ReleaseConfirm";
import { TileConfirm } from "./TileConfirm";
import { TileHolds } from "./TileHolds";
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
 */
export type TileSheetProps = {
  row: TileRow;
  floor: boolean;
  onOpenJob: (jobId: string) => void;
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

export function TileSheet({ row, floor, onOpenJob, onAct, onRescue, onClear, onRelease, onDeleteBranch, onForget, onCopied, onClose }: TileSheetProps) {
  const { slot, held } = row;
  /** The confirm open on this tile, if one is. */
  const [confirming, setConfirming] = useState<Confirming | null>(null);
  /** The Scrap's confirm is open. */
  const [scrapping, setScrapping] = useState(false);
  const state = stateOf(row);
  const job = jobOf(row);
  const waiting = row.acting === true || confirming !== null || scrapping;
  const rescue = slot !== undefined && rescuable(slot) ? slot.rescue : undefined;
  const reading = rescue?.state === "reading";
  const toContinue =
    rescue?.verdict === "unfinished" ||
    (rescue?.verdict === undefined && (rescue?.summary !== undefined || (rescue?.items ?? []).length > 0));
  const unmerged = held?.held.find((reason) => reason.why === "unmerged");
  const path = slot?.path ?? held?.path;
  const branch = slot?.branch ?? held?.branch;

  const send = () => {
    const which = confirming;
    setConfirming(null);
    if (which === "release") {
      if (slot !== undefined && slot.held.state === "session") onRelease?.(slot.slot, slot.held.holder);
      return;
    }
    if (held === undefined) return;
    if (which === "clear") onClear?.(held.job_id);
    else if (which === "branch" && unmerged !== undefined) onDeleteBranch?.(held.job_id, unmerged.tip);
    else if (which === "forget") onForget?.(held.job_id);
  };
  const rescued = (which: RescueAct) => {
    setScrapping(false);
    if (slot !== undefined) onRescue?.(which, slot.slot);
  };

  const foot =
    onRescue === undefined || slot === undefined || rescue === undefined || scrapping || confirming !== null ? undefined : reading ? (
      <Button variant="secondary" size="sm" ground="sunken" disabled={row.acting === true} onClick={() => rescued("stop")}>
        Stop
      </Button>
    ) : (
      <>
        {toContinue ? (
          <Button variant="secondary" size="sm" ground="sunken" disabled={row.acting === true} onClick={() => rescued("pick_up")}>
            Pick up
          </Button>
        ) : null}
        <Button variant="secondary" size="sm" ground="sunken" disabled={row.acting === true} onClick={() => rescued("stash")}>
          Stash
        </Button>
        <Button variant="destructive" size="sm" disabled={row.acting === true} onClick={() => setScrapping(true)}>
          Scrap
        </Button>
      </>
    );

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
        </span>
      }
      closeLabel="Close panel"
      closeBinding="Esc"
      {...(foot === undefined ? {} : { footer: foot })}
      onClose={onClose}
    >
      <div className="armada-tile-sheet__body">
        <TileActs
          row={row}
          waiting={waiting}
          onAct={onAct}
          onRescue={onRescue}
          onConfirm={held === undefined && slot?.held.state !== "session" ? undefined : (which) => setConfirming(which)}
        />
        {row.refused === undefined ? null : (
          <p className="armada-tile-sheet__refused" role="alert">
            {row.refused}
          </p>
        )}
        {row.said === undefined && row.receipt === undefined ? null : (
          <ul className="armada-tile-sheet__said" role="status">
            {[...(row.said === undefined ? [] : [row.said]), ...(row.receipt ?? [])].map((one) => (
              <li key={one}>{one}</li>
            ))}
          </ul>
        )}
        {confirming === "release" ? <ReleaseConfirm row={row} onSend={send} onCancel={() => setConfirming(null)} /> : null}
        {confirming === null || confirming === "release" || held === undefined ? null : (
          <TileConfirm which={confirming} row={row} cost={row.cost ?? { files: [] }} onSend={send} onCancel={() => setConfirming(null)} />
        )}
        {held === undefined ? null : <TileHolds reasons={held.held} sat={row.sat} status={held.status} job={row.job} />}
        {slot === undefined || rescue === undefined ? null : <SlotFinding slot={slot} />}
        {scrapping && slot !== undefined ? (
          <ScrapConfirm slot={slot} onScrap={() => rescued("scrap")} onCancel={() => setScrapping(false)} />
        ) : null}
        <div className="armada-tile-sheet__where">
          {path === undefined ? null : <Copied Glyph={Folder} said="path" value={path} {...(onCopied === undefined ? {} : { onCopied })} />}
          {branch === undefined ? null : <Copied Glyph={GitBranch} said="branch" value={branch} {...(onCopied === undefined ? {} : { onCopied })} />}
        </div>
      </div>
    </Sheet>
  );
}
