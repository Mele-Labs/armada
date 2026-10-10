// The palette's dispatch row: what was typed in the title bar's one bar, offered as the work itself
// (the owner, 10 Oct 2026), and where the palette sits when it opens over that bar.

import { useLayoutEffect, useState, type CSSProperties } from "react";
import { Plus } from "lucide-react";

import { KbdChord } from "../Kbd/Kbd";

export type DispatchKind = "job" | "session";

export type PaletteDispatch = {
  kind: DispatchKind;
  /** Tab switches the kind. Absent while Sessions are not offered, and the row stays a Job's. */
  onKind?: (kind: DispatchKind) => void;
  onDispatch: (words: string, kind: DispatchKind) => void;
  /** ⇧↵: the full composer, holding the words, for a file, a sketch or a long request. */
  onCompose?: (words: string) => void;
  /** Opened by `n`: the dispatch row leads whatever else matches. */
  leads?: boolean;
};

/**
 * Words that read as work rather than a search: a link to a ticket. A sentence is not enough, since
 * a Job is searched by its title and a title is a sentence — "pin the store" opens that Job.
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
  ref,
  onEnter,
  onChoose,
}: {
  words: string;
  kind: DispatchKind;
  active: boolean;
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
      className={["armada-palette__row", "armada-palette__row--dispatch", active ? "armada-palette__row--active" : ""].join(" ").trim()}
      onMouseEnter={onEnter}
      onClick={onChoose}
    >
      <span className="armada-palette__glyph">
        <Plus size={16} strokeWidth={2} aria-hidden />
      </span>
      <span className="armada-palette__label">
        {kind === "job" ? "Dispatch " : "Start a Session with "}
        <span className="armada-palette__words">“{words}”</span>
        {kind === "job" ? " as a Job" : ""}
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
      {dispatch.onKind === undefined ? null : <span><KbdChord keys={["⇥"]} /> Job / Session</span>}
      {dispatch.onCompose === undefined ? null : <span><KbdChord keys={["⇧", "↵"]} /> full composer</span>}
    </div>
  );
}
