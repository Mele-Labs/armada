import type { ReactNode } from "react";

/**
 * One label and its value, in the task inspector.
 *
 * **The label is a register and the value is plain text.** The label's small
 * caps in `--fg-subtle` against the value in `--fg-default` is what separates
 * one field from the next, and the gap between fields does the rest. The value
 * sat in a sunken box once, `ProposalFields`' treatment, and the owner read it
 * as a text input he could type in (29 Sep 2026) — a panel that only reads must
 * not look like a form. Rows that are rows, a file list or a test list, divide
 * with a hairline instead.
 *
 * **Not an app-wide component.** That is parked; this is the Plan inspector's.
 */
export function TaskField({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <section className="armada-task-sheet__field">
      <h3 className="armada-task-sheet__label">{label}</h3>
      <div className="armada-task-sheet__value">{children}</div>
    </section>
  );
}
