import { CirclePause, DoorClosedLocked, DoorOpen, Eraser, GitBranchMinus, LifeBuoy, PackageX, Play, Power, Trash2, Unplug } from "lucide-react";
import type { LucideIcon } from "lucide-react";
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

/** A stranded slot, or a Job's that ended and kept its slot: both hold work a person can rescue. */
export function rescuable(slot: WorktreeSlot): boolean {
  return slot.held.state === "stranded" || (slot.held.state === "job" && slot.held.kept !== undefined);
}

/** Which confirm an act opens in the panel, before it is sent. */
export type Confirming = "clear" | "branch" | "forget" | "release" | "pause";

export type TileActsProps = {
  row: TileRow;
  /** An act on the tile is out, or a confirm is open: the acts wait. */
  waiting: boolean;
  onAct?: ((act: SlotAct, slot?: number) => void) | undefined;
  onRescue?: ((act: RescueAct, slot: number) => void) | undefined;
  onConfirm?: ((which: Confirming) => void) | undefined;
  /** Resume a paused Job. Sent at once: nothing is lost by it. */
  onResume?: (() => void) | undefined;
};

/**
 * What each reclaim act does, from `reclaim_worktree`, `delete_branch` and
 * `forget_job`. **A bay is released to the pool**: its worktree stays and HEAD
 * is detached. A worktree outside the pool is removed with `git worktree
 * remove`. **Uncommitted files are committed to the branch first, either way**,
 * and the branch is kept. With none, the branch is deleted only where the base
 * has all its commits. Delete branch is refused while the worktree is on disk.
 */
export function tipsOf(row: TileRow): Record<"clear" | "branch" | "forget" | "pause", string> {
  const { slot, held } = row;
  const name = nameOf(row);
  const branch = row.cost?.branch?.name ?? held?.branch ?? slot?.branch ?? "its branch";
  const base = row.cost?.branch?.base ?? slot?.base ?? "main";
  const tip = row.cost?.branch?.tip;
  const keeps = `Deletes branch ${branch} only if ${base} has all its commits, otherwise keeps it.`;
  const saves = (row.cost?.files.length ?? 0) > 0;
  const saved = `Commits the uncommitted files to branch ${branch} as a WIP commit`;
  return {
    pause: `${row.held?.status === "running" ? "Stops the Drone and its processes. " : ""}${saves ? `${saved}, then releases` : "Releases"} ${name}. The work stays on branch ${branch}.`,
    clear: saves
      ? slot === undefined
        ? `${saved}, then removes the worktree at ${held?.path ?? name}. Keeps branch ${branch}.`
        : `${saved}, then releases ${name} to the pool. Keeps branch ${branch}.`
      : slot === undefined
        ? `Removes the worktree at ${held?.path ?? name}. ${keeps}`
        : `Releases ${name} to the pool: detaches its worktree at .armada/slots/${name} from ${branch} and keeps the directory. ${keeps}`,
    branch: `Deletes branch ${branch}${tip === undefined ? "" : ` at ${tip}`}. Its commits not on ${base} stay reachable only from that commit.`,
    forget: `Deletes the record of Job ${row.job ?? held?.job_title ?? ""}. The worktree and branch are not touched.`,
  };
}

/**
 * The acts a tile takes, each only where it applies. Slot acts come from the
 * bay: Rescue or Stop, Close slot or Reopen slot, and Remove slot where the
 * pool would let it go. Worktree acts come from the Job's reading: Clear,
 * Delete branch and Forget Job, each opening its confirm. **Room is left after
 * them** for acts that land later, which are the screen's to offer and not this
 * row's to guess.
 */
export function TileActs({ row, waiting, onAct, onRescue, onConfirm, onResume }: TileActsProps) {
  const { slot, offered } = row;
  const name = nameOf(row);
  const tips = tipsOf(row);
  const removable = slot !== undefined && (slot.held.state === "free" || slot.held.state === "unmade");
  const rescue = slot?.rescue;
  const base = slot?.base ?? "main";
  const acts = [
    onRescue !== undefined && slot !== undefined && rescuable(slot) && rescue === undefined ? (
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
    onRescue !== undefined && slot !== undefined && rescuable(slot) && rescue?.state === "reading" ? (
      <Act key="stop" label="Stop" said={`Stops the Scout reading ${name}.`} Glyph={Power} tone="neutral" waiting={waiting} onPress={() => onRescue("stop", slot.slot)} />
    ) : null,
    onConfirm !== undefined && slot?.held.state === "session" ? (
      <Act
        key="release"
        label="Release"
        said={`Commits any uncommitted files in ${name} to ${slot.branch ?? "its branch"} as a WIP commit, then releases the slot. Keeps the branch.`}
        Glyph={Unplug}
        tone="neutral"
        waiting={waiting}
        onPress={() => onConfirm("release")}
      />
    ) : null,
    onConfirm !== undefined && offered?.pause === true ? (
      <Act key="pause" label="Pause" said={tips.pause} Glyph={CirclePause} tone="neutral" waiting={waiting} onPress={() => onConfirm("pause")} />
    ) : null,
    onResume !== undefined && offered?.resume === true ? (
      <Act
        key="resume"
        label="Resume"
        said={`Puts branch ${row.held?.branch ?? slot?.branch ?? "its branch"} back in a slot. The Job is not started.`}
        Glyph={Play}
        tone="neutral"
        waiting={waiting}
        onPress={onResume}
      />
    ) : null,
    onConfirm !== undefined && offered?.clear === true ? (
      <Act key="clear" label="Clear" said={tips.clear} Glyph={Eraser} tone="destructive" waiting={waiting} onPress={() => onConfirm("clear")} />
    ) : null,
    onConfirm !== undefined && offered?.deleteBranch === true ? (
      <Act key="branch" label="Delete branch" said={tips.branch} Glyph={GitBranchMinus} tone="destructive" waiting={waiting} onPress={() => onConfirm("branch")} />
    ) : null,
    onConfirm !== undefined && offered?.forget === true ? (
      <Act key="forget" label="Forget Job" said={tips.forget} Glyph={PackageX} tone="destructive" waiting={waiting} onPress={() => onConfirm("forget")} />
    ) : null,
    onAct === undefined || slot === undefined ? null : slot.closed === true ? (
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
    onAct !== undefined && slot !== undefined && removable && slot.held.state !== "busy" ? (
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
