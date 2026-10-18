import type { KeyboardEvent } from "react";

import { Button } from "../../primitives/Button/Button";
import { Card } from "../../primitives/Card/Card";
import { Kbd } from "../../primitives/Kbd/Kbd";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * What Fleet saw a Session do in a repository with no Manifest, offered as ways to set it up: the thread's last card. Each line stands on
 * a fact, which is its tooltip. **Mock only**, one of two takes: the other draws the same lines in the Needs you band.
 * Enter on the card is Set up and Backspace is Not now.
 */
export type NextOffer = {
  repository: string;
  items: readonly { id: string; text: string; fact: string }[];
  onSetUp: () => void;
  onNotNow: () => void;
};

export function NextCard({ offer }: { offer: NextOffer }) {
  const keys = (event: KeyboardEvent<HTMLElement>) => {
    if (event.target !== event.currentTarget || event.metaKey || event.ctrlKey || event.altKey) return;
    if (event.key === "Enter") {
      event.preventDefault();
      offer.onSetUp();
    } else if (event.key === "Backspace") {
      event.preventDefault();
      offer.onNotNow();
    }
  };
  return (
    <Card flat className="armada-session-thread__ask armada-next-card" role="region" aria-label={`Next for ${offer.repository}`} tabIndex={0} onKeyDown={keys}>
      <span className="armada-session-thread__eyebrow">Next for {offer.repository}</span>
      <ul className="armada-next-card__items">
        {offer.items.map((one) => (
          <Tooltip key={one.id} label={one.fact} asChild>
            <li className="armada-next-card__item" tabIndex={0} aria-label={one.text}>
              {one.text}
            </li>
          </Tooltip>
        ))}
      </ul>
      <div className="armada-next-card__acts">
        <Button size="sm" variant="primary" onClick={offer.onSetUp}>
          Set up
          <Kbd aria-hidden>↵</Kbd>
        </Button>
        <Button size="sm" variant="ghost" onClick={offer.onNotNow}>
          Not now
          <Kbd aria-hidden>⌫</Kbd>
        </Button>
      </div>
    </Card>
  );
}

/** One suggestion opened in the Needs you band: the fact it stands on, and the two acts, which settle every suggestion for the repository. */
export function NextBody({ fact, onSetUp, onNotNow }: { fact: string; onSetUp: () => void; onNotNow: () => void }) {
  return (
    <div className="armada-next-card armada-next-card--band">
      <span className="armada-next-card__fact">{fact}</span>
      <div className="armada-next-card__acts">
        <Button size="sm" variant="primary" onClick={onSetUp}>
          Set up
        </Button>
        <Button size="sm" variant="ghost" onClick={onNotNow}>
          Not now
        </Button>
      </div>
    </div>
  );
}
