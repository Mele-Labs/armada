// The palette's dispatch row: what was typed in the title bar's one bar, offered as the work itself
// (the owner, 10 Oct 2026).

import { ChevronDown, Plus, type LucideIcon } from "lucide-react";

import { KbdChord } from "../Kbd/Kbd";
import { Tooltip } from "../Tooltip/Tooltip";
import { ActionsKey } from "./actions-panel";
import type { PaletteChipOption } from "./chips";
import type { PaletteEntry, PaletteSection } from "./CommandPalette";

export type { PaletteChipOption } from "./chips";

export type DispatchKind = "session" | "autopilot";

/** What the one bar files in his projects rather than dispatches: a task, or a project. */
export type AddKind = "task" | "project";

/** Every kind Tab steps through on the one bar. */
export type BarKind = DispatchKind | AddKind;

/** Whether the kind adds to his projects, so Enter adds whatever the words match. */
export const adds = (kind: BarKind): kind is AddKind => kind === "task" || kind === "project";

/**
 * The palette's second step: the places the words can be filed, listed in the palette itself. Choosing one files them
 * there and closes the palette; Esc backs out to the words.
 */
export type PaletteStep = {
  /** What is being filed, said over the list while the step is up. */
  title: string;
  /** The command the step is asking for, drawn at the start of the field with the title as its placeholder, so the person sees where a key or a chain has put them. */
  of?: string;
  sections: readonly PaletteSection[];
  entries: readonly PaletteEntry[];
  /** What was searched, for the state where nothing matched. */
  searched: string;
  /** Filing the choice closes the palette. A step returned is asked next, in the palette, with the choice not yet filed. */
  onChoose: (entry: PaletteEntry) => PaletteStep | void;
  /** What ↵ does here, said in the footer: Next while more is asked, else the act's verb. Absent says Add. */
  act?: string;
  /** Takes what is typed: a row for the typed word is listed, with the id `word:` and the word as its label. */
  word?: true;
};

/**
 * A choice the dispatch row carries after its words, drawn as a chip, so where the words go is read before ↵ files them. Tab
 * opens its list in the palette; a marker typed before a name (`@armada`) sets it from the words, and the token leaves the words.
 */
export type PaletteChip = {
  id: string;
  /** What it chooses, in its tooltip and its name: Repository, Project. */
  name: string;
  /** Its glyph, from the registry. */
  icon?: LucideIcon;
  /** Its choices, in the order its list draws them. One alone is fixed: drawn, with no list. */
  options: readonly PaletteChipOption[];
  /** The option chosen, by id. */
  chosen: string;
  /** Typed before a name, sets the chip from the words. Absent, only its list does. */
  marker?: string;
  /** Chosen from its list. */
  onChoose: (id: string) => void;
};

export type PaletteDispatch = {
  kind: BarKind;
  /** Tab switches the kind. Absent while there is no other kind to offer, and the row stays a Session's. */
  onKind?: (kind: BarKind) => void;
  /** The kinds Tab steps through, in order: a Session where absent. Autopilot is one when the host serves it (#2092), Task and Project when it keeps projects. */
  kinds?: readonly BarKind[];
  /** `chosen` is each chip's option, by chip id, with the tokens read; `words` are without them. */
  onDispatch: (words: string, kind: BarKind, chosen?: Readonly<Record<string, string>>) => void;
  /** What the row carries for the kind it is on, as chips after the words. Absent or empty, none. */
  chips?: readonly PaletteChip[];
  /** Where the words need a place chosen first, the step that lists the places. `undefined` sends them at once. */
  step?: (words: string, kind: BarKind) => PaletteStep | undefined;
};

const KIND: Record<BarKind, string> = { session: "Job", autopilot: "Autopilot", task: "Task", project: "Project" };

/** What the row offers to do with the words, for each kind. */
const OFFER: Record<BarKind, string> = { session: "Start a Job with ", autopilot: "Start Autopilot with ", task: "Add a task ", project: "Add a project " };

/** The kind Tab steps to from this one, round the kinds offered. */
export function nextKind(dispatch: PaletteDispatch): BarKind {
  const kinds = dispatch.kinds ?? ["session"];
  return kinds[(kinds.indexOf(dispatch.kind) + 1) % kinds.length] ?? "session";
}

/**
 * Words that read as work rather than a search: a link to a ticket. A sentence is not enough, since
 * a Session is searched by its title and a title is a sentence — "pin the store" opens that Session.
 */
export function readsAsWork(query: string): boolean {
  return /^https?:\/\//.test(query.trim());
}

export const DISPATCH_ROW = "armada-palette-row-dispatch";

/** A chip as the row draws it: its choice's label, and whether its list is up. */
export type DrawnChip = { chip: PaletteChip; label: string; open: boolean };

