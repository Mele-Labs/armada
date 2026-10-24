import { useId, useLayoutEffect, useRef, useState } from "react";

import { Button } from "../../primitives/Button/Button";

/** What a claim is read from: a judge's concern, or an adversary's finding. `produced` is what was found. */
export type Claim = { text: string; expected?: string; produced?: string; consequence?: string; cites?: readonly string[] };

/**
 * A claim to read: the consequence over everything, then what was expected, what was found and what it cites, each a label
 * and its value in one shared pair of columns, so the values start on one line. A value is clamped to two lines with a More
 * press for the rest. A claim kept without its parts is its `text`, a line to a paragraph, with its cites under it.
 */
export function ConcernClaim({ concern }: { concern: Claim }) {
  const { expected, produced, consequence, cites } = concern;
  if (expected === undefined && produced === undefined && consequence === undefined) {
    return (
      <div className="armada-session-thread__claim">
        {concern.text.split("\n").filter((line) => line.trim() !== "").map((line, at) => (
          <p key={at} className="armada-session-thread__claim-text">{line}</p>
        ))}
        <ConcernCites cites={cites} />
      </div>
    );
  }
  return (
    <div className="armada-session-thread__claim">
      {consequence === undefined ? null : <p className="armada-session-thread__claim-head">{consequence}</p>}
      {expected === undefined ? null : <ClaimRow label="Expected" text={expected} />}
      {produced === undefined ? null : <ClaimRow label="Found" text={produced} />}
      {cites === undefined || cites.length === 0 ? null : (
        <div className="armada-session-thread__claim-row">
          <span className="armada-session-thread__claim-label">Cites</span>
          <ConcernCites cites={cites} />
        </div>
      )}
    </div>
  );
}

function ClaimRow({ label, text }: { label: string; text: string }) {
  const value = useRef<HTMLParagraphElement>(null);
  const [open, setOpen] = useState(false);
  const [clamped, setClamped] = useState(false);
  const id = useId();
  // More is offered only where the two lines leave words out: measured while it is closed, kept while it is open.
  useLayoutEffect(() => {
    const one = value.current;
    if (one === null || open) return;
    const measure = () => setClamped(one.scrollHeight > one.clientHeight + 1);
    measure();
    const watch = new ResizeObserver(measure);
    watch.observe(one);
    return () => watch.disconnect();
  }, [text, open]);
  return (
    <div className="armada-session-thread__claim-row">
      <span className="armada-session-thread__claim-label">{label}</span>
      <p ref={value} id={id} className="armada-session-thread__claim-text" data-open={open || undefined}>{text}</p>
      {clamped || open ? (
        <Button size="sm" variant="ghost" aria-expanded={open} aria-controls={id} aria-label={`${open ? "Less" : "More"} of ${label.toLowerCase()}`} onClick={() => setOpen(!open)}>
          {open ? "Less" : "More"}
        </Button>
      ) : null}
    </div>
  );
}

/** What a claim cites, as paths with a `:line` where one names it. */
function ConcernCites({ cites }: { cites: readonly string[] | undefined }) {
  if (cites === undefined || cites.length === 0) return null;
  return (
    <ul className="armada-session-thread__cites" aria-label="Cites">
      {cites.map((path) => (
        <li key={path}>
          <code title={path}>{path}</code>
        </li>
      ))}
    </ul>
  );
}
