import {
  Box,
  DoorClosedLocked,
  DoorOpen,
  Flame,
  FolderPlus,
  FolderX,
  Ghost,
  Anchor,
  KeyRound,
  LifeBuoy,
  LoaderCircle,
  Package,
  Power,
  Snowflake,
  SquareDashed,
  Trash2,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useState } from "react";
import type { RescueAct, SlotAct, WorktreeSlot } from "@armada/protocol";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { ScrapConfirm, SlotFinding } from "./SlotFinding";

/**
 * A repository's worktree pool as a grid of bays, one per slot, styled by
 * availability: a held bay is a filled card under a band in the leased hue, a
 * free one an open dashed outline, a stranded one hatched in the warning hue,
 * and a slot not made a faint ghost. A slot a person closed is shuttered in the
 * closed hue; a held one closed keeps its holder and takes the closed mark.
 * `armada worktree --status` is the same reading. Marks are group `Worktree
 * slot` in `packages/icons/icons.toml`.
 *
 * A stranded bay can be rescued: a Scout reads it, and the bay opens across the
 * grid to its Finding, where the owner Scraps or Stashes it. Pick up is not here.
 */
export type PoolSlotsProps = {
  rows: readonly PoolSlotRow[];
  /** Open the Job holding a slot. */
  onOpenJob: (jobId: string) => void;
  /**
   * Change the pool: add a slot (no `slot`), or remove, close or reopen one.
   * Absent draws no acts and no add tile.
   */
  onAct?: (act: SlotAct, slot?: number) => void;
  /**
   * Rescue a stranded slot: start or stop its Scout, or Scrap or Stash what it
   * holds. A Scrap is sent only from its confirm. Absent draws none of the acts.
   */
  onRescue?: (act: RescueAct, slot: number) => void;
  /** Why the last add was refused, drawn on the add tile. */
  addRefused?: string;
  /** An add is out, so the tile waits. */
  adding?: boolean;
};

export type PoolSlotRow = {
  slot: WorktreeSlot;
  /** How long its holder has had it, formatted by the caller's clock. */
  heldFor?: string;
  /** Why the last act on this slot was refused, said on the bay. */
  refused?: string;
  /** What the last rescue act did, as a bare fact on the bay: the branch a Scrap kept, the commit a Stash made. */
  said?: string;
  /** An act on this slot is out, so its acts wait. */
  acting?: boolean;
};

/** How a bay is drawn. */
type Bay = "held" | "kept" | "busy" | "stranded" | "free" | "ghost";

type State = { bay: Bay; Glyph: LucideIcon; word: string; said: string };

function stateOf(slot: WorktreeSlot): State {
  const held = slot.held;
  switch (held.state) {
    case "job":
      if (held.kept !== undefined) {
        return { bay: "kept", Glyph: Anchor, word: "Kept", said: `Kept: ${held.kept}` };
      }
      return { bay: "held", Glyph: KeyRound, word: "Held", said: "Held" };
    case "session":
      return { bay: "held", Glyph: KeyRound, word: "Held", said: "Held" };
    case "busy":
      return { bay: "busy", Glyph: LoaderCircle, word: "Busy", said: "Being taken or given back" };
    case "stranded":
      return { bay: "stranded", Glyph: Ghost, word: "Stranded", said: `Stranded: ${held.why}` };
    case "free":
      return { bay: "free", Glyph: DoorOpen, word: "Free", said: "Free" };
    case "unmade":
      return { bay: "ghost", Glyph: SquareDashed, word: "Not made yet", said: "Not made yet" };
    case "not_a_checkout":
      return { bay: "ghost", Glyph: FolderX, word: "Not a checkout", said: "Not a checkout" };
  }
}

/** Warm or cold, with its word where `worded`, and named on hover. */
function Warmth({ warm, worded }: { warm: boolean; worded: boolean }) {
  const Glyph = warm ? Flame : Snowflake;
  const said = warm ? "Warm" : "Cold";
  return (
    <Tooltip label={said}>
      <span
        className="armada-bay__warmth"
        data-warm={warm ? "" : undefined}
        role="img"
        aria-label={said}
      >
        <Glyph size={12} strokeWidth={2} aria-hidden />
        {worded ? <span aria-hidden>{said.toLowerCase()}</span> : null}
      </span>
    </Tooltip>
  );
}

/** A bare figure, with what it measures said whole in its tooltip and its accessible name. */
function Figure({ shown, said }: { shown: string; said: string }) {
  return (
    <Tooltip label={said}>
      <span className="armada-bay__figure" aria-label={said}>
        {shown}
      </span>
    </Tooltip>
  );
}

