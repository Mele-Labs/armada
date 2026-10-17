import { createContext, useContext, useId, useState } from "react";
import type { MouseEvent, ReactNode } from "react";
import { ChevronDown, ChevronRight, SquareTerminal, Swords } from "lucide-react";

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
export type ClashHolderRef = { kind: "session" | "main"; id: string; label: string };

/** One clash as a holder sees it: who it is with, and where the changes collide. */
export type ClashView = { with: ClashHolderRef; paths: readonly string[] };

export type Clashing = {
  of: (kind: "session", id: string) => readonly ClashView[];
  /** Opens the other holder: its Session. */
  open: (holder: ClashHolderRef) => void;
};

const Held = createContext<Clashing | undefined>(undefined);

export function ClashesProvider({ value, children }: { value: Clashing; children: ReactNode }) {
  return <Held.Provider value={value}>{children}</Held.Provider>;
}

const NONE: readonly ClashView[] = [];

/** The clashes a Session is in. Empty where nothing provides them. */
export function useClashesOf(kind: "session", id: string | undefined): readonly ClashView[] {
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

/**
 * The mark for a holder, one however many clashes it is in; nothing where it has none. In one clash a press opens
 * the other holder. In several the card names each with its paths, and a press is the row's own: the open
 * Session's bar links every holder.
 */
export function ClashMark({ kind, id, size = 12 }: { kind: "session"; id: string | undefined; size?: 12 | 16 }) {
  const held = useContext(Held);
  const clashes = useClashesOf(kind, id);
  if (clashes.length === 0) return null;
  if (clashes.length > 1) {
    const said = `Conflicts with ${clashes.map((clash) => clash.with.label).join(", ")}`;
    const card = (
      <span className="armada-clash-mark__card">
        <span>Conflicts with</span>
        {clashes.map((clash) => (
          <span key={`${clash.with.kind}:${clash.with.id}`}>{`${clash.with.label} · ${clash.paths.join(", ")}`}</span>
        ))}
      </span>
    );
    return (
      <Tooltip label={card}>
        <span className="armada-clash-mark" role="img" aria-label={said}>
          <Swords size={size} strokeWidth={2} aria-hidden="true" />
        </span>
      </Tooltip>
    );
  }
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

/** The Sessions whose bar he opened, by owner. One bar to an owner, so a clash arriving does not change it. */
const opened = new Set<string>();

/**
 * The clashes in an open Session, as one bar however many there are. Folded it is a line: the other holders as chips,
 * each linked and named in a tooltip. Opened it lists each clash as a row: the other holder and the paths that conflict.
 * A long list of paths folds to its first few and one chip to open the rest. Nothing where there is none.
 */
export function ClashNotice({ kind, id }: { kind: "session"; id: string | undefined }) {
  const clashes = useClashesOf(kind, id);
  if (clashes.length === 0) return null;
  return (
    <section className="armada-clash-notice" aria-label="Write clash">
      <ClashBar clashes={clashes} owner={`${kind}:${id}`} />
    </section>
  );
}

function Holder({ clash }: { clash: ClashView }) {
  const held = useContext(Held);
  return clash.with.kind === "main" ? (
    <Tooltip label={clashSaid(clash)}>
      <span className="armada-clash-notice__with" data-main>
        {clash.with.label}
      </span>
    </Tooltip>
  ) : (
    // Beside the toggle and not in it: a press opens the holder and leaves the bar as it is.
    <Tooltip label={clashSaid(clash)}>
      <button type="button" className="armada-clash-notice__with" aria-label={`Open ${clash.with.label}`} onClick={() => held?.open(clash.with)}>
        <SquareTerminal size={14} strokeWidth={2} aria-hidden="true" />
        {clash.with.label}
      </button>
    </Tooltip>
  );
}

/** What a folded bar says of more than one other holder, in place of a chip for each: the owner's ask of 17 Oct 2026. */
function othersSaid(clashes: readonly ClashView[]): string {
  return `${clashes.length} ${clashes.some((one) => one.with.kind === "main") ? "others" : "other Sessions"}`;
}

function ClashBar({ clashes, owner }: { clashes: readonly ClashView[]; owner: string }) {
  const [shown, setShown] = useState(opened.has(owner));
  const toggle = () => {
    if (shown) opened.delete(owner);
    else opened.add(owner);
    setShown(!shown);
  };
  const boxId = useId();
  const Fold = shown ? ChevronDown : ChevronRight;
  return (
    <div className="armada-clash-notice__row">
      {/* The whole line folds and opens; a holder's own link, beside it, only opens the holder. */}
      <div className="armada-clash-notice__head" onClick={(event) => { if ((event.target as Element).closest("button.armada-clash-notice__with") === null) toggle(); }}>
        <button type="button" className="armada-clash-notice__toggle" aria-expanded={shown} aria-controls={boxId}>
          <Fold size={12} strokeWidth={2} aria-hidden="true" />
          <span className="armada-clash-mark" aria-hidden="true">
            <Swords size={16} strokeWidth={2} />
          </span>
          Write clash
        </button>
        {shown ? null : clashes.length === 1 ? (
          <span className="armada-clash-notice__chips">
            <Holder clash={clashes[0]!} />
          </span>
        ) : (
          <span className="armada-clash-notice__others">{othersSaid(clashes)}</span>
        )}
      </div>
      <div className="armada-clash-notice__fold-box" id={boxId} data-open={shown || undefined} inert={!shown}>
        <ul className="armada-clash-notice__clashes" aria-label="Clashes">
          {clashes.map((clash) => (
            <li key={`${clash.with.kind}:${clash.with.id}`} className="armada-clash-notice__clash">
              <Holder clash={clash} />
              <ClashPaths clash={clash} />
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}

/** The paths a clash names, a long list folded to its first few and one chip to open the rest. With `onPath`, each path is a press. */
export function ClashPaths({ clash, onPath }: { clash: ClashView; onPath?: (path: string) => void }) {
  const [all, setAll] = useState(false);
  const folds = clash.paths.length > SHOWN + 1;
  const drawn = folds && !all ? clash.paths.slice(0, SHOWN) : clash.paths;
  return (
    <ul className="armada-clash-notice__paths" aria-label={`Conflicting files with ${clash.with.label}`}>
      {drawn.map((path) => {
        const cut = path.lastIndexOf("/") + 1;
        const chip = <PathChip directory={path.slice(0, cut)} basename={path.slice(cut)} title={path} />;
        return (
          <li key={path}>
            {onPath === undefined ? (
              chip
            ) : (
              <button type="button" className="armada-clash-notice__open" aria-label={`Open ${path}`} onClick={() => onPath(path)}>
                {chip}
              </button>
            )}
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
  );
}
