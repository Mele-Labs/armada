import type { HTMLAttributes } from "react";

/**
 * A figure or an id the system reports, drawn as a segmented display: mono at `--text-2xs` with tabular numerals,
 * in a well of `--bg-sunken` behind a `--border-subtle` hairline at the inset curve. `#1899`, a commit, `13:50`,
 * a duration, a setting's key and a bay's number are all this one thing.
 *
 * **It belongs at the right of its row, in a column of its own.** A readout in a list takes `fixed`, which gives it
 * one width for every row of that list, so the figures line up down the page whether or not a row has one: a row
 * without one draws an empty `Readout` (`aria-hidden`) and keeps the column. The number of characters a column holds
 * is the list's to say, as `--readout-chars` on the row; the default is six.
 *
 * A readout is a reading and never a control. A press belongs on a button beside it.
 */
export type ReadoutProps = HTMLAttributes<HTMLSpanElement> & {
  /** One width for every row of the list, the figure at the well's right edge. */
  fixed?: boolean;
};

export function Readout({ className, fixed = false, children, ...rest }: ReadoutProps) {
  const empty = children === undefined || children === null || children === "";
  return (
    <span
      {...rest}
      className={className === undefined ? "armada-readout" : `armada-readout ${className}`}
      data-fixed={fixed || undefined}
      data-empty={empty || undefined}
      aria-hidden={empty ? true : rest["aria-hidden"]}
    >
      {children}
    </span>
  );
}
