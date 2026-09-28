import type { ReactNode } from "react";

import { ConceptLabel } from "../../concepts";

/**
 * A run of settings, label beside value down one column.
 *
 * **The tracks are declared here and borrowed by every row**, so the labels
 * line up down the panel rather than per row. Content-sized rather than a
 * drawn width: a fixed label column transcribed from a board truncates the
 * longest label beside empty space, which is the defect `bridge.md` records.
 *
 * **Every label starts at the same edge, and the tier rows used to not**
 * (`yc0v`, 28 Sep). Difficult, Medium and Easy were right-aligned and a size
 * smaller, to read as belonging to the heading above them — which took three
 * of the panel's labels off the line the other nine sit on. The heading
 * already says they belong to it.
 */
export function ProposalFields({ children }: { children: ReactNode }) {
  return <div className="armada-proposal__fields">{children}</div>;
}

/**
 * One setting: its name, and its value in a box of its own.
 *
 * **The box is what separates one setting from the next** — the grid without
 * it was one wall of same-weight text, which is the reading the owner
 * refused. A control brings its own box, so `bare` leaves it alone rather
 * than drawing a second one round it.
 */
export function ProposalField({
  label,
  bare,
  children,
}: {
  /** The setting's name. A word `concepts.ts` knows gets its sentence on hover. */
  label: string;
  /** The value is a control, which already draws its own box. */
  bare?: boolean;
  children: ReactNode;
}) {
  return (
    <div className="armada-proposal__field">
      <ConceptLabel className="armada-proposal__field-label">{label}</ConceptLabel>
      <div className="armada-proposal__field-value" data-bare={bare === true ? "true" : undefined}>
        {children}
      </div>
    </div>
  );
}

/** A setting's own note, under the run it belongs to. */
export function ProposalFieldNote({ children }: { children: ReactNode }) {
  return <p className="armada-proposal__field-note">{children}</p>;
}
