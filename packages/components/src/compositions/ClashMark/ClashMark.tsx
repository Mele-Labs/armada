import { createContext, useContext } from "react";
import type { MouseEvent, ReactNode } from "react";
import { SquareTerminal, Swords, Workflow } from "lucide-react";

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
 * **No count is drawn** beside any of it.
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

/**
 * The clash in an open Job or Session: the other holder, linked, and the paths that conflict.
 * Nothing where there is none.
 */
export function ClashNotice({ kind, id }: { kind: "job" | "session"; id: string | undefined }) {
  const held = useContext(Held);
  const clashes = useClashesOf(kind, id);
  if (clashes.length === 0) return null;
  return (
    <section className="armada-clash-notice" aria-label="Write clash">
      {clashes.map((clash) => {
        const Glyph = clash.with.kind === "job" ? Workflow : SquareTerminal;
        return (
          <div className="armada-clash-notice__row" key={`${clash.with.kind}:${clash.with.id}`}>
            <Tooltip label="Write clash">
              <span className="armada-clash-mark" role="img" aria-label="Write clash">
                <Swords size={16} strokeWidth={2} aria-hidden="true" />
              </span>
            </Tooltip>
            {clash.with.kind === "main" ? (
              <span className="armada-clash-notice__with" data-main>
                {clash.with.label}
              </span>
            ) : (
              <button type="button" className="armada-clash-notice__with" aria-label={`Open ${clash.with.label}`} onClick={() => held?.open(clash.with)}>
                <Glyph size={14} strokeWidth={2} aria-hidden="true" />
                {clash.with.label}
              </button>
            )}
            <ul className="armada-clash-notice__paths" aria-label="Conflicting files">
              {clash.paths.map((path) => {
                const cut = path.lastIndexOf("/") + 1;
                return (
                  <li key={path}>
                    <PathChip directory={path.slice(0, cut)} basename={path.slice(cut)} title={path} />
                  </li>
                );
              })}
            </ul>
          </div>
        );
      })}
    </section>
  );
}
