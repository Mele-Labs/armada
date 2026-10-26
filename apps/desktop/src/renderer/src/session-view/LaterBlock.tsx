// Deferred: the follow-ups the agent kept for itself, in the Landing, folded under one label. A press opens them, and the button is a tab stop that Enter and Space press.
// Each row has two presses after its words: Remind sends the sentence back to the agent, and Drop settles it himself and takes it off the list.
// The issues the Job filed for later stand in the same list, a row each that opens the issue as the Blueprint's chips do; they have no Remind or Drop.

import { useState } from "react";
import { BellRing, ChevronDown, X } from "lucide-react";
import { Button, Tooltip } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

import { IssueRow, type Issue } from "./IssuesBlock";
import "./later-block.css";

type One = NonNullable<Session["later"]>[number];

export function LaterBlock({ later, issues, manifestId, onRemind, onDrop }: { later: NonNullable<Session["later"]>; issues: readonly Issue[]; manifestId: string | undefined; onRemind: (sentence: string) => void; onDrop: (one: One) => void }) {
  const [open, setOpen] = useState(false);
  return (
    <section className="sv-later" data-open={open || undefined} aria-label="Deferred">
      <button type="button" className="sv-later__row" aria-expanded={open} onClick={() => setOpen((was) => !was)}>
        <span className="sv-later__label">Deferred</span>
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
          {issues.map((issue) => (
            <li key={issue.number} className="sv-later__item" data-kind="issue">
              <IssueRow issue={issue} manifestId={manifestId} />
            </li>
          ))}
        </ul>
      ) : null}
    </section>
  );
}
