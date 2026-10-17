// The palette's dispatch row: what was typed in the title bar's one bar, offered as the work itself
// (the owner, 10 Oct 2026), and where the palette sits when it opens over that bar.

import { useLayoutEffect, useState, type CSSProperties } from "react";
import { Plus } from "lucide-react";

import { KbdChord } from "../Kbd/Kbd";

export type DispatchKind = "session" | "autopilot";

export type PaletteDispatch = {
  kind: DispatchKind;
  /** Tab switches the kind. Absent while there is no other kind to offer, and the row stays a Session's. */
  onKind?: (kind: DispatchKind) => void;
  /** The kinds Tab steps through, in order: a Session where absent. Autopilot is one when the host serves it (#2092). */
  kinds?: readonly DispatchKind[];
  onDispatch: (words: string, kind: DispatchKind) => void;
};

const KIND: Record<DispatchKind, string> = { session: "Job", autopilot: "Autopilot" };

/** The kind Tab steps to from this one, round the kinds offered. */
export function nextKind(dispatch: PaletteDispatch): DispatchKind {
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
  kind: DispatchKind;
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
        {kind === "session" ? "Start a Job with " : "Start Autopilot with "}
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
  return (
    <div className="armada-palette__keys-foot" aria-hidden>
      <span><KbdChord keys={["↵"]} /> {sends ? "dispatch" : "open"}</span>
      <span><KbdChord keys={["⌘", "↵"]} /> dispatch</span>
      {dispatch.onKind === undefined ? null : <span><KbdChord keys={["⇥"]} /> {(dispatch.kinds ?? ["session"]).map((kind) => KIND[kind]).join(" / ")}</span>}
    </div>
  );
}
