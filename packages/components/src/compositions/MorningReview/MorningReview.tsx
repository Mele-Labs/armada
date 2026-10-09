import { useState } from "react";
import { BadgeCheck, Footprints, GitMerge, LockKeyhole, type LucideIcon } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { Sheet } from "../../primitives/Sheet/Sheet";

/** A question sleep mode answered for the owner. `corrected` is what he sent back, once he has. */
export type MorningDecided = { id: string; who: string; asked: string; chose: string; corrected?: string };

/** Something only the owner can settle: a pull request to approve, a refusal that stands. */
export type MorningBlocked = { id: string; who: string; text: string };

/** A pull request that merged while he slept. */
export type MorningLanded = { id: string; who: string; title: string; pr: string };

/** A walk held for him to look at. */
export type MorningWalk = { id: string; who: string; title: string; onOpen: () => void };

export type MorningReviewProps = {
  open: boolean;
  decided: readonly MorningDecided[];
  blocked: readonly MorningBlocked[];
  landed: readonly MorningLanded[];
  walks: readonly MorningWalk[];
  /** Sends the correction to the agent that was answered for. */
  onOverride: (id: string, text: string) => void;
  onClose?: () => void;
};

/**
 * What the night left, in four sections: what was decided for the owner, what
 * still needs him, what landed, and the walks held for him. **A section with
 * nothing in it is not drawn**, and none carries a count.
 *
 * Override is the one act on the sheet that reaches an agent: it opens a field
 * on the row, and sending it tells the Session or Job it chose wrongly.
 */
export function MorningReview({ open, decided, blocked, landed, walks, onOverride, onClose }: MorningReviewProps) {
  const [overriding, setOverriding] = useState<string | null>(null);
  const [text, setText] = useState("");
  const send = (id: string) => {
    if (text.trim() === "") return;
    onOverride(id, text.trim());
    setOverriding(null);
    setText("");
  };

  return (
    <Sheet kind="morning-review" open={open} floating title="Morning review" closeLabel="Close" closeBinding="Esc" onClose={onClose}>
      <div className="armada-morning">
        <Section icon={BadgeCheck} label="Decided for you" shown={decided.length > 0}>
          {decided.map((one) => (
            <li key={one.id} className="armada-morning__row" aria-label={one.who} data-corrected={one.corrected !== undefined}>
              <span className="armada-morning__who">{one.who}</span>
              <span className="armada-morning__asked">{one.asked}</span>
              <span className="armada-morning__chose">{one.chose}</span>
              {one.corrected !== undefined ? (
                <span className="armada-morning__corrected">{one.corrected}</span>
              ) : overriding === one.id ? (
                <form
                  className="armada-morning__override"
                  onSubmit={(event) => {
                    event.preventDefault();
                    send(one.id);
                  }}
                >
                  <Input aria-label="Correction" value={text} autoFocus onChange={(event) => setText(event.target.value)} />
                  <Button type="submit" variant="tonal" size="sm" disabled={text.trim() === ""}>
                    Send
                  </Button>
                  <Button type="button" variant="ghost" size="sm" onClick={() => setOverriding(null)}>
                    Cancel
                  </Button>
                </form>
              ) : (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    setOverriding(one.id);
                    setText("");
                  }}
                >
                  Override
                </Button>
              )}
            </li>
          ))}
        </Section>

        <Section icon={LockKeyhole} label="Still needs you" shown={blocked.length > 0}>
          {blocked.map((one) => (
            <li key={one.id} className="armada-morning__row">
              <span className="armada-morning__who">{one.who}</span>
              <span className="armada-morning__asked">{one.text}</span>
            </li>
          ))}
        </Section>

        <Section icon={GitMerge} label="Landed overnight" shown={landed.length > 0}>
          {landed.map((one) => (
            <li key={one.id} className="armada-morning__row">
              <span className="armada-morning__who">{one.who}</span>
              <span className="armada-morning__asked">{one.title}</span>
              <span className="armada-morning__pr">{one.pr}</span>
            </li>
          ))}
        </Section>

        <Section icon={Footprints} label="Walks waiting" shown={walks.length > 0}>
          {walks.map((one) => (
            <li key={one.id} className="armada-morning__row">
              <span className="armada-morning__who">{one.who}</span>
              <span className="armada-morning__asked">{one.title}</span>
              <Button variant="ghost" size="sm" onClick={one.onOpen}>
                Walk
              </Button>
            </li>
          ))}
        </Section>
      </div>
    </Sheet>
  );
}

function Section({ icon: Icon, label, shown, children }: { icon: LucideIcon; label: string; shown: boolean; children: React.ReactNode }) {
  if (!shown) return null;
  return (
    <section className="armada-morning__section" aria-label={label}>
      <h3 className="armada-morning__heading">
        <Icon size={16} strokeWidth={2} aria-hidden />
        {label}
      </h3>
      <ul className="armada-morning__rows">{children}</ul>
    </section>
  );
}