/** What was typed, as the row that dispatches it. */
export function DispatchRow({
  words,
  kind,
  chips = [],
  active,
  trailing,
  ref,
  onEnter,
  onChoose,
  onChip,
}: {
  words: string;
  kind: BarKind;
  chips?: readonly DrawnChip[];
  /** A chip with a list was pressed: its list opens, and nothing is filed. */
  onChip?: (id: string) => void;
  active: boolean;
  /** At the foot of the matches, where it stays pinned in view however far the list scrolls. */
  trailing: boolean;
  ref?: React.Ref<HTMLDivElement> | undefined;
  onEnter: () => void;
  onChoose: () => void;
}) {
  return (
    <div
      ref={ref}
      id={DISPATCH_ROW}
      role="option"
      aria-selected={active}
      className={["armada-palette__row", "armada-palette__row--dispatch", trailing ? "armada-palette__row--trailing" : "", active ? "armada-palette__row--active" : ""].filter((one) => one !== "").join(" ")}
      onMouseEnter={onEnter}
      onClick={onChoose}
    >
      <span className="armada-palette__glyph">
        <Plus size={16} strokeWidth={2} aria-hidden />
      </span>
      <span className="armada-palette__label">
        <span className="armada-palette__title">
          {OFFER[kind]}
          <span className="armada-palette__words">“{words}”</span>
        </span>
        {chips.length === 0 ? null : (
          <span className="armada-palette__chips">
            {chips.map(({ chip, label, open }) => (
              <Chip key={chip.id} chip={chip} label={label} open={open} onOpen={() => onChip?.(chip.id)} />
            ))}
          </span>
        )}
      </span>
      {/* The result rows' kind column, so its kind ends on their edge. Its key, ⌘↵, is on the foot. */}
      <span className="armada-palette__kind">{KIND[kind]}</span>
    </div>
  );
}

/** One chip: its glyph and its choice. One with a list opens it on a press, and the focus stays in the words. */
function Chip({ chip, label, open, onOpen }: { chip: PaletteChip; label: string; open: boolean; onOpen: () => void }) {
  const Glyph = chip.icon;
  const said = `${chip.name} ${label}`;
  const inner = (
    <>
      {Glyph === undefined ? null : <Glyph size={12} strokeWidth={2} aria-hidden />}
      <span className="armada-palette__chip-name">{label}</span>
    </>
  );
  if (chip.options.length < 2) {
    return (
      <Tooltip label={chip.name}>
        <span className="armada-palette__chip" role="img" aria-label={said}>
          {inner}
        </span>
      </Tooltip>
    );
  }
  return (
    <Tooltip label={chip.name} shortcut="⇥">
      <button
        type="button"
        tabIndex={-1}
        className="armada-palette__chip"
        data-open={open || undefined}
        aria-label={said}
        aria-haspopup="listbox"
        aria-expanded={open}
        onMouseDown={(event) => event.preventDefault()}
        onClick={(event) => {
          event.stopPropagation();
          onOpen();
        }}
      >
        {inner}
        <ChevronDown size={12} strokeWidth={2} aria-hidden />
      </button>
    </Tooltip>
  );
}

/** The keys the one bar answers, along the palette's foot while there is something typed to send. */
export function DispatchKeys({ dispatch, sends, acts }: { dispatch: PaletteDispatch; sends: boolean; acts: boolean }) {
  const verb = adds(dispatch.kind) ? "Add" : "Dispatch";
  return (
    <div className="armada-palette__keys-foot" aria-hidden>
      <span>{sends ? verb : "Open"} <KbdChord keys={["↵"]} /></span>
      <span>{verb} <KbdChord keys={["⌘", "↵"]} /></span>
      {listed(dispatch)[0] !== undefined ? (
        <span>{listed(dispatch)[0]!.name} <KbdChord keys={["⇥"]} /></span>
      ) : dispatch.onKind === undefined ? null : (
        <span>{KIND[nextKind(dispatch)]} <KbdChord keys={["⇥"]} /></span>
      )}
      {acts ? <ActionsKey /> : null}
    </div>
  );
}

/** The chips Tab steps through: those with a list. */
export const listed = (dispatch: PaletteDispatch): PaletteChip[] => (dispatch.chips ?? []).filter((one) => one.options.length > 1);

/** The kind Shift+Tab steps back to, round the kinds offered. */
export function previousKind(dispatch: PaletteDispatch): BarKind {
  const kinds = dispatch.kinds ?? ["session"];
  return kinds[(kinds.indexOf(dispatch.kind) - 1 + kinds.length) % kinds.length] ?? "session";
}

/** The keys a chip's list answers, or a typed token's completions, along the palette's foot. */
export function ChoiceKeys({ open }: { open: boolean }) {
  return (
    <div className="armada-palette__keys-foot" aria-hidden>
      <span>Choose <KbdChord keys={["↵"]} /></span>
      {open ? <span>Next <KbdChord keys={["⇥"]} /></span> : <span>Choose <KbdChord keys={["⇥"]} /></span>}
      {open ? <span>Back <KbdChord keys={["Esc"]} /></span> : null}
    </div>
  );
}

/** The keys a second step answers, along the palette's foot. */
export function StepKeys({ act = "Add" }: { act?: string }) {
  return (
    <div className="armada-palette__keys-foot" aria-hidden>
      <span>{act} <KbdChord keys={["↵"]} /></span>
      <span>Back <KbdChord keys={["Esc"]} /></span>
    </div>
  );
}
