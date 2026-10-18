import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent, ReactNode } from "react";
import { BellRing, ChevronDown, ChevronLeft, ChevronUp, MessageCircleQuestion, Scale } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { useScrimFold } from "./useScrimFold";
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

const TYPES_WORDS = "input, textarea, select, [contenteditable]";

/** One thing waiting, as the list draws it and the detail opens it. */
type Thing = { key: string; Glyph: LucideIcon; said: string; tip: string; text: ReactNode; name: string; row?: LedgerEntry; content?: ReactNode };

/**
 * Everything the agent needs from the person, in one panel joined to the composer. Folded it is a trigger on the composer's top edge, a glyph
 * per thing waiting named on hover, the question's first. Opened it is a list, one row to a thing, the questions first; a press on a row opens
 * that thing in the panel's place to be answered or addressed, and Back returns to the list. It never shows them all at once.
 * It has no height of its own: the host places it in a box sitting on the composer and keeps the room for the trigger clear.
 *
 * **A question that arrives opens the panel once**, by its `id`, to the list, and moves focus to its first row only when that would not
 * interrupt typing. Needs you alone never opens it. `open` is the host's, so a ledger row can raise it too.
 */
export function NeedsYouPanel({
  rows,
  question,
  judges,
  open,
  onOpen,
  take = "eyebrow",
}: {
  rows: readonly LedgerEntry[];
  /** `label` is the question's header, `text` the line its row reads, and `children` the card that answers it. */
  question?: { id: string; label: string; text: string; children: ReactNode };
  /** A judge's concerns, after the question and before the items: `text` is the claim its row reads, `children` the card that sends or dismisses it. */
  judges?: readonly { id: string; text: string; children: ReactNode }[];
  open: boolean;
  onOpen: (next: boolean) => void;
  take?: NeedsYouTake;
}) {
  const section = useRef<HTMLElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const [at, setAt] = useState<string | undefined>();
  // Questions already opened once, by id, so folding one away is not undone by the next render or by coming back to its Session.
  const opened = useRef(new Set<string>());
  // Where focus goes when the panel next draws its list: the first row, or the row just left.
  const landing = useRef<string | undefined>(undefined);
  const host = useRef(onOpen);
  host.current = onOpen;
  const id = question?.id;
  const things: Thing[] = [
    ...(question === undefined ? [] : [{ key: `question:${question.id}`, Glyph: MessageCircleQuestion, said: question.label, tip: question.label, text: question.text, name: question.text, content: question.children }]),
    ...(judges ?? []).map((one) => ({ key: `judge:${one.id}`, Glyph: Scale, said: "Judge", tip: "Judge", text: one.text, name: one.text, content: one.children })),
    ...rows.map((row) => ({ key: row.key, ...waitingMark(row), tip: row.name, text: row.text, name: row.name, row })),
  ];
  const current = at === undefined ? undefined : things.find((one) => one.key === at);
  const listed = open && current === undefined;
  const focusRow = (key: string | undefined) => {
    const buttons = [...(section.current?.querySelectorAll<HTMLElement>(".armada-needs-panel__list button") ?? [])];
    (key === undefined ? buttons[0] : buttons.find((one) => one.dataset.thing === key))?.focus();
  };
  useEffect(() => {
    if (id === undefined || opened.current.has(id)) return;
    opened.current.add(id);
    host.current(true);
    if (focusIsFree()) landing.current = "";
  }, [id]);
  // The panel put away, or the thing opened gone (answered, run, removed), returns to the list; a list with nothing in it folds the panel.
  useEffect(() => {
    if (!open || (at !== undefined && current === undefined)) setAt(undefined);
  }, [open, at, current]);
  const nothing = things.length === 0;
  useEffect(() => {
    if (nothing && open) host.current(false);
  }, [nothing, open]);
  useEffect(() => {
    const body = section.current;
    if (current !== undefined) {
      window.requestAnimationFrame(() => body?.querySelector<HTMLElement>(".armada-needs-panel__content :is(input, textarea, [role='radio'], [role='checkbox'], button)")?.focus());
    } else if (listed && landing.current !== undefined) {
      const to = landing.current === "" ? undefined : landing.current;
      landing.current = undefined;
      window.requestAnimationFrame(() => focusRow(to));
    }
  }, [current?.key, listed]);
  // Dimmed, the thread is not there to be pressed: a press on it folds the panel. The band is the only take that dims it.
  useScrimFold(section, open && take === "band", () => host.current(false));
  if (nothing) return null;
  const back = () => {
    landing.current = at;
    setAt(undefined);
  };
  const key = (event: KeyboardEvent) => {
    if (!open || event.defaultPrevented) return;
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      if (current !== undefined) return back();
      onOpen(false);
      trigger.current?.focus();
    } else if (event.key === "Backspace" && current !== undefined && !(event.target instanceof Element && event.target.closest(TYPES_WORDS) !== null)) {
      event.preventDefault();
      back();
    } else if ((event.key === "ArrowDown" || event.key === "ArrowUp") && current === undefined) {
      const buttons = [...(section.current?.querySelectorAll<HTMLElement>(".armada-needs-panel__list button") ?? [])];
      const here = buttons.findIndex((one) => one === document.activeElement);
      if (here < 0) return;
      event.preventDefault();
      buttons[Math.min(buttons.length - 1, Math.max(0, here + (event.key === "ArrowDown" ? 1 : -1)))]?.focus();
    }
  };
  // The band turns its one chevron, and the tab swaps it.
  const Fold = take === "band" || !open ? ChevronUp : ChevronDown;
  const press = () => {
    if (!open) landing.current = "";
    onOpen(!open);
  };
  return (
    <section ref={section} className="armada-needs-panel" aria-label="Needs you" data-take={take} data-open={open} onKeyDown={key} data-keeps-escape={open ? "" : undefined}>
      <button ref={trigger} type="button" className="armada-needs-panel__trigger" aria-label="Needs you" aria-expanded={open} onClick={press}>
        <BellRing size={12} strokeWidth={2} aria-hidden />
        <span className="armada-needs-panel__label">Needs you</span>
        <span className="armada-needs-panel__glyphs">
          {things.map((one) => (
            <Tooltip key={one.key} label={one.tip}>
              <span className="armada-session-mark" role="img" aria-label={one.tip}>
                <one.Glyph size={12} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
          ))}
        </span>
        <Fold className="armada-needs-panel__fold" size={12} strokeWidth={2} aria-hidden />
      </button>
      <div className="armada-needs-panel__reveal" data-open={open}>
        <div className="armada-needs-panel__shell">
          <div className="armada-needs-panel__body" data-open={open}>
            {current === undefined ? (
              <ul className="armada-session-ledger__rows armada-needs-panel__list">
                {things.map((one) => (
                  <li key={one.key} className="armada-session-ledger__row" aria-label={one.name}>
                    <button type="button" className="armada-session-ledger__open" aria-label={`Open ${one.name}`} data-thing={one.key} onClick={() => setAt(one.key)}>
                      <Tooltip label={one.said}>
                        <span className="armada-session-mark" role="img" aria-label={one.said}>
                          <one.Glyph size={12} strokeWidth={2} aria-hidden />
                        </span>
                      </Tooltip>
                      <span className="armada-session-ledger__text">{one.text}</span>
                    </button>
                  </li>
                ))}
              </ul>
            ) : (
              <div className="armada-needs-panel__detail">
                <Tooltip label="Back" shortcut="Esc">
                  <Button variant="ghost" size="sm" iconOnly aria-label="Back" onClick={back}>
                    <ChevronLeft size={16} aria-hidden />
                  </Button>
                </Tooltip>
                <div className="armada-needs-panel__content">
                  {current.row === undefined ? current.content : <LedgerRows kind="waiting" Glyph={BellRing} rows={[current.row]} expanded />}
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </section>
  );
}
