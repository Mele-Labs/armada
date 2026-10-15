import { useContext, useState } from "react";
import type { ReactNode } from "react";
import { releasable, releaseHolder } from "@armada/protocol";
import type { RescueAct, SlotAct } from "@armada/protocol";

import { Button } from "../../primitives/Button/Button";
import { ChipOwnership } from "../OwnerChip/OwnerChip";
import { ScrapConfirm, SlotFinding } from "./SlotFinding";
import { TileActs, rescuable } from "./TileActs";
import type { Confirming } from "./TileActs";
import { ReleaseConfirm } from "./ReleaseConfirm";
import { TileConfirm } from "./TileConfirm";
import { TileHolds } from "./TileHolds";
import type { TileRow } from "./tiles";

/** What managing one tile sends, each only where the caller can send it. */
export type TileManageProps = {
  row: TileRow;
  /** Absent draws no slot acts. */
  onAct?: ((act: SlotAct, slot?: number) => void) | undefined;
  /** Absent draws none of the rescue acts. */
  onRescue?: ((act: RescueAct, slot: number) => void) | undefined;
  /** Absent draws no Clear. */
  onClear?: ((jobId: string) => void) | undefined;
  /** Release a slot an agent session holds, for the holder it showed. */
  onRelease?: ((slot: number, holder: string) => void) | undefined;
  /** Absent draws no Delete branch. Sends the tip the person confirmed. */
  onDeleteBranch?: ((jobId: string, tip: string) => void) | undefined;
  /** Absent draws no Forget Job. */
  onForget?: ((jobId: string) => void) | undefined;
  /** Absent draws no Pause. Sent from its confirm. */
  onPause?: ((jobId: string) => void) | undefined;
  /** Absent draws no Resume. */
  onResume?: ((jobId: string) => void) | undefined;
};

/**
 * Everything a tile can be managed with, apart from where it is drawn: its acts, what the last one said, the confirm that is
 * open, what the worktree holds and the Scout's Finding. **One body for the tile's sheet and for a bay's pane on the Cockpit**, so
 * the same acts carry the same confirms and tooltips wherever the bay is opened. `foot` is the rescue buttons that follow a
 * Finding: the sheet puts them in its footer and the pane under the body. `reading` is a Scout reading now, whose Stop the
 * acts already carry.
 */
export function useTileManage({ row, onAct, onRescue, onClear, onRelease, onDeleteBranch, onForget, onPause, onResume }: TileManageProps): {
  body: ReactNode;
  foot: ReactNode | undefined;
  reading: boolean;
} {
  const { slot, held } = row;
  /** The confirm open on this tile, if one is. */
  const [confirming, setConfirming] = useState<Confirming | null>(null);
  /** Where a Session is opened from, when the window serves any. */
  const ownership = useContext(ChipOwnership);
  /** The Scrap's confirm is open. */
  const [scrapping, setScrapping] = useState(false);
  const waiting = row.acting === true || confirming !== null || scrapping;
  const rescue = slot !== undefined && rescuable(slot) ? slot.rescue : undefined;
  const reading = rescue?.state === "reading";
  const toContinue =
    rescue?.verdict === "unfinished" ||
    (rescue?.verdict === undefined && (rescue?.summary !== undefined || (rescue?.items ?? []).length > 0));
  const unmerged = held?.held.find((reason) => reason.why === "unmerged");

  const send = () => {
    const which = confirming;
    setConfirming(null);
    if (which === "release") {
      const holder = slot === undefined ? undefined : releaseHolder(slot);
      if (slot !== undefined && holder !== undefined) onRelease?.(slot.slot, holder);
      return;
    }
    if (held === undefined) return;
    if (which === "pause") onPause?.(held.job_id);
    else if (which === "clear") onClear?.(held.job_id);
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

  const body = (
    <>
      <TileActs
        row={row}
        waiting={waiting}
        onAct={onAct}
        onRescue={onRescue}
        onResume={held === undefined || onResume === undefined ? undefined : () => onResume(held.job_id)}
        onConfirm={held === undefined && !(slot !== undefined && releasable(slot) && onRelease !== undefined) ? undefined : (which) => setConfirming(which)}
      />
      {row.refused === undefined ? null : (
        <>
          <p className="armada-tile-sheet__refused" role="alert">
            {row.refused}
          </p>
          {row.openSession === undefined || ownership === null ? null : (
            <Button variant="secondary" size="sm" onClick={() => ownership.open(row.openSession!)}>
              Open Session
            </Button>
          )}
        </>
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
      {scrapping && slot !== undefined ? <ScrapConfirm slot={slot} onScrap={() => rescued("scrap")} onCancel={() => setScrapping(false)} /> : null}
    </>
  );

  return { body, foot, reading };
}
