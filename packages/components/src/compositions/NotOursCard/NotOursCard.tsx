import { useState } from "react";
import { CornerUpRight } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Card } from "../../primitives/Card/Card";
import { CopyAct } from "../../primitives/Prose/Prose";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/** A part set aside as not ours that failed, with the agent's sentence for why it is not the change's doing, and the exact lines it failed on where its reading gave them. */
export type NotOursPart = { name: string; sentence: string; error?: string };

/**
 * A watch that ended not ours, waiting on the person in Needs you: the parts that failed, each with the agent's sentence, the
 * failing run's link, and Fix it, Ignore and Escalate in one row. Escalate shows `summary`, plain lines for the owners of what
 * broke, with one copy control; it sends nothing, and the card stays until Fix it or Ignore settles it.
 */
export function NotOursCard({
  watch,
  parts,
  link,
  summary,
  onFixIt,
  onIgnore,
  onOpenLink,
}: {
  watch: string;
  parts: readonly NotOursPart[];
  link?: string;
  summary: string;
  onFixIt: () => void;
  onIgnore: () => void;
  onOpenLink?: (address: string) => void;
}) {
  const [escalating, setEscalating] = useState(false);
  const failed = `${parts.map((one) => one.name).join(", ")} failed`;
  return (
    <Card flat className="armada-not-ours" role="group" aria-label={`Not ours: ${watch}, ${failed}`}>
      <div className="armada-not-ours__band">
        <Tooltip label={`Not ours: ${failed}`}>
          <span role="img" aria-label={`Not ours: ${failed}`} className="armada-not-ours__mark">
            <CornerUpRight size={16} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
        <span className="armada-not-ours__watch">{watch}</span>
        {link === undefined ? null : (
          <a
            className="armada-not-ours__link"
            href={link}
            onClick={(event) => {
              if (onOpenLink === undefined) return;
              event.preventDefault();
              onOpenLink(link);
            }}
          >
            Failing run
          </a>
        )}
      </div>
      <ul className="armada-not-ours__parts" aria-label="What failed">
        {parts.map((one) => (
          <li key={one.name} className="armada-not-ours__part">
            <span className="armada-not-ours__name">{one.name} failed</span>
            <span className="armada-not-ours__sentence">Not ours: {one.sentence}</span>
            {one.error === undefined ? null : (
              <pre className="armada-not-ours__lines" aria-label={`${one.name} failed on`}>
                {one.error}
              </pre>
            )}
          </li>
        ))}
      </ul>
      <div className="armada-not-ours__answers" role="group" aria-label="Answers">
        <Tooltip label="Asks the agent to fix it">
          <Button size="sm" variant="secondary" onClick={onFixIt}>
            Fix it
          </Button>
        </Tooltip>
        <Button size="sm" variant="ghost" onClick={onIgnore}>
          Ignore
        </Button>
        <Button size="sm" variant="ghost" aria-expanded={escalating} onClick={() => setEscalating(!escalating)}>
          Escalate
        </Button>
      </div>
      {escalating ? (
        <section className="armada-not-ours__summary" aria-label="Summary for the owners">
          <pre className="armada-not-ours__lines">{summary}</pre>
          <CopyAct text={summary} label="Copy summary" />
        </section>
      ) : null}
    </Card>
  );
}
