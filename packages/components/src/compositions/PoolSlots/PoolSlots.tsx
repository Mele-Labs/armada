import { Box, DoorOpen, Flame, FolderX, Ghost, KeyRound, LoaderCircle, Snowflake, SquareDashed } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { WorktreeSlot } from "@armada/protocol";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * A repository's worktree pool as a grid of bays, one per slot, styled by
 * availability: a held bay is a filled card under a band in the leased hue, a
 * free one an open dashed outline, a stranded one hatched in the warning hue,
 * and a slot not made a faint ghost. `armada worktree --status` is the same
 * reading. Marks are group `Worktree slot` in `packages/icons/icons.toml`.
 */
export type PoolSlotsProps = {
  rows: readonly PoolSlotRow[];
  /** Open the Job holding a slot. */
  onOpenJob: (jobId: string) => void;
};

export type PoolSlotRow = {
  slot: WorktreeSlot;
  /** How long its holder has had it, formatted by the caller's clock. */
  heldFor?: string;
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

function BayTile({ row, onOpenJob }: { row: PoolSlotRow; onOpenJob: (jobId: string) => void }) {
  const { slot, heldFor } = row;
  const state = stateOf(slot);
  const name = `slot-${slot.slot}`;
  const mark = (
    <span className="armada-bay__state" role="img" aria-label={state.said}>
      <state.Glyph size={12} strokeWidth={2} aria-hidden />
    </span>
  );

  if (state.bay === "free" || state.bay === "ghost") {
    return (
      <li className="armada-bay" data-bay={state.bay} aria-label={name}>
        <Tooltip label={slot.path}>
          <span className="armada-bay__name">{name}</span>
        </Tooltip>
        <div className="armada-bay__open">
          <span className="armada-bay__word">
            {mark}
            <span aria-hidden>{state.word}</span>
          </span>
          {state.bay === "free" ? <Warmth warm={slot.warm} worded /> : null}
        </div>
      </li>
    );
  }

  return (
    <li className="armada-bay" data-bay={state.bay} aria-label={name}>
      <div className="armada-bay__band">
        <Tooltip label={state.said}>{mark}</Tooltip>
        <span className="armada-bay__eyebrow" aria-hidden>
          {state.word}
        </span>
        <Tooltip label={slot.path}>
          <span className="armada-bay__name">{name}</span>
        </Tooltip>
      </div>
      <div className="armada-bay__body">
        <Holder slot={slot} onOpenJob={onOpenJob} />
        {slot.branch === undefined ? null : (
          <Tooltip label={slot.branch}>
            <span className="armada-bay__branch">{slot.branch}</span>
          </Tooltip>
        )}
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

export function PoolSlots({ rows, onOpenJob }: PoolSlotsProps) {
  return (
    <ul className="armada-pool-slots" aria-label="Worktree slots">
      {rows.map((row) => (
        <BayTile key={`${row.slot.manifest_id}/${row.slot.slot}`} row={row} onOpenJob={onOpenJob} />
      ))}
    </ul>
  );
}
