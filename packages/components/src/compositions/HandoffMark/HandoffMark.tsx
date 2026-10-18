import { Layers, LoaderCircle } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/** A Job that asked to carry on in a new one and was told yes, as one turning ring on its row until the new Job is up. A glyph and its tooltip, no phrase. */
export function HandoffMark() {
  return (
    <Tooltip label="Handing off">
      <span className="armada-handoff-mark" role="img" aria-label="Handing off">
        <LoaderCircle size={14} strokeWidth={2} aria-hidden />
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
