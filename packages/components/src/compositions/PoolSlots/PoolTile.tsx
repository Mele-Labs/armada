import { Anchor, Box, ScanSearch, DoorClosedLocked, DoorOpen, Flame, FolderX, Ghost, KeyRound, LoaderCircle, Snowflake, SquareDashed } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { WorktreeHeld, WorktreeSlot } from "@armada/protocol";
import { reclaimable } from "@armada/protocol";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { Mark } from "./SlotFinding";
import { holdsSaid } from "./TileHolds";
import { nameOf } from "./tiles";
import type { TileRow } from "./tiles";

/** How a tile is drawn. */
export type Bay = "held" | "kept" | "busy" | "stranded" | "free" | "ghost";

export type State = { bay: Bay; Glyph: LucideIcon; word: string; said: string };

function slotState(slot: WorktreeSlot): State {
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

/** A Job's worktree outside the pool: held while it runs, kept for what it holds, free to go, or gone. */
function outsideState(held: WorktreeHeld): State {
  if (!reclaimable(held)) return { bay: "held", Glyph: KeyRound, word: "Held", said: "Held: its Job has not finished" };
  if (held.held.length > 0) {
    return { bay: "kept", Glyph: Anchor, word: "Kept", said: `Kept: ${holdsSaid(held.held)}` };
  }
  if (!held.on_disk) return { bay: "ghost", Glyph: FolderX, word: "Gone", said: "Worktree removed" };
  return { bay: "free", Glyph: DoorOpen, word: "Free", said: "Free: nothing uncommitted or unmerged" };
}

export function stateOf(row: TileRow): State {
  if (row.slot !== undefined) return slotState(row.slot);
  return outsideState(row.held!);
}

/** Warm or cold, with its word where `worded`, and named on hover. */
function Warmth({ warm, worded }: { warm: boolean; worded: boolean }) {
  const Glyph = warm ? Flame : Snowflake;
  const said = warm ? "Warm" : "Cold";
  return (
    <Tooltip label={said}>
      <span className="armada-bay__warmth" data-warm={warm ? "" : undefined} role="img" aria-label={said}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
        {worded ? <span aria-hidden>{said.toLowerCase()}</span> : null}
      </span>
    </Tooltip>
  );
}

/** A bare figure, with what it measures said whole in its tooltip and its accessible name. */
export function Figure({ shown, said }: { shown: string; said: string }) {
  return (
    <Tooltip label={said}>
      <span className="armada-bay__figure" aria-label={said}>
        {shown}
      </span>
    </Tooltip>
  );
}

/** The Job holding a tile: its mark and its title, a link that opens it. */
export function JobLink({ jobId, title, onOpenJob }: { jobId: string; title: string; onOpenJob: (jobId: string) => void }) {
  return (
    <button type="button" className="armada-bay__job" onClick={() => onOpenJob(jobId)}>
      <Box size={12} strokeWidth={2} aria-hidden />
      <span>{title}</span>
    </button>
  );
}

/** The Job a tile belongs to, from its bay or its worktree reading. */
export function jobOf(row: TileRow): { jobId: string; title: string } | null {
  const held = row.slot?.held;
  if (held?.state === "job") return { jobId: held.job_id, title: held.job_title ?? held.job_id };
  if (row.slot === undefined && row.held !== undefined) return { jobId: row.held.job_id, title: row.held.job_title };
  return null;
}

function Holder({ row, onOpenJob }: { row: TileRow; onOpenJob: (jobId: string) => void }) {
  const job = jobOf(row);
  const held = row.slot?.held;
  if (job !== null) {
    return (
      <>
        <JobLink jobId={job.jobId} title={job.title} onOpenJob={onOpenJob} />
        {held?.state === "job" && held.kept !== undefined ? <span className="armada-bay__why">{held.kept}</span> : null}
      </>
    );
  }
  if (held?.state === "session") return <span className="armada-bay__session">{held.holder}</span>;
  if (held?.state === "stranded") return <span className="armada-bay__why">{held.why}</span>;
  return null;
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

/**
 * One tile. **The whole tile opens its panel**, and its name is the button that
 * says so to the keyboard and to a screen reader; the Job link inside it is the
 * one other thing that takes a press.
 */
export function PoolTile({
  row,
  open,
  onOpenJob,
  onOpen,
}: {
  row: TileRow;
  open: boolean;
  onOpenJob: (jobId: string) => void;
  onOpen: () => void;
}) {
  const { slot, held, heldFor, sat } = row;
  const name = nameOf(row);
  const state = stateOf(row);
  const closed = slot?.closed === true;
  const mark = (
    <span className="armada-bay__state" role="img" aria-label={state.said}>
      <state.Glyph size={12} strokeWidth={2} aria-hidden />
    </span>
  );
  const named = (
    <Tooltip label={slot?.path ?? held?.path ?? name}>
      <button type="button" className="armada-bay__name" aria-expanded={open} onClick={onOpen}>
        {name}
      </button>
    </Tooltip>
  );
  // A press anywhere on the tile opens it, except on another control inside.
  const press = (event: { target: EventTarget }) => {
    if (event.target instanceof Element && event.target.closest("button, a") !== null) return;
    onOpen();
  };
  const common = { "aria-label": name, "aria-busy": row.acting || undefined, "data-open": open || undefined, onClick: press };

  if (state.bay === "free" || state.bay === "ghost") {
    // Closed, an open bay is shuttered: the shut door is its state, and what
    // it would be open is the tooltip's.
    return (
      <li className="armada-bay" data-bay={closed ? "closed" : state.bay} {...common}>
        <div className="armada-bay__top">{named}</div>
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
          {slot !== undefined && state.bay === "free" && !closed ? <Warmth warm={slot.warm} worded /> : null}
        </div>
      </li>
    );
  }

  return (
    <li className="armada-bay" data-bay={state.bay} data-closed={closed ? "" : undefined} {...common}>
      <div className="armada-bay__band">
        <Tooltip label={state.said}>{mark}</Tooltip>
        {closed ? <ClosedMark /> : null}
        <span className="armada-bay__eyebrow" aria-hidden>
          {state.word}
        </span>
        {named}
      </div>
      <div className="armada-bay__body">
        <Holder row={row} onOpenJob={onOpenJob} />
        {(slot?.branch ?? held?.branch) === undefined ? null : (
          <Tooltip label={(slot?.branch ?? held?.branch)!}>
            <span className="armada-bay__branch">{slot?.branch ?? held?.branch}</span>
          </Tooltip>
        )}
        <div className="armada-bay__foot">
          {slot?.rescue?.state === "reading" ? (
            <Mark said="Reading">
              <ScanSearch className="armada-finding__reading" size={12} strokeWidth={2} aria-hidden />
            </Mark>
          ) : null}
          {heldFor === undefined ? null : <Figure shown={heldFor} said={`Held for ${heldFor}`} />}
          {slot === undefined && sat !== undefined ? <Figure shown={sat} said={`Last moved ${sat} ago`} /> : null}
          {slot?.behind === undefined ? null : (
            <Figure
              shown={`${slot.behind} behind`}
              said={`${slot.behind} ${slot.behind === 1 ? "commit" : "commits"} behind ${slot.base}`}
            />
          )}
          {slot === undefined ? null : <Warmth warm={slot.warm} worded={false} />}
        </div>
      </div>
    </li>
  );
}
