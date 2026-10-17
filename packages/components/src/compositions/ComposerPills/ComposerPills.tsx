import { useEffect, useRef } from "react";
import type { KeyboardEvent, ReactNode } from "react";
import { BellRing, ChevronDown, ChevronUp, MessageCircleQuestion } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { LedgerRows, waitingMark, type LedgerEntry } from "../SessionLedger/SessionLedger";

/** The two sheets the pills open. One is open at a time. */
export type PillSheet = "needs" | "question";

/** Focus may move to an arriving question only from nowhere, or from a composer with nothing typed in it. */
function focusIsFree(): boolean {
  const active = document.activeElement;
  if (active === null || active === document.body) return true;
  if (!(active instanceof HTMLElement) || active.closest(".armada-session-composer") === null) return false;
  const typed = active instanceof HTMLTextAreaElement || active instanceof HTMLInputElement ? active.value : (active.textContent ?? "");
  return typed.trim() === "";
}

/**
 * What waits on the person, as pills hovering just above the composer: Needs you, a glyph per item named on hover, and a question
 * when the agent has asked one. A pressed pill raises its sheet from above the row over the foot of the thread, and the row stays put.
 * It has no height of its own: the host places it in a box sitting on the composer and keeps the room clear.
 *
 * **A question that arrives opens its sheet once**, by its `id`, and moves focus into it only when that would not interrupt typing.
 * Needs you never opens by itself. `open` is the host's, so a ledger row can raise the question too.
 */
export function ComposerPills({
  rows,
  question,
  open,
  onOpen,
}: {
  rows: readonly LedgerEntry[];
  question?: { id: string; label: string; children: ReactNode };
  open: PillSheet | undefined;
  onOpen: (next: PillSheet | undefined) => void;
}) {
  const needsPill = useRef<HTMLButtonElement>(null);
  const questionPill = useRef<HTMLButtonElement>(null);
  const questionSheet = useRef<HTMLDivElement>(null);
  const has = { needs: rows.length > 0, question: question !== undefined };
  const shown = open !== undefined && has[open] ? open : undefined;
  // Questions already opened once, by id, so folding one away is not undone by the next render or by coming back to its Session.
  const opened = useRef(new Set<string>());
  const host = useRef(onOpen);
  host.current = onOpen;
  const id = question?.id;
  useEffect(() => {
    if (id === undefined || opened.current.has(id)) return;
    opened.current.add(id);
    host.current("question");
    if (!focusIsFree()) return;
    const sheet = questionSheet.current;
    window.requestAnimationFrame(() => (sheet?.querySelector<HTMLElement>('[role="radio"], [role="checkbox"]') ?? sheet?.querySelector<HTMLElement>("button, textarea, input"))?.focus());
  }, [id]);
  // A sheet whose content has gone is put away, so it does not come back open when something arrives.
  useEffect(() => {
    if (open !== undefined && shown === undefined) host.current(undefined);
  }, [open, shown]);
  if (!has.needs && !has.question) return null;
  const close = (event: KeyboardEvent) => {
    if (shown === undefined || event.key !== "Escape" || event.defaultPrevented) return;
    event.preventDefault();
    event.stopPropagation();
    onOpen(undefined);
    (shown === "needs" ? needsPill : questionPill).current?.focus();
  };
  const toggle = (sheet: PillSheet) => onOpen(shown === sheet ? undefined : sheet);
  const Fold = (sheet: PillSheet) => (shown === sheet ? ChevronDown : ChevronUp);
  const NeedsFold = Fold("needs");
  const QuestionFold = Fold("question");
  return (
    <section className="armada-composer-pills" aria-label="Needs you" onKeyDown={close} data-keeps-escape={shown === undefined ? undefined : ""}>
      <div className="armada-composer-pills__sheets">
        {!has.needs ? null : (
          <div className="armada-composer-pills__sheet" data-kind="needs" data-open={shown === "needs"}>
            <LedgerRows kind="waiting" Glyph={BellRing} rows={rows} />
          </div>
        )}
        {question === undefined ? null : (
          <div ref={questionSheet} className="armada-composer-pills__sheet" data-kind="question" data-open={shown === "question"}>
            {question.children}
          </div>
        )}
      </div>
      <div className="armada-composer-pills__row">
        {!has.needs ? null : (
          <button ref={needsPill} type="button" className="armada-composer-pills__pill" data-kind="needs" aria-expanded={shown === "needs"} onClick={() => toggle("needs")}>
            <BellRing size={12} strokeWidth={2} aria-hidden />
            <span className="armada-composer-pills__label">Needs you</span>
            <span className="armada-composer-pills__glyphs">
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
            <NeedsFold size={12} strokeWidth={2} aria-hidden />
          </button>
        )}
        {question === undefined ? null : (
          <button ref={questionPill} type="button" className="armada-composer-pills__pill" data-kind="question" aria-expanded={shown === "question"} onClick={() => toggle("question")}>
            <MessageCircleQuestion size={12} strokeWidth={2} aria-hidden />
            <span className="armada-composer-pills__label">{question.label}</span>
            <QuestionFold size={12} strokeWidth={2} aria-hidden />
          </button>
        )}
      </div>
    </section>
  );
}
