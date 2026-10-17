import { useEffect, useRef } from "react";
import type { KeyboardEvent, ReactNode } from "react";
import { BellRing, ChevronDown, ChevronUp, MessageCircleQuestion } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { LedgerRows, waitingMark, type LedgerEntry } from "../SessionLedger/SessionLedger";

/** Mock scaffolding, dropped once one is chosen: a tab on the composer's edge that becomes the panel's folder tab, or a band on the box with the panel above it. */
export type NeedsYouTake = "eyebrow" | "band";

/** Focus may move to an arriving question only from nowhere, or from a composer with nothing typed in it. */
function focusIsFree(): boolean {
  const active = document.activeElement;
  if (active === null || active === document.body) return true;
  if (!(active instanceof HTMLElement) || active.closest(".armada-session-composer") === null) return false;
  const typed = active instanceof HTMLTextAreaElement || active instanceof HTMLInputElement ? active.value : (active.textContent ?? "");
  return typed.trim() === "";
}

/**
 * Everything the agent needs from the person, in one panel joined to the composer: the question first, then what the Session waits on.
 * Folded it is a trigger on the composer's top edge, a glyph per thing waiting named on hover, the question's first. Opened, the panel
 * rises from the composer and shares its outline, which is what shows it belongs to the trigger. It has no height of its own: the host
 * places it in a box sitting on the composer and keeps the room for the trigger clear.
 *
 * **A question that arrives opens the panel once**, by its `id`, and moves focus into it only when that would not interrupt typing.
 * Needs you alone never opens it. `open` is the host's, so a ledger row can raise it too.
 */
export function NeedsYouPanel({
  rows,
  question,
  open,
  onOpen,
  take = "eyebrow",
}: {
  rows: readonly LedgerEntry[];
  question?: { id: string; label: string; children: ReactNode };
  open: boolean;
  onOpen: (next: boolean) => void;
  take?: NeedsYouTake;
}) {
  const trigger = useRef<HTMLButtonElement>(null);
  const panel = useRef<HTMLDivElement>(null);
  // Questions already opened once, by id, so folding one away is not undone by the next render or by coming back to its Session.
  const opened = useRef(new Set<string>());
  const host = useRef(onOpen);
  host.current = onOpen;
  const id = question?.id;
  const asking = question !== undefined;
  useEffect(() => {
    if (id === undefined || opened.current.has(id)) return;
    opened.current.add(id);
    host.current(true);
    if (!focusIsFree()) return;
    const body = panel.current;
    window.requestAnimationFrame(() => (body?.querySelector<HTMLElement>('[role="radio"], [role="checkbox"]') ?? body?.querySelector<HTMLElement>("button, textarea, input"))?.focus());
  }, [id]);
  // An answered question puts the panel away. The rows empty for a moment while an item runs, and the panel must still be up when it settles.
  const was = useRef(asking);
  useEffect(() => {
    if (was.current && !asking) host.current(false);
    was.current = asking;
  }, [asking]);
  if (rows.length === 0 && question === undefined) return null;
  const shown = open;
  const close = (event: KeyboardEvent) => {
    if (!shown || event.key !== "Escape" || event.defaultPrevented) return;
    event.preventDefault();
    event.stopPropagation();
    onOpen(false);
    trigger.current?.focus();
  };
  const Fold = shown ? ChevronDown : ChevronUp;
  return (
    <section className="armada-needs-panel" aria-label="Needs you" data-take={take} data-open={shown} onKeyDown={close} data-keeps-escape={shown ? "" : undefined}>
      <button ref={trigger} type="button" className="armada-needs-panel__trigger" aria-label="Needs you" aria-expanded={shown} onClick={() => onOpen(!shown)}>
        <BellRing size={12} strokeWidth={2} aria-hidden />
        <span className="armada-needs-panel__label">Needs you</span>
        <span className="armada-needs-panel__glyphs">
          {question === undefined ? null : (
            <Tooltip label={question.label}>
              <span className="armada-session-mark" role="img" aria-label={question.label}>
                <MessageCircleQuestion size={12} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
          )}
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
        <Fold className="armada-needs-panel__fold" size={12} strokeWidth={2} aria-hidden />
      </button>
      <div ref={panel} className="armada-needs-panel__body" data-open={shown}>
        {question === undefined ? null : <div className="armada-needs-panel__question">{question.children}</div>}
        {rows.length === 0 ? null : (
          <div className="armada-needs-panel__rows">
            <LedgerRows kind="waiting" Glyph={BellRing} rows={rows} />
          </div>
        )}
      </div>
    </section>
  );
}
