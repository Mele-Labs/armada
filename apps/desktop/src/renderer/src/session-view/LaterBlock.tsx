// Progress: the follow-ups the agent kept for itself, folded under one label. A press opens them, and the button is a tab stop that Enter and Space press.

import { useState } from "react";
import { ChevronDown } from "lucide-react";
import type { Session } from "@armada/screens/src/draft/sessions";

import "./later-block.css";

export function LaterBlock({ later }: { later: NonNullable<Session["later"]> }) {
  const [open, setOpen] = useState(false);
  return (
    <section className="sv-later" data-open={open || undefined} aria-label="Later">
      <button type="button" className="sv-later__row" aria-expanded={open} onClick={() => setOpen((was) => !was)}>
        <span className="sv-later__label">Later</span>
        <ChevronDown className="sv-later__chevron" size={12} strokeWidth={2} aria-hidden />
      </button>
      {open ? (
        <ul className="sv-later__list">
          {later.map((one) => (
            <li key={one.at + one.text} className="sv-later__item">
              {one.text}
            </li>
          ))}
        </ul>
      ) : null}
    </section>
  );
}
