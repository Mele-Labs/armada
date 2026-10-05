import { useState } from "react";
import type { RescueAct } from "@armada/protocol";

import { Button } from "../../primitives/Button/Button";
import { Sheet } from "../../primitives/Sheet/Sheet";
import type { PoolSlotRow } from "./PoolSlots";
import { ScrapConfirm, SlotFinding } from "./SlotFinding";

/**
 * A bay's rescue, read in the trailing sheet the app opens everything else in:
 * the Scout's read while it goes, its Finding after, and the acts on the work. `floating`, as the Record's and the Drones' sheets are, because a bay
 * is one tile in a grid and a sheet contained in the screen would cover its
 * neighbours' column rather than the work area.
 *
 * **A Scrap is sent only from its confirm**, which names what would go. The
 * sheet closes on the press of Scrap, Stash or Pick up, so a refusal is said on
 * the bay, where every other act on a bay says it.
 *
 * **Pick up is offered where there is work to continue**: a verdict of
 * Unfinished, or no verdict and the Scout's own words. Scraps has nothing to
 * continue, and a Finding with neither has nothing to brief a Job with.
 */
export type SlotSheetProps = {
  row: PoolSlotRow;
  floor: boolean;
  /** Absent draws no acts. */
  onRescue?: (act: RescueAct, slot: number) => void;
  onClose: () => void;
};

export function SlotSheet({ row, floor, onRescue, onClose }: SlotSheetProps) {
  const { slot } = row;
  /** The Scrap's confirm is open. */
  const [scrapping, setScrapping] = useState(false);
  const reading = slot.rescue?.state === "reading";
  const waiting = row.acting === true;
  const finding = slot.rescue;
  const toContinue =
    finding?.verdict === "unfinished" ||
    (finding?.verdict === undefined && (finding?.summary !== undefined || (finding?.items ?? []).length > 0));
  const act = (which: RescueAct) => {
    onClose();
    onRescue?.(which, slot.slot);
  };

  const foot =
    onRescue === undefined || scrapping ? undefined : reading ? (
      <Button variant="secondary" size="sm" ground="sunken" disabled={waiting} onClick={() => onRescue("stop", slot.slot)}>
        Stop
      </Button>
    ) : (
      <>
        {toContinue ? (
          <Button variant="secondary" size="sm" ground="sunken" disabled={waiting} onClick={() => act("pick_up")}>
            Pick up
          </Button>
        ) : null}
        <Button variant="secondary" size="sm" ground="sunken" disabled={waiting} onClick={() => act("stash")}>
          Stash
        </Button>
        <Button variant="destructive" size="sm" disabled={waiting} onClick={() => setScrapping(true)}>
          Scrap
        </Button>
      </>
    );

  return (
    <Sheet
      kind="slot-finding"
      open
      floating
      floor={floor}
      title="Finding"
      subtitle={
        <span className="armada-slot-sheet__about">
          <span>{`slot-${slot.slot}`}</span>
          {slot.branch === undefined ? null : <span>{slot.branch}</span>}
        </span>
      }
      closeLabel="Close"
      closeBinding="Esc"
      {...(foot === undefined ? {} : { footer: foot })}
      onClose={onClose}
    >
      <div className="armada-slot-sheet__body">
        <SlotFinding slot={slot} />
        {scrapping ? (
          <ScrapConfirm slot={slot} onScrap={() => act("scrap")} onCancel={() => setScrapping(false)} />
        ) : null}
      </div>
    </Sheet>
  );
}
