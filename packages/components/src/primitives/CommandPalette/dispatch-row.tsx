// The palette's dispatch row: what was typed in the title bar's one bar, offered as the work itself
// (the owner, 10 Oct 2026), and where the palette sits when it opens over that bar.

import { useLayoutEffect, useState, type CSSProperties } from "react";
import { Plus } from "lucide-react";

import { KbdChord } from "../Kbd/Kbd";
import type { PaletteEntry, PaletteSection } from "./CommandPalette";

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
  sections: readonly PaletteSection[];
  entries: readonly PaletteEntry[];
  /** What was searched, for the state where nothing matched. */
  searched: string;
  onChoose: (entry: PaletteEntry) => void;
};

export type PaletteDispatch = {
  kind: BarKind;
  /** Tab switches the kind. Absent while there is no other kind to offer, and the row stays a Session's. */
  onKind?: (kind: BarKind) => void;
  /** The kinds Tab steps through, in order: a Session where absent. Autopilot is one when the host serves it (#2092), Task and Project when it keeps projects. */
  kinds?: readonly BarKind[];
  onDispatch: (words: string, kind: BarKind) => void;
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

/** What was typed, as the row that dispatches it. */
export function DispatchRow({
  words,
  kind,
  active,
  trailing,
  ref,
  onEnter,
  onChoose,
}: {
  words: string;
  kind: BarKind;
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
        {OFFER[kind]}
        <span className="armada-palette__words">“{words}”</span>
      </span>
      <span className="armada-palette__keys">
        <KbdChord keys={["⌘", "↵"]} />
      </span>
    </div>
  );
}

/**
 * Where the palette sits when it opens over a field: the field's own box, so its input lands where
 * the field was. Read once on opening, as `Popover` reads its trigger; a resize while open leaves it.
 */
export function useAnchor(open: boolean, anchor: string | undefined): CSSProperties | undefined {
  const [place, setPlace] = useState<CSSProperties>();
  useLayoutEffect(() => {
    if (!open || anchor === undefined) return setPlace(undefined);
    const box = document.querySelector(anchor)?.getBoundingClientRect();
    if (box === undefined || box.width === 0) return setPlace(undefined);
    setPlace({ top: box.top, left: box.left, width: box.width, ["--palette-field" as string]: `${box.height}px` });
  }, [open, anchor]);
  return place;
}


/** The keys the one bar answers, along the palette's foot while there is something typed to send. */
export function DispatchKeys({ dispatch, sends }: { dispatch: PaletteDispatch; sends: boolean }) {
  const verb = adds(dispatch.kind) ? "add" : "dispatch";
  return (
    <div className="armada-palette__keys-foot" aria-hidden>
      <span><KbdChord keys={["↵"]} /> {sends ? verb : "open"}</span>
      <span><KbdChord keys={["⌘", "↵"]} /> {verb}</span>
      {dispatch.onKind === undefined ? null : <span><KbdChord keys={["⇥"]} /> {(dispatch.kinds ?? ["session"]).map((kind) => KIND[kind]).join(" / ")}</span>}
    </div>
  );
}

/** The keys a second step answers, along the palette's foot. */
export function StepKeys() {
  return (
    <div className="armada-palette__keys-foot" aria-hidden>
      <span><KbdChord keys={["↵"]} /> add</span>
      <span><KbdChord keys={["Esc"]} /> back</span>
    </div>
  );
}
