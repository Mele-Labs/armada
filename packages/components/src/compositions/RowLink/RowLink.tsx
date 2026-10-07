import type { ReactNode } from "react";

/**
 * One line that opens a row of the Record: a 12px mark, a name, and what it
 * came to. **One treatment for every such line** — a Check at a group's
 * boundary, and a task a failed Check held back — so pressing one reads the
 * same wherever it is drawn (the owner, 29 Sep 2026).
 *
 * **A button only where there is a row to open.** Absent `onOpen`, the same
 * line is drawn as text, and nothing pretends to be pressable.
 */
export type RowLinkProps = {
  /** The mark leading it. Absent keeps its slot, so names line up. */
  mark?: ReactNode;
  /** The name — a Check's, or a task's id and title. */
  children: ReactNode;
  /** Draw the name in the mono face, as a declared name is. */
  mono?: boolean;
  /** What it came to, as a word at its end. */
  says?: string;
  /** The hue the mark and the word take. Only the two that are a verdict take one. */
  tone?: "passed" | "failed";
  /** The accessible name, where the line's own text does not say it whole. */
  label?: string;
  /** What the line's word rests on, said on hover. */
  hint?: string;
  onOpen?: () => void;
};

export function RowLink({ mark, children, mono = false, says, tone, label, hint, onOpen }: RowLinkProps) {
  const line = (
    <>
      <span className="armada-row-link__mark" aria-hidden={label === undefined ? undefined : true}>
        {mark}
      </span>
      <span className="armada-row-link__name" data-mono={mono ? "true" : undefined}>
        {children}
      </span>
      {says === undefined ? null : <span className="armada-row-link__says">{says}</span>}
    </>
  );
  return onOpen === undefined ? (
    <span className="armada-row-link" data-tone={tone} title={hint}>
      {line}
    </span>
  ) : (
    <button type="button" className="armada-row-link" data-tone={tone} aria-label={label} title={hint} onClick={onOpen}>
      {line}
    </button>
  );
}