function Holder({ slot, onOpenJob }: { slot: WorktreeSlot; onOpenJob: (jobId: string) => void }) {
  const held = slot.held;
  if (held.state === "job") {
    return (
      <>
        <button type="button" className="armada-bay__job" onClick={() => onOpenJob(held.job_id)}>
          <Box size={12} strokeWidth={2} aria-hidden />
          <span>{held.job_title ?? held.job_id}</span>
        </button>
        {held.kept === undefined ? null : <span className="armada-bay__why">{held.kept}</span>}
      </>
    );
  }
  if (held.state === "session") return <span className="armada-bay__session">{held.holder}</span>;
  if (held.state === "stranded") return <span className="armada-bay__why">{held.why}</span>;
  return null;
}

/** One act on a bay: a glyph, named by its tooltip and its accessible name. */
function Act({
  said,
  Glyph,
  waiting,
  onPress,
}: {
  said: string;
  Glyph: LucideIcon;
  waiting: boolean;
  onPress: () => void;
}) {
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
function rescuable(slot: WorktreeSlot): boolean {
  return slot.held.state === "stranded" || (slot.held.state === "job" && slot.held.kept !== undefined);
}

/** What a stranded bay offers by where its rescue is: Rescue, Stop, or Scrap and Stash. */
function RescueActs({
  slot,
  waiting,
  onRescue,
  onScrap,
}: {
  slot: WorktreeSlot;
  waiting: boolean;
  onRescue: (act: RescueAct, slot: number) => void;
  /** Open the Scrap's confirm. Absent while it is open, so its own Scrap is the only one. */
  onScrap: (() => void) | undefined;
}) {
  const rescue = slot.rescue;
  if (rescue === undefined) {
    return <Act said="Rescue" Glyph={LifeBuoy} waiting={waiting} onPress={() => onRescue("start", slot.slot)} />;
  }
  if (rescue.state === "reading") {
    return <Act said="Stop" Glyph={Power} waiting={waiting} onPress={() => onRescue("stop", slot.slot)} />;
  }
  if (onScrap === undefined) return null;
  return (
    <>
      <Act said="Scrap" Glyph={Trash2} waiting={waiting} onPress={onScrap} />
      <Act said="Stash" Glyph={Package} waiting={waiting} onPress={() => onRescue("stash", slot.slot)} />
    </>
  );
}

/** Close or reopen on every bay, and remove where the pool would let it go. */
function Acts({
  row,
  state,
  onAct,
  onRescue,
  onScrap,
}: {
  row: PoolSlotRow;
  state: State;
  onAct: PoolSlotsProps["onAct"];
  onRescue: PoolSlotsProps["onRescue"];
  onScrap: (() => void) | undefined;
}) {
  if (onAct === undefined && onRescue === undefined) return null;
  const { slot } = row;
  const waiting = row.acting === true;
  const removable = slot.held.state === "free" || slot.held.state === "unmade";
  return (
    <span className="armada-bay__acts">
      {onRescue === undefined || !rescuable(slot) ? null : (
        <RescueActs slot={slot} waiting={waiting} onRescue={onRescue} onScrap={onScrap} />
      )}
      {onAct === undefined ? null : slot.closed === true ? (
        <Act said="Reopen" Glyph={DoorOpen} waiting={waiting} onPress={() => onAct("open", slot.slot)} />
      ) : (
        <Act said="Close" Glyph={DoorClosedLocked} waiting={waiting} onPress={() => onAct("close", slot.slot)} />
      )}
      {onAct !== undefined && removable && state.bay !== "busy" ? (
        <Act said="Remove" Glyph={Trash2} waiting={waiting} onPress={() => onAct("remove", slot.slot)} />
      ) : null}
    </span>
  );
}

/** The closed mark, beside a held bay's own state. */
function ClosedMark() {
  return (
    <Tooltip label="Closed">
      <span className="armada-bay__closed" role="img" aria-label="Closed">
        <DoorClosedLocked size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

function Refused({ said }: { said: string | undefined }) {
  return said === undefined ? null : (
    <p className="armada-bay__refused" role="alert">
      {said}
    </p>
  );
}

function Said({ said }: { said: string | undefined }) {
  return said === undefined ? null : (
    <p className="armada-bay__said" role="status">
      {said}
    </p>
  );
}

function BayTile({
  row,
  onOpenJob,
  onAct,
  onRescue,
}: {
  row: PoolSlotRow;
  onOpenJob: (jobId: string) => void;
  onAct: PoolSlotsProps["onAct"];
  onRescue: PoolSlotsProps["onRescue"];
}) {
  const { slot, heldFor } = row;
  /** The Scrap's confirm is open on this bay. */
  const [scrapping, setScrapping] = useState(false);
  const rescue = rescuable(slot) ? slot.rescue : undefined;
  const scrappable = rescue !== undefined && rescue.state !== "reading";
  const acts = (state: State) => (
    <Acts
      row={row}
      state={state}
      onAct={onAct}
      onRescue={onRescue}
      onScrap={scrappable && !scrapping ? () => setScrapping(true) : undefined}
    />
  );
  const state = stateOf(slot);
  const closed = slot.closed === true;
  const name = `slot-${slot.slot}`;
  const mark = (
    <span className="armada-bay__state" role="img" aria-label={state.said}>
      <state.Glyph size={12} strokeWidth={2} aria-hidden />
    </span>
  );
  const named = (
    <Tooltip label={slot.path}>
      <span className="armada-bay__name">{name}</span>
    </Tooltip>
  );

  if (state.bay === "free" || state.bay === "ghost") {
    // Closed, an open bay is shuttered: the shut door is its state, and what
    // it would be open is the tooltip's.
    return (
      <li className="armada-bay" data-bay={closed ? "closed" : state.bay} aria-label={name} aria-busy={row.acting || undefined}>
        <div className="armada-bay__top">
          {named}
          {acts(state)}
        </div>
        <div className="armada-bay__open">
          {closed ? (
            <Tooltip label={state.said}>
              <span className="armada-bay__word">
                <span className="armada-bay__state" role="img" aria-label="Closed">
                  <DoorClosedLocked size={12} strokeWidth={2} aria-hidden />
                </span>
                <span aria-hidden>Closed</span>
              </span>
            </Tooltip>
          ) : (
            <span className="armada-bay__word">
              {mark}
              <span aria-hidden>{state.word}</span>
            </span>
          )}
          {state.bay === "free" && !closed ? <Warmth warm={slot.warm} worded /> : null}
        </div>
        <Refused said={row.refused} />
        <Said said={row.said} />
      </li>
    );
  }

  return (
    <li
      className="armada-bay"
      data-bay={state.bay}
      data-closed={closed ? "" : undefined}
      data-open={rescue === undefined ? undefined : ""}
      aria-label={name}
      aria-busy={row.acting || undefined}
    >
      <div className="armada-bay__band">
        <Tooltip label={state.said}>{mark}</Tooltip>
        {closed ? <ClosedMark /> : null}
        <span className="armada-bay__eyebrow" aria-hidden>
          {state.word}
        </span>
        {named}
        {acts(state)}
      </div>
      <div className="armada-bay__body">
        <Holder slot={slot} onOpenJob={onOpenJob} />
        {slot.branch === undefined ? null : (
          <Tooltip label={slot.branch}>
            <span className="armada-bay__branch">{slot.branch}</span>
          </Tooltip>
        )}
        <SlotFinding slot={slot} />
        {scrapping && scrappable && onRescue !== undefined ? (
          <ScrapConfirm
            slot={slot}
            onScrap={() => {
              setScrapping(false);
              onRescue("scrap", slot.slot);
            }}
            onCancel={() => setScrapping(false)}
          />
        ) : null}
        <Refused said={row.refused} />
        <Said said={row.said} />
        <div className="armada-bay__foot">
          {heldFor === undefined ? null : <Figure shown={heldFor} said={`Held for ${heldFor}`} />}
          {slot.behind === undefined ? null : (
            <Figure
              shown={`${slot.behind} behind`}
              said={`${slot.behind} ${slot.behind === 1 ? "commit" : "commits"} behind ${slot.base}`}
            />
          )}
          <Warmth warm={slot.warm} worded={false} />
        </div>
      </div>
    </li>
  );
}

/** The ghost tile after the last bay: one more slot, made by the next lease. */
function AddTile({ onAct, refused, adding }: { onAct: NonNullable<PoolSlotsProps["onAct"]>; refused?: string; adding?: boolean }) {
  return (
    <li className="armada-bay" data-bay="add" aria-busy={adding || undefined}>
      <Tooltip label="Add a slot">
        <button
          type="button"
          className="armada-bay__add"
          aria-label="Add a slot"
          aria-disabled={adding || undefined}
          onClick={adding ? undefined : () => onAct("add")}
        >
          <FolderPlus size={16} strokeWidth={2} aria-hidden />
        </button>
      </Tooltip>
      <Refused said={refused} />
    </li>
  );
}

export function PoolSlots({ rows, onOpenJob, onAct, onRescue, addRefused, adding }: PoolSlotsProps) {
  return (
    <ul className="armada-pool-slots" aria-label="Worktree slots">
      {rows.map((row) => (
        <BayTile
          key={`${row.slot.manifest_id}/${row.slot.slot}`}
          row={row}
          onOpenJob={onOpenJob}
          onAct={onAct}
          onRescue={onRescue}
        />
      ))}
      {onAct === undefined ? null : <AddTile onAct={onAct} refused={addRefused} adding={adding} />}
    </ul>
  );
}
