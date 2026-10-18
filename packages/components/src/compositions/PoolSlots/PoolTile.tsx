import { ScanSearch, DoorClosedLocked, DoorOpen, Flame, FolderX, Ghost, KeyRound, LoaderCircle, Mail, Snowflake, SquareDashed, SquareTerminal, Unplug } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useContext } from "react";
import type { WorktreeSlot } from "@armada/protocol";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { ChipOwnership, OwnerChip } from "../OwnerChip/OwnerChip";
import { Mark } from "./SlotFinding";
import { keyOf, nameOf } from "./tiles";
import type { TileRow } from "./tiles";

/** How a tile is drawn. */
export type Bay = "held" | "busy" | "stranded" | "free" | "ghost";

export type State = { bay: Bay; Glyph: LucideIcon; word: string; said: string };

function slotState(slot: WorktreeSlot): State {
  const held = slot.held;
  switch (held.state) {
    case "session":
    case "hosted_session":
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

export const stateOf = (row: TileRow): State => slotState(row.slot);

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

function Holder({ row }: { row: TileRow }) {
  const held = row.slot.held;
  if (held.state === "session") return <span className="armada-bay__session">{held.session_title ?? held.holder}</span>;
  if (held.state === "hosted_session" && held.session_title !== undefined) return <span className="armada-bay__session">{held.session_title}</span>;
  if (held.state === "stranded") return <span className="armada-bay__why">{held.why}</span>;
  return null;
}

/** A time of day as the limit mark says it: `10:42 AM`, in local time. */
const CLOCK = new Intl.DateTimeFormat("en-US", { hour: "numeric", minute: "2-digit" });
const clock = (instant: string) => CLOCK.format(new Date(instant));

/**
 * Who held a stranded slot last, under its branch: the mark of what it was and
 * its title, a link where it can still be opened, then what Fleet did about it.
 * A Session opens only while it is open.
 */
function LastHolder({ slot }: { slot: WorktreeSlot }) {
  const ownership = useContext(ChipOwnership);
  const who = slot.stranded?.last_holder;
  if (who === undefined) return null;
  const opens = who.open && ownership !== null ? () => ownership.open(who.id) : undefined;
  return (
    <div className="armada-bay__holder">
      {who.title === undefined ? (
        <Mark said="Job">
          <SquareTerminal size={12} strokeWidth={2} aria-hidden />
        </Mark>
      ) : opens === undefined ? (
        <span className="armada-bay__who">
          <SquareTerminal size={12} strokeWidth={2} aria-hidden />
          <span>{who.title}</span>
        </span>
      ) : (
        <button type="button" className="armada-bay__job" onClick={opens}>
          <SquareTerminal size={12} strokeWidth={2} aria-hidden />
          <span>{who.title}</span>
        </button>
      )}
      {slot.stranded?.told === undefined ? null : (
        <Mark said={`Told ${clock(slot.stranded.told)}`}>
          <Mail size={12} strokeWidth={2} aria-hidden />
        </Mark>
      )}
      {who.open ? null : (
        <Mark said="Job closed">
          <Unplug size={12} strokeWidth={2} aria-hidden />
        </Mark>
      )}
    </div>
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

/**
 * One tile. **The whole tile opens its panel**, and its name is the button that
 * says so to the keyboard and to a screen reader.
 */
export function PoolTile({ row, open, onOpen }: { row: TileRow; open: boolean; onOpen: () => void }) {
  const { slot, heldFor } = row;
  const name = nameOf(row);
  const state = stateOf(row);
  const closed = slot.closed === true;
  const mark = (
    <span className="armada-bay__state" role="img" aria-label={state.said}>
      <state.Glyph size={12} strokeWidth={2} aria-hidden />
    </span>
  );
  const named = (
    <Tooltip label={slot.path}>
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
  const common = { "data-tile": keyOf(row), "aria-label": name, "aria-busy": row.acting || undefined, "data-open": open || undefined, onClick: press };

  if (state.bay === "free" || state.bay === "ghost") {
    // Closed, an open bay is shuttered: the shut door is its state, and what
    // it would be open is the tooltip's.
    return (
      <li className="armada-bay" data-bay={closed ? "closed" : state.bay} {...common}>
        <div className="armada-bay__top">
          {named}
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
        <Holder row={row} />
        {slot.branch === undefined ? null : (
          // A Session that owns the branch is named on hover; where none does, the tooltip is what it was.
          <OwnerChip
            chip={{ kind: "branch", name: slot.branch }}
            plain={
              <Tooltip label={slot.branch}>
                <span className="armada-bay__branch">{slot.branch}</span>
              </Tooltip>
            }
          >
            <span className="armada-bay__branch">{slot.branch}</span>
          </OwnerChip>
        )}
        <LastHolder slot={slot} />
        <div className="armada-bay__foot">
          {slot.rescue?.state === "answered" ? (
            <Mark said="Finding ready">
              <ScanSearch size={12} strokeWidth={2} aria-hidden />
            </Mark>
          ) : null}
          {slot.rescue?.state === "reading" ? (
            <Mark said="Reading">
              <ScanSearch className="armada-finding__reading" size={12} strokeWidth={2} aria-hidden />
            </Mark>
          ) : null}
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
