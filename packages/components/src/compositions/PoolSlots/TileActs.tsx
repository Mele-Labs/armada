import { DoorClosedLocked, DoorOpen, LifeBuoy, Power, Trash2, Unplug } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { releasable } from "@armada/protocol";
import type { RescueAct, SlotAct, WorktreeSlot } from "@armada/protocol";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { nameOf } from "./tiles";
import type { TileRow } from "./tiles";

/** How an act reads at rest: destructive in the error hue, a rescue in the warning hue, the rest neutral. */
type Tone = "destructive" | "rescue" | "neutral";

/**
 * One act on a tile: a glyph and its label in a bordered button, tinted by its
 * tone. **The tooltip says what the act does in git's words**: which directory
 * or branch it acts on and what happens to it.
 */
function Act({
  label,
  said,
  Glyph,
  tone,
  waiting,
  onPress,
}: {
  label: string;
  said: string;
  Glyph: LucideIcon;
  tone: Tone;
  waiting: boolean;
  onPress: () => void;
}) {
  return (
    <Tooltip label={said}>
      <button
        type="button"
        className="armada-tile-act"
        data-tone={tone}
        aria-disabled={waiting || undefined}
        onClick={waiting ? undefined : onPress}
      >
        <Glyph size={16} strokeWidth={2} aria-hidden />
        <span>{label}</span>
      </button>
    </Tooltip>
  );
}

/** A stranded slot holds work a person can rescue. */
export function rescuable(slot: WorktreeSlot): boolean {
  return slot.held.state === "stranded";
}

export type TileActsProps = {
  row: TileRow;
  /** An act on the tile is out, or a confirm is open: the acts wait. */
  waiting: boolean;
  onAct?: ((act: SlotAct, slot?: number) => void) | undefined;
  onRescue?: ((act: RescueAct, slot: number) => void) | undefined;
  /** Open Release's confirm. */
  onRelease?: (() => void) | undefined;
};

/**
 * What each rescue act does, from `RescueAct` in Fleet and `rescue` in the
 * leasing adapter. **Stash commits what is uncommitted to the branch, pushes it
 * under its own name and frees the slot.** Pick up is a Stash that then carries
 * on from that branch. Scrap resets and cleans the checkout, and deletes the
 * branch only where the base has all its commits.
 */
export function rescueTipsOf(row: TileRow): Record<"stop" | "pick_up" | "stash" | "scrap", string> {
  const { slot } = row;
  const name = nameOf(row);
  const branch = slot.branch ?? "its branch";
  const base = slot.base;
  const stashed = `Commits the uncommitted files in ${name} to branch ${branch} as a WIP commit, pushes ${branch} to origin and releases the slot`;
  return {
    stop: `Stops the Scout reading ${name}.`,
    pick_up: `${stashed}, then carries on from ${branch}.`,
    stash: `${stashed}.`,
    scrap: `Discards the uncommitted and untracked files in ${name} and releases the slot. Deletes branch ${branch} only if ${base} has all its commits, otherwise keeps it.`,
  };
}

/**
 * The acts a tile takes, each only where it applies: Rescue or Stop, Release,
 * Close slot or Reopen slot, and Remove slot where the pool would let it go.
 */
export function TileActs({ row, waiting, onAct, onRescue, onRelease }: TileActsProps) {
  const { slot } = row;
  const name = nameOf(row);
  const removable = slot.held.state === "free" || slot.held.state === "unmade";
  const rescue = slot.rescue;
  const base = slot.base;
  const acts = [
    onRescue !== undefined && rescuable(slot) && rescue === undefined ? (
      <Act
        key="rescue"
        label="Rescue"
        said={`Starts a Scout that reads the uncommitted changes and the commits not on ${base} in ${name}. It changes nothing.`}
        Glyph={LifeBuoy}
        tone="rescue"
        waiting={waiting}
        onPress={() => onRescue("start", slot.slot)}
      />
    ) : null,
    onRescue !== undefined && rescuable(slot) && rescue?.state === "reading" ? (
      <Act key="stop" label="Stop" said={rescueTipsOf(row).stop} Glyph={Power} tone="neutral" waiting={waiting} onPress={() => onRescue("stop", slot.slot)} />
    ) : null,
    onRelease !== undefined && releasable(slot) ? (
      <Act
        key="release"
        label="Release"
        said={`Commits any uncommitted files in ${name} to ${slot.branch ?? "its branch"} as a WIP commit, then releases the slot. Keeps the branch.`}
        Glyph={Unplug}
        tone="neutral"
        waiting={waiting}
        onPress={onRelease}
      />
    ) : null,
    onAct === undefined ? null : slot.closed === true ? (
      <Act
        key="open"
        label="Reopen slot"
        said={`Reopens ${name}: a new lease can take it again.`}
        Glyph={DoorOpen}
        tone="neutral"
        waiting={waiting}
        onPress={() => onAct("open", slot.slot)}
      />
    ) : (
      <Act
        key="close"
        label="Close slot"
        said={`Closes ${name}: no new lease will take it. Its holder keeps it until its lease ends.`}
        Glyph={DoorClosedLocked}
        tone="neutral"
        waiting={waiting}
        onPress={() => onAct("close", slot.slot)}
      />
    ),
    onAct !== undefined && removable && slot.held.state !== "busy" ? (
      <Act
        key="remove"
        label="Remove slot"
        said={`Removes ${name} from the pool with git worktree remove on .armada/slots/${name}. Refused while it holds uncommitted changes.`}
        Glyph={Trash2}
        tone="destructive"
        waiting={waiting}
        onPress={() => onAct("remove", slot.slot)}
      />
    ) : null,
  ].filter((one) => one !== null);
  if (acts.length === 0) return null;
  return (
    <div className="armada-tile-sheet__acts" role="group" aria-label="Acts">
      {acts}
    </div>
  );
}
