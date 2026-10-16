import { createContext, useContext, useId, useState } from "react";
import type { MouseEvent, ReactNode } from "react";
import { ChevronDown, ChevronRight, SquareTerminal, Swords, Workflow } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { PathChip } from "../PathChip/PathChip";

/**
 * A write clash, on both rows of the pair. **One registry mark (`swords`) with
 * its tooltip**, named for the other holder and the paths that conflict; a
 * press opens that holder. The holder is `main` when an agent's changes no
 * longer fit a main that moved, and there is nothing of main to open, so that
 * mark is a picture and not a button.
 *
 * Fleet tells the agents; this only draws what it says. A pair that touches
 * one file without a conflict is not in the list, so nothing is drawn.
 * **No count is drawn** beside any of it; the one figure is the `N more` on a folded list of paths.
 *
 * The rows ask for their own marks (`ClashMark kind id`), so a list does not
 * carry clashes through its props: the host provides what is known once.
 */
export type ClashHolderRef = { kind: "job" | "session" | "main"; id: string; label: string };

/** One clash as a holder sees it: who it is with, and where the changes collide. */
export type ClashView = { with: ClashHolderRef; paths: readonly string[] };

export type Clashing = {
  of: (kind: "job" | "session", id: string) => readonly ClashView[];
  /** Opens the other holder: its Job, or its Session. */
  open: (holder: ClashHolderRef) => void;
};

const Held = createContext<Clashing | undefined>(undefined);

export function ClashesProvider({ value, children }: { value: Clashing; children: ReactNode }) {
  return <Held.Provider value={value}>{children}</Held.Provider>;
}

const NONE: readonly ClashView[] = [];

/** The clashes a Job or a Session is in. Empty where nothing provides them. */
export function useClashesOf(kind: "job" | "session", id: string | undefined): readonly ClashView[] {
  const held = useContext(Held);
  return held === undefined || id === undefined ? NONE : held.of(kind, id);
}

/** Opens the other holder of a clash. For a view that draws the mark its own way, as the fleet map does on a star. */
export function useOpenClash(): (holder: ClashHolderRef) => void {
  const held = useContext(Held);
  return (holder) => held?.open(holder);
}

/** What the mark's tooltip says. */
export const clashSaid = (clash: ClashView): string => `Conflicts with ${clash.with.label} · ${clash.paths.join(", ")}`;

/** The marks for a holder, one a clash; nothing where it has none. */
export function ClashMark({ kind, id, size = 12 }: { kind: "job" | "session"; id: string | undefined; size?: 12 | 16 }) {
  const held = useContext(Held);
  const clashes = useClashesOf(kind, id);
  if (clashes.length === 0) return null;
  return (
    <>
      {clashes.map((clash) => {
        const said = clashSaid(clash);
        const glyph = <Swords size={size} strokeWidth={2} aria-hidden="true" />;
        const key = `${clash.with.kind}:${clash.with.id}`;
        return clash.with.kind === "main" ? (
          <Tooltip key={key} label={said}>
            <span className="armada-clash-mark" role="img" aria-label={said}>
              {glyph}
            </span>
          </Tooltip>
        ) : (
          <Tooltip key={key} label={said}>
            <button
              type="button"
              className="armada-clash-mark"
              data-opens
              aria-label={said}
              onClick={(event: MouseEvent) => {
                // The row it sits on selects or opens itself on a press; this opens the other holder.
                event.stopPropagation();
                held?.open(clash.with);
              }}
            >
              {glyph}
            </button>
          </Tooltip>
        );
      })}
    </>
  );
}

/** Paths drawn while a long list is folded. A list of one more than this is drawn whole: "1 more" saves nothing. */
const SHOWN = 3;

/** The clashes whose rows he opened, by owner and holder. A clash not here starts folded, so a new one folds again. */
const opened = new Set<string>();

/**
 * The clash in an open Job or Session: the other holder, linked, and the paths that conflict.
 * A long list folds to its first few paths and one chip to open the rest. Nothing where there is none.
 */
export function ClashNotice({ kind, id }: { kind: "job" | "session"; id: string | undefined }) {
  const clashes = useClashesOf(kind, id);
  if (clashes.length === 0) return null;
  return (
    <section className="armada-clash-notice" aria-label="Write clash">
      {clashes.map((clash) => (
        <ClashRow key={`${clash.with.kind}:${clash.with.id}`} clash={clash} owner={`${kind}:${id}`} />
      ))}
    </section>
  );
}

function ClashRow({ clash, owner }: { clash: ClashView; owner: string }) {
  const held = useContext(Held);
  const remembered = `${owner}>${clash.with.kind}:${clash.with.id}`;
  const [shown, setShown] = useState(opened.has(remembered));
  const toggle = () => {
    if (shown) opened.delete(remembered);
    else opened.add(remembered);
    setShown(!shown);
  };
  const [all, setAll] = useState(false);
  const listId = useId();
  const Glyph = clash.with.kind === "job" ? Workflow : SquareTerminal;
  const folds = clash.paths.length > SHOWN + 1;
  const drawn = folds && !all ? clash.paths.slice(0, SHOWN) : clash.paths;
  const Fold = shown ? ChevronDown : ChevronRight;
  const holder = clash.with.kind === "main" ? (
    <span className="armada-clash-notice__with" data-main>
      {clash.with.label}
    </span>
  ) : (
    // Beside the toggle and not in it: a press opens the holder and leaves the panel as it is.
    <button type="button" className="armada-clash-notice__with" aria-label={`Open ${clash.with.label}`} onClick={() => held?.open(clash.with)}>
      <Glyph size={14} strokeWidth={2} aria-hidden="true" />
      {clash.with.label}
    </button>
  );
  return (
    <div className="armada-clash-notice__row">
      {/* The whole line folds and opens; the holder's own link, beside it, only opens the holder. */}
      <div className="armada-clash-notice__head" onClick={(event) => { if ((event.target as Element).closest("button.armada-clash-notice__with") === null) toggle(); }}>
        <button type="button" className="armada-clash-notice__toggle" aria-expanded={shown} aria-controls={listId}>
          <Fold size={12} strokeWidth={2} aria-hidden="true" />
          <span className="armada-clash-mark" aria-hidden="true">
            <Swords size={16} strokeWidth={2} />
          </span>
          Write clash
        </button>
        {holder}
      </div>
      <div className="armada-clash-notice__fold-box" data-open={shown || undefined} inert={!shown}>
      <ul className="armada-clash-notice__paths" id={listId} aria-label="Conflicting files">
        {drawn.map((path) => {
          const cut = path.lastIndexOf("/") + 1;
          return (
            <li key={path}>
              <PathChip directory={path.slice(0, cut)} basename={path.slice(cut)} title={path} />
            </li>
          );
        })}
        {folds ? (
          <li>
            <button type="button" className="armada-path armada-clash-notice__fold" aria-expanded={all} onClick={() => setAll(!all)}>
              <span className="armada-path__base">{all ? "Fewer" : `${clash.paths.length - SHOWN} more`}</span>
            </button>
          </li>
        ) : null}
      </ul>
      </div>
    </div>
  );
}
