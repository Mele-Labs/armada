import type { ReactNode } from "react";

/**
 * One label and its value, in the task inspector.
 *
 * **The label is a register and the value is a box.** Four notes across four
 * surfaces say the same thing: *"they are all the same color and weight just a
 * slight difference to the font size"* (owner, 28 Sep 2026). `ProposalFields`
 * answered it for the proposal by putting the value in `--bg-sunken` inside
 * `--border-default`, and this is that treatment stacked — the design board
 * draws this panel with the label above rather than beside, because 392px has
 * no room for two tracks.
 *
 * **Not an app-wide component.** That is parked; this is the Plan inspector's.
 */
export function TaskField({
  label,
  note,
  bare,
  children,
}: {
  label: string;
  /** A fact about the value, under it. `declared 3 · touched 2`. */
  note?: ReactNode;
  /** The value draws its own boxes — a list of rows, a control. */
  bare?: boolean;
  children: ReactNode;
}) {
  return (
    <section className="armada-task-sheet__field">
      <h3 className="armada-task-sheet__label">{label}</h3>
      <div className="armada-task-sheet__value" data-bare={bare === true ? "true" : undefined}>
        {children}
      </div>
      {note === undefined ? null : <p className="armada-task-sheet__note">{note}</p>}
    </section>
  );
}
