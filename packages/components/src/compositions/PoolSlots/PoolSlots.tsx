import {
  Box,
  DoorClosedLocked,
  DoorOpen,
  Flame,
  FolderPlus,
  FolderX,
  Ghost,
  KeyRound,
  LoaderCircle,
  Snowflake,
  SquareDashed,
  Trash2,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { SlotAct, WorktreeSlot } from "@armada/protocol";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * A repository's worktree pool as a grid of bays, one per slot, styled by
 * availability: a held bay is a filled card under a band in the leased hue, a
 * free one an open dashed outline, a stranded one hatched in the warning hue,
 * and a slot not made a faint ghost. A slot a person closed is shuttered in the
 * closed hue; a held one closed keeps its holder and takes the closed mark.
 * `armada worktree --status` is the same reading. Marks are group `Worktree
 * slot` in `packages/icons/icons.toml`.
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
  /** An act on this slot is out, so its acts wait. */
  acting?: boolean;
};

/** How a bay is drawn. */
type Bay = "held" | "busy" | "stranded" | "free" | "ghost";

type State = { bay: Bay; Glyph: LucideIcon; word: string; said: string };

function stateOf(slot: WorktreeSlot): State {
  const held = slot.held;
  switch (held.state) {
    case "job":
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

/** A bare figure, named by its tooltip and its accessible name. */
function Figure({ shown, value, named }: { shown: string; value: string; named: string }) {
  return (
    <Tooltip label={named}>
      <span className="armada-bay__figure" aria-label={`${named}: ${value}`}>
        {shown}
      </span>
    </Tooltip>
  );
}

function Holder({ slot, onOpenJob }: { slot: WorktreeSlot; onOpenJob: (jobId: string) => void }) {
  const held = slot.held;
  if (held.state === "job") {
    return (
      <button type="button" className="armada-bay__job" onClick={() => onOpenJob(held.job_id)}>
        <Box size={12} strokeWidth={2} aria-hidden />
        <span>{held.job_title ?? held.job_id}</span>
      </button>
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

/** Close or reopen on every bay, and remove where the pool would let it go. */
function Acts({ row, state, onAct }: { row: PoolSlotRow; state: State; onAct: PoolSlotsProps["onAct"] }) {
  if (onAct === undefined) return null;
  const { slot } = row;
  const waiting = row.acting === true;
  const removable = slot.held.state === "free" || slot.held.state === "unmade";
  return (
    <span className="armada-bay__acts">
      {slot.closed === true ? (
        <Act said="Reopen" Glyph={DoorOpen} waiting={waiting} onPress={() => onAct("open", slot.slot)} />
      ) : (
        <Act said="Close" Glyph={DoorClosedLocked} waiting={waiting} onPress={() => onAct("close", slot.slot)} />
      )}
      {removable && state.bay !== "busy" ? (
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

function BayTile({
  row,
  onOpenJob,
  onAct,
}: {
  row: PoolSlotRow;
  onOpenJob: (jobId: string) => void;
  onAct: PoolSlotsProps["onAct"];
}) {
  const { slot, heldFor } = row;
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
          <Acts row={row} state={state} onAct={onAct} />
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
      </li>
    );
  }

  return (
    <li
      className="armada-bay"
      data-bay={state.bay}
      data-closed={closed ? "" : undefined}
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
        <Acts row={row} state={state} onAct={onAct} />
      </div>
      <div className="armada-bay__body">
        <Holder slot={slot} onOpenJob={onOpenJob} />
        {slot.branch === undefined ? null : (
          <Tooltip label={slot.branch}>
            <span className="armada-bay__branch">{slot.branch}</span>
          </Tooltip>
        )}
        <Refused said={row.refused} />
        <div className="armada-bay__foot">
          {heldFor === undefined ? null : <Figure shown={heldFor} value={heldFor} named="Held for" />}
          {slot.behind === undefined ? null : (
            <Figure
              shown={`${slot.behind} behind`}
              value={String(slot.behind)}
              named={`Commits behind ${slot.base}`}
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

export function PoolSlots({ rows, onOpenJob, onAct, addRefused, adding }: PoolSlotsProps) {
  return (
    <ul className="armada-pool-slots" aria-label="Worktree slots">
      {rows.map((row) => (
        <BayTile key={`${row.slot.manifest_id}/${row.slot.slot}`} row={row} onOpenJob={onOpenJob} onAct={onAct} />
      ))}
      {onAct === undefined ? null : <AddTile onAct={onAct} refused={addRefused} adding={adding} />}
    </ul>
  );
}
