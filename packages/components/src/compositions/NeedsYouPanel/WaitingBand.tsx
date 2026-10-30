import { useRef } from "react";
import type { KeyboardEvent } from "react";
import { ChevronUp, CircleDot, GitMerge, Radar, type LucideIcon } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { useEscapeFold } from "./useEscapeFold";
import { useScrimFold } from "./useScrimFold";

/** One thing a Job waits on that is not him: a watch, its pull request in the queue or set to merge, or its pull request's Checks running. */
export type WaitingLine = { kind: "watch" | "merge" | "checks"; said: string };

const GLYPH: Record<WaitingLine["kind"], LucideIcon> = { watch: Radar, merge: GitMerge, checks: CircleDot };

/**
 * A Job waiting on something that is not him, as a band on the composer in the waiting hue, drawn as the Needs you band is: a glyph for each
 * thing it waits on, named on hover, and pressed it rides up as the header of a panel with a row for each. Esc and a press on the dimmed thread
 * fold it. It shares the panel's styles, so the band and the body move the same way.
 */
export function WaitingBand({ waits, open, onOpen }: { waits: readonly WaitingLine[]; open: boolean; onOpen: (next: boolean) => void }) {
  const section = useRef<HTMLElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  useScrimFold(section, open, () => onOpen(false));
  useEscapeFold(section, open, () => onOpen(false));
  const close = (event: KeyboardEvent) => {
    if (!open || event.key !== "Escape" || event.defaultPrevented) return;
    event.preventDefault();
    event.stopPropagation();
    onOpen(false);
    trigger.current?.focus();
  };
  return (
    <section ref={section} className="armada-needs-panel" aria-label="Waiting" data-kind="waiting" data-open={open} onKeyDown={close} data-keeps-escape={open ? "" : undefined}>
      <div className="armada-needs-panel__band">
        <button ref={trigger} type="button" className="armada-needs-panel__trigger" aria-label="Waiting" aria-expanded={open} onClick={() => onOpen(!open)}>
          <Radar size={12} strokeWidth={2} aria-hidden />
          <span className="armada-needs-panel__label">Waiting</span>
          <span className="armada-needs-panel__glyphs">
            {waits.map((one) => {
              const Glyph = GLYPH[one.kind];
              return (
                <Tooltip key={one.said} label={one.said}>
                  <span className="armada-session-mark" role="img" aria-label={one.said}>
                    <Glyph size={12} strokeWidth={2} aria-hidden />
                  </span>
                </Tooltip>
              );
            })}
          </span>
          <ChevronUp className="armada-needs-panel__fold" size={12} strokeWidth={2} aria-hidden />
        </button>
      </div>
      <div className="armada-needs-panel__reveal" data-open={open}>
        <div className="armada-needs-panel__shell">
          <div className="armada-needs-panel__body" data-open={open}>
            <ul className="armada-session-ledger__rows" aria-label="Waiting on">
              {waits.map((one) => {
                const Glyph = GLYPH[one.kind];
                return (
                  <li key={one.said} className="armada-session-ledger__row" aria-label={one.said}>
                    <span className="armada-session-ledger__open">
                      <span className="armada-session-mark" role="img" aria-hidden>
                        <Glyph size={12} strokeWidth={2} aria-hidden />
                      </span>
                      <span className="armada-session-ledger__text">{one.said}</span>
                    </span>
                  </li>
                );
              })}
            </ul>
          </div>
        </div>
      </div>
    </section>
  );
}
