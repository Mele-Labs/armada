import type { ButtonHTMLAttributes, ReactNode, Ref } from "react";

import { Skeleton } from "../../primitives/Skeleton/Skeleton";
import { Stamp } from "../../primitives/Stamp/Stamp";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

export type ListCardProps = Omit<ButtonHTMLAttributes<HTMLButtonElement>, "title" | "children"> & {
  /** What it is called, on one line. */
  title: ReactNode;
  /** The title may take a second line, on a list whose titles say more than a few words. */
  wrap?: boolean;
  /** The marks in the corner, each a `ListCardMark`, in the one order: linked item, Job, state. A list passes the same ones for every row, so an empty one keeps its place. */
  marks?: ReactNode;
  /** What it is, on the line under the title: its number or key, a `ListCardChip`. */
  tag?: ReactNode;
  /** When it last moved, on the tag's line at the right: a `ListCardTime`, or what a surface draws in its place. */
  at?: ReactNode;
  /** Lines under those, as the person's View menu has them. */
  lines?: ReactNode;
  selected?: boolean;
  /** Needs him: the card's faint tint pulses while it does. */
  needs?: boolean;
  ref?: Ref<HTMLButtonElement>;
};

/**
 * One thing on a list, the same card whatever it is: a Job, an issue, a pull request or a Logbook entry. **The title takes the room, the marks hold the top right corner in
 * fixed places and the state is last**, so a state is found in one place on every list. The number is on its own line under the title, and the time is plain quiet text at the end
 * of that line, with no box. Everything else is a line under those, which the person turns on.
 */
export function ListCard({
  title,
  wrap = false,
  marks,
  tag,
  at,
  lines,
  selected = false,
  needs = false,
  className,
  ref,
  ...rest
}: ListCardProps) {
  return (
    <button
      {...rest}
      ref={ref}
      type="button"
      className={className === undefined ? "armada-list-card" : `armada-list-card ${className}`}
      data-wrap={wrap || undefined}
      data-selected={selected || undefined}
      data-needs={needs || undefined}
      aria-current={selected || undefined}
    >
      <span className="armada-list-card__title">{title}</span>
      {marks === undefined ? null : <span className="armada-list-card__marks">{marks}</span>}
      {tag === undefined ? null : <span className="armada-list-card__tag">{tag}</span>}
      {at === undefined ? null : <span className="armada-list-card__at">{at}</span>}
      {lines === undefined ? null : <span className="armada-list-card__lines">{lines}</span>}
    </button>
  );
}

/** One place in the corner. It keeps its width when empty, and `wide` takes two glyphs, as a state of two parts does. */
export function ListCardMark({ wide = false, children }: { wide?: boolean; children?: ReactNode }) {
  return (
    <span className="armada-list-card__mark" data-wide={wide || undefined}>
      {children}
    </span>
  );
}

/** A number or key on the card's tag line. */
export function ListCardChip({ children }: { children: ReactNode }) {
  return <span className="armada-list-card__chip">{children}</span>;
}

/** The time, as an age, with what it is the age of in the tooltip. */
export function ListCardTime({ at, said }: { at: string | number; said: string }) {
  return (
    <Tooltip label={said}>
      <span>
        <Stamp still first="age" at={at} />
      </span>
    </Tooltip>
  );
}

/** The card before its list has answered: its title and tag as bars, so nothing moves when they land. */
export function ListCardPending({ width }: { width: string }) {
  return (
    <div className="armada-list-card" data-pending="" aria-hidden>
      <span className="armada-list-card__title">
        <Skeleton width={width} />
      </span>
      <span className="armada-list-card__tag">
        <Skeleton width="3rem" />
      </span>
    </div>
  );
}
