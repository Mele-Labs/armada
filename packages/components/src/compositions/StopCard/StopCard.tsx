import { useEffect, useRef, useState } from "react";
import type { HTMLAttributes, ReactNode, Ref } from "react";

/** One fact of a card: its term, its value, and whether the value is mono (a figure the system reported). */
export type StopCardRow = readonly [term: string, value: string, mono?: boolean];

export type StopCardProps = Omit<HTMLAttributes<HTMLDivElement>, "title"> & {
  /** The toned pill on the head's leading side. */
  pill: ReactNode;
  /** Subtle, on the head's trailing side. */
  where: ReactNode;
  /** In heading weight. */
  title: ReactNode;
  /** Between the title and the facts: a bar, a strip. */
  lead?: ReactNode;
  /** The facts, as subtle terms over their values. None draws no list. */
  rows?: readonly StopCardRow[];
  ref?: Ref<HTMLDivElement>;
  /** What follows the facts: acts, a hint. */
  children?: ReactNode;
};

/**
 * The card a metro stop opens in the Cockpit's Bays view, and the one a plan meter opens in the title
 * bar: a head with a toned pill and a subtle place, a title, and facts. **The tone is the caller's**, as
 * `--tone` on the card (its top edge and the pill take it); where the card sits, when it opens and how it
 * is kept open are the caller's too, since the two places differ in all three.
 */
export function StopCard({ pill, where, title, lead, rows = [], className, children, ref, ...rest }: StopCardProps) {
  return (
    <div ref={ref} className={["armada-stop-card", className].filter(Boolean).join(" ")} {...rest}>
      <span className="armada-stop-card__head">
        <span className="armada-stop-card__pill">{pill}</span>
        <span className="armada-stop-card__where">{where}</span>
      </span>
      <span className="armada-stop-card__title">{title}</span>
      {lead}
      {rows.length === 0 ? null : (
        <dl className="armada-stop-card__facts">
          {rows.map(([term, value, mono]) => (
            <div key={term}>
              <dt>{term}</dt>
              <dd data-mono={mono || undefined}>{value}</dd>
            </div>
          ))}
        </dl>
      )}
      {children}
    </div>
  );
}

/** A subtle heading over a list of facts, drawn after a card's own. An empty list draws nothing. */
export function StopCardList({ heading, rows }: { heading: string; rows: readonly StopCardRow[] }) {
  if (rows.length === 0) return null;
  return (
    <>
      <span className="armada-stop-card__heading">{heading}</span>
      <dl className="armada-stop-card__facts">
        {rows.map(([term, value, mono]) => (
          <div key={term}>
            <dt>{term}</dt>
            <dd data-mono={mono || undefined}>{value}</dd>
          </div>
        ))}
      </dl>
    </>
  );
}

/**
 * When a card is open, as the metro stop's is: **hover or focus opens it, a press keeps it open, and Esc
 * or a press outside closes it.** `bind` goes on the element holding both the trigger and the card.
 */
export function useCardOpen() {
  const [hovered, setHovered] = useState(false);
  const [pinned, setPinned] = useState(false);
  const here = useRef<HTMLSpanElement>(null);
  const open = hovered || pinned;
  useEffect(() => {
    if (!open) return;
    const close = () => {
      setPinned(false);
      setHovered(false);
    };
    const outside = (event: PointerEvent) => {
      if (here.current !== null && !here.current.contains(event.target as Node)) close();
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape") close();
    };
    document.addEventListener("pointerdown", outside);
    document.addEventListener("keydown", escape);
    return () => {
      document.removeEventListener("pointerdown", outside);
      document.removeEventListener("keydown", escape);
    };
  }, [open]);
  return {
    open,
    pinned,
    toggle: () => setPinned((on) => !on),
    /** Closes it, hovered or kept: for an act in the card that takes the owner somewhere else. */
    close: () => {
      setPinned(false);
      setHovered(false);
    },
    bind: {
      ref: here,
      onMouseEnter: () => setHovered(true),
      onMouseLeave: () => setHovered(false),
      onFocus: () => setHovered(true),
      onBlur: () => setHovered(false),
    },
  };
}
