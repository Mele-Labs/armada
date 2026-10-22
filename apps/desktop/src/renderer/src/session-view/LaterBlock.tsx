// Progress: the follow-ups the agent kept for itself, folded under one label. A press opens them, and the button is a tab stop that Enter and Space press.
// Each row has two presses after its words: Remind sends the sentence back to the agent, and Drop settles it himself and takes it off the list.

import { useState } from "react";
import { BellRing, ChevronDown, X } from "lucide-react";
import { Button, Tooltip } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

import "./later-block.css";

type One = NonNullable<Session["later"]>[number];

export function LaterBlock({ later, onRemind, onDrop }: { later: NonNullable<Session["later"]>; onRemind: (sentence: string) => void; onDrop: (one: One) => void }) {
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
            <li key={one.id} className="sv-later__item">
              <span className="sv-later__text">{one.text}</span>
              <Tooltip label="Remind">
                <Button variant="ghost" size="sm" iconOnly aria-label={`Remind: ${one.text}`} onClick={() => onRemind(one.text)}>
                  <BellRing size={12} strokeWidth={2} aria-hidden />
                </Button>
              </Tooltip>
              <Tooltip label="Drop">
                <Button variant="ghost" size="sm" iconOnly aria-label={`Drop: ${one.text}`} onClick={() => onDrop(one)}>
                  <X size={12} strokeWidth={2} aria-hidden />
                </Button>
              </Tooltip>
            </li>
          ))}
        </ul>
      ) : null}
    </section>
  );
}
