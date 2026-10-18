import { useContext, useState } from "react";
import type { ReactNode } from "react";
import { releasable, releaseHolder } from "@armada/protocol";
import type { RescueAct, SlotAct } from "@armada/protocol";

import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { ChipOwnership } from "../OwnerChip/OwnerChip";
import { ScrapConfirm, SlotFinding } from "./SlotFinding";
import { TileActs, rescuable, rescueTipsOf } from "./TileActs";
import { ReleaseConfirm } from "./ReleaseConfirm";
import type { TileRow } from "./tiles";

/** What managing one tile sends, each only where the caller can send it. */
export type TileManageProps = {
  row: TileRow;
  /** Absent draws no slot acts. */
  onAct?: ((act: SlotAct, slot?: number) => void) | undefined;
  /** Absent draws none of the rescue acts. */
  onRescue?: ((act: RescueAct, slot: number) => void) | undefined;
  /** Release a slot an agent session holds, for the holder it showed. */
  onRelease?: ((slot: number, holder: string) => void) | undefined;
};

/**
 * Everything a tile can be managed with, apart from where it is drawn: its acts, what the last one said, the confirm that is
 * open, what the worktree holds and the Scout's Finding. **One body for the tile's sheet and for a bay's pane on the Cockpit**, so
 * the same acts carry the same confirms and tooltips wherever the bay is opened. `foot` is the rescue buttons that follow a
 * Finding: the sheet puts them in its footer and the pane under the body. `reading` is a Scout reading now, whose Stop the
 * acts already carry.
 */
export function useTileManage({ row, onAct, onRescue, onRelease }: TileManageProps): {
  body: ReactNode;
  foot: ReactNode | undefined;
  reading: boolean;
} {
  const { slot } = row;
  /** Release's confirm is open. */
  const [releasing, setReleasing] = useState(false);
  /** Where a Session is opened from, when the window serves any. */
  const ownership = useContext(ChipOwnership);
  /** The Scrap's confirm is open. */
  const [scrapping, setScrapping] = useState(false);
  const waiting = row.acting === true || releasing || scrapping;
  const rescue = rescuable(slot) ? slot.rescue : undefined;
  const reading = rescue?.state === "reading";
  const toContinue =
    rescue?.verdict === "unfinished" ||
    (rescue?.verdict === undefined && (rescue?.summary !== undefined || (rescue?.items ?? []).length > 0));

  const send = () => {
    setReleasing(false);
    const holder = releaseHolder(slot);
    if (holder !== undefined) onRelease?.(slot.slot, holder);
  };
  const rescued = (which: RescueAct) => {
    setScrapping(false);
    onRescue?.(which, slot.slot);
  };

  const tips = rescueTipsOf(row);
  const foot =
    onRescue === undefined || rescue === undefined || scrapping || releasing ? undefined : reading ? (
      <Tooltip label={tips.stop}>
        <Button variant="secondary" size="sm" ground="sunken" disabled={row.acting === true} onClick={() => rescued("stop")}>
          Stop
        </Button>
      </Tooltip>
    ) : (
      <>
        {toContinue ? (
          <Tooltip label={tips.pick_up}>
            <Button variant="secondary" size="sm" ground="sunken" disabled={row.acting === true} onClick={() => rescued("pick_up")}>
              Pick up
            </Button>
          </Tooltip>
        ) : null}
        <Tooltip label={tips.stash}>
          <Button variant="secondary" size="sm" ground="sunken" disabled={row.acting === true} onClick={() => rescued("stash")}>
            Stash
          </Button>
        </Tooltip>
        <Tooltip label={tips.scrap}>
          <Button variant="destructive" size="sm" disabled={row.acting === true} onClick={() => setScrapping(true)}>
            Scrap
          </Button>
        </Tooltip>
      </>
    );

  const body = (
    <>
      <TileActs
        row={row}
        waiting={waiting}
        onAct={onAct}
        onRescue={onRescue}
        onRelease={releasable(slot) && onRelease !== undefined ? () => setReleasing(true) : undefined}
      />
      {row.refused === undefined ? null : (
        <>
          <p className="armada-tile-sheet__refused" role="alert">
            {row.refused}
          </p>
          {row.openSession === undefined || ownership === null ? null : (
            <Button variant="secondary" size="sm" onClick={() => ownership.open(row.openSession!)}>
              Open Job
            </Button>
          )}
        </>
      )}
      {row.said === undefined ? null : (
        <ul className="armada-tile-sheet__said" role="status">
          <li>{row.said}</li>
        </ul>
      )}
      {releasing ? <ReleaseConfirm row={row} onSend={send} onCancel={() => setReleasing(false)} /> : null}
      {rescue === undefined ? null : <SlotFinding slot={slot} />}
      {scrapping ? <ScrapConfirm slot={slot} onScrap={() => rescued("scrap")} onCancel={() => setScrapping(false)} /> : null}
    </>
  );

  return { body, foot, reading };
}
