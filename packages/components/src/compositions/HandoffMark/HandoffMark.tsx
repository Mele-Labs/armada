import { Layers, LoaderCircle, X } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * A Job that asked to carry on in a new one and was told yes, as one glyph on its row. Coming up it turns; if the new Job
 * did not start the Job is as it was and this stands still. **A glyph and its tooltip, no phrase.**
 */
export function HandoffMark({ failed = false }: { failed?: boolean }) {
  const said = failed ? "Did not start" : "Handing off";
  const Glyph = failed ? X : LoaderCircle;
  return (
    <Tooltip label={said}>
      <span className="armada-handoff-mark" role="img" aria-label={said} data-failed={failed || undefined}>
        <Glyph size={14} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** The earlier Job behind a row that took its place: a press opens it. Beside the row, as the clash mark is. */
export function EarlierMark({ onOpen }: { onOpen: () => void }) {
  return (
    <Tooltip label="Earlier Job" asChild>
      <button type="button" className="armada-earlier-mark" aria-label="Earlier Job" onClick={onOpen}>
        <Layers size={12} strokeWidth={2} aria-hidden />
      </button>
    </Tooltip>
  );
}
