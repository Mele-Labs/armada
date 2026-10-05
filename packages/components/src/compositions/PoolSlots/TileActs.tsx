import { DoorClosedLocked, DoorOpen, Eraser, GitBranchMinus, LifeBuoy, PackageX, Power, Trash2 } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { RescueAct, SlotAct, WorktreeSlot } from "@armada/protocol";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { Offered } from "./tiles";

/** One act on a tile: a glyph in a bordered button, named by its tooltip and its accessible name. */
function Act({ said, Glyph, waiting, onPress }: { said: string; Glyph: LucideIcon; waiting: boolean; onPress: () => void }) {
  return (
    <Tooltip label={said}>
      <button
        type="button"
        className="armada-bay__act"
        aria-label={said}
        aria-disabled={waiting || undefined}
        onClick={waiting ? undefined : onPress}
      >
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </button>
    </Tooltip>
  );
}

/** A stranded slot, or a Job's that ended and kept its slot: both hold work a person can rescue. */
export function rescuable(slot: WorktreeSlot): boolean {
  return slot.held.state === "stranded" || (slot.held.state === "job" && slot.held.kept !== undefined);
}

/** Which confirm an act opens in the panel, before it is sent. */
export type Confirming = "clear" | "branch" | "forget";

export type TileActsProps = {
  slot?: WorktreeSlot | undefined;
  offered?: Offered | undefined;
  /** An act on the tile is out, or a confirm is open: the acts wait. */
  waiting: boolean;
  onAct?: ((act: SlotAct, slot?: number) => void) | undefined;
  onRescue?: ((act: RescueAct, slot: number) => void) | undefined;
  onConfirm?: ((which: Confirming) => void) | undefined;
};

/**
 * The acts a tile takes, each only where it applies. Slot acts come from the
 * bay: Rescue or Stop, Close or Reopen, and Remove where the pool would let the
 * slot go. Worktree acts come from the Job's reading: Clear, Delete branch and
 * Forget Job, each opening its confirm. **Room is left after them** for acts
 * that land later, which are the screen's to offer and not this row's to guess.
 */
export function TileActs({ slot, offered, waiting, onAct, onRescue, onConfirm }: TileActsProps) {
  const removable = slot !== undefined && (slot.held.state === "free" || slot.held.state === "unmade");
  const rescue = slot?.rescue;
  const acts = [
    onRescue !== undefined && slot !== undefined && rescuable(slot) && rescue === undefined ? (
      <Act key="rescue" said="Rescue" Glyph={LifeBuoy} waiting={waiting} onPress={() => onRescue("start", slot.slot)} />
    ) : null,
    onRescue !== undefined && slot !== undefined && rescuable(slot) && rescue?.state === "reading" ? (
      <Act key="stop" said="Stop" Glyph={Power} waiting={waiting} onPress={() => onRescue("stop", slot.slot)} />
    ) : null,
    onConfirm !== undefined && offered?.clear === true ? (
      <Act key="clear" said="Clear" Glyph={Eraser} waiting={waiting} onPress={() => onConfirm("clear")} />
    ) : null,
    onConfirm !== undefined && offered?.deleteBranch === true ? (
      <Act key="branch" said="Delete branch" Glyph={GitBranchMinus} waiting={waiting} onPress={() => onConfirm("branch")} />
    ) : null,
    onConfirm !== undefined && offered?.forget === true ? (
      <Act key="forget" said="Forget Job" Glyph={PackageX} waiting={waiting} onPress={() => onConfirm("forget")} />
    ) : null,
    onAct === undefined || slot === undefined ? null : slot.closed === true ? (
      <Act key="open" said="Reopen" Glyph={DoorOpen} waiting={waiting} onPress={() => onAct("open", slot.slot)} />
    ) : (
      <Act key="close" said="Close" Glyph={DoorClosedLocked} waiting={waiting} onPress={() => onAct("close", slot.slot)} />
    ),
    onAct !== undefined && slot !== undefined && removable && slot.held.state !== "busy" ? (
      <Act key="remove" said="Remove" Glyph={Trash2} waiting={waiting} onPress={() => onAct("remove", slot.slot)} />
    ) : null,
  ].filter((one) => one !== null);
  if (acts.length === 0) return null;
  return (
    <div className="armada-tile-sheet__acts" role="group" aria-label="Acts">
      {acts}
    </div>
  );
}
