import { useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import { BellRing, ChevronDown, ChevronUp } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { LedgerRows, waitingMark, type LedgerEntry } from "../SessionLedger/SessionLedger";

/**
 * What the Session waits on the person for, as a tab on the composer's top edge: a glyph per item, named on hover.
 * Pressed, a sheet rises from that edge over the foot of the thread and holds the rows; the tab rides its top edge.
 * It draws up out of the edge and has no height of its own. The host places it inside a box that sits on the composer.
 */
export function NeedsYouEyebrow({ rows }: { rows: readonly LedgerEntry[] }) {
  const [open, setOpen] = useState(false);
  const press = useRef<HTMLButtonElement>(null);
  const Fold = open ? ChevronDown : ChevronUp;
  if (rows.length === 0) return null;
  const close = (event: KeyboardEvent) => {
    if (!open || event.key !== "Escape" || event.defaultPrevented) return;
    event.preventDefault();
    event.stopPropagation();
    setOpen(false);
    press.current?.focus();
  };
  return (
    <section className="armada-needs-eyebrow" aria-label="Needs you" onKeyDown={close} data-keeps-escape={open ? "" : undefined}>
      <button ref={press} type="button" className="armada-needs-eyebrow__tab" aria-expanded={open} onClick={() => setOpen(!open)}>
        <BellRing size={12} strokeWidth={2} aria-hidden />
        <span className="armada-needs-eyebrow__label">Needs you</span>
        <span className="armada-needs-eyebrow__glyphs">
          {rows.map((row) => {
            const { Glyph } = waitingMark(row);
            return (
              <Tooltip key={row.key} label={row.name}>
                <span className="armada-session-mark" role="img" aria-label={row.name}>
                  <Glyph size={12} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
            );
          })}
        </span>
        <Fold size={12} strokeWidth={2} aria-hidden />
      </button>
      <div className="armada-needs-eyebrow__sheet" data-open={open}>
        <LedgerRows kind="waiting" Glyph={BellRing} rows={rows} />
      </div>
    </section>
  );
}
