// A disclosure row of the folded Progress: its name, a one-line summary of what it holds, and a chevron. A press, Enter or Space opens what it holds inline.

import { useState, type ReactNode } from "react";
import { ChevronDown } from "lucide-react";

/** `summary` is drawn by the row, told whether it stands open: the plan's row gives its current group's title up once the stepper under it names the group. */
export function Fold({ label, summary, children }: { label: string; summary: (open: boolean) => ReactNode; children: ReactNode }) {
  const [open, setOpen] = useState(false);
  return (
    <section className="sv-fold" data-open={open || undefined}>
      <button type="button" className="sv-fold__row" aria-expanded={open} aria-label={label} onClick={() => setOpen((was) => !was)}>
        <span className="sv-fold__label">{label}</span>
        <span className="sv-fold__summary">{summary(open)}</span>
        <ChevronDown className="sv-fold__chevron" size={12} strokeWidth={2} aria-hidden />
      </button>
      {open ? (
        <div className="sv-expand">
          <div className="sv-fold__body">{children}</div>
        </div>
      ) : null}
    </section>
  );
}
