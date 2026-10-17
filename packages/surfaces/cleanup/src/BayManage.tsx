// Every act Cleanup's tile panel offers on a bay, drawn for the bay's pane on the Cockpit: the same acts, confirms and tooltips,
// in whatever state the bay is in, sent through the same handlers (`tile-acts.ts`). Nothing is drawn where the window has no
// way to send an act.

import { useTileManage } from "@armada/components";
import type { TileRow } from "@armada/components";
import type { WorktreeSlot } from "@armada/protocol";

import { sitting } from "@armada/screens/src/held";
import { keyOf, useTileActs } from "./tile-acts";
import type { TileActsIn } from "./tile-acts";

export type BayManageProps = TileActsIn & {
  slot: WorktreeSlot;
};

/** The acts a pane can send for a bay. Each is absent where the window has no way to send it. */
export type BayActs = Pick<TileActsIn, "onChange" | "onRescue">;

/**
 * Drawn under `armada-bay-manage`, which the pane styles. The slot's `key` is the caller's to set per bay, so a confirm left open
 * on one bay is not carried to the next.
 */
export function BayManage({ slot, ...rest }: BayManageProps) {
  const { worked, acts } = useTileActs(rest);
  const key = keyOf(slot.manifest_id, slot.slot);
  const heldFor = slot.since === undefined ? null : sitting(slot.since, rest.now);
  const row: TileRow = { key, name: `slot-${slot.slot}`, slot, ...(heldFor === null ? {} : { heldFor }), ...worked(key) };
  const { body, foot, reading } = useTileManage({ row, ...acts(slot.manifest_id) });
  return (
    <div className="armada-bay-manage">
      {body}
      {reading || foot === undefined ? null : <div className="armada-bay-manage__foot">{foot}</div>}
    </div>
  );
}
