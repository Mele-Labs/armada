import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent, ReactNode } from "react";
import { BellRing, ChevronLeft, ChevronUp, MessageCircleQuestion, Scale, ThumbsDown, ThumbsUp } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { DropdownMenu } from "../../primitives/DropdownMenu/DropdownMenu";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { useEscapeFold } from "./useEscapeFold";
import { useScrimFold } from "./useScrimFold";
import { LedgerRows, RestartKey, waitingMark, type LedgerEntry } from "../SessionLedger/SessionLedger";

/** Focus may move to an arriving question from anywhere but a box with something typed in it: a row he just pressed to come into the Session is not one. */
function focusIsFree(): boolean {
  const active = document.activeElement;
  if (active === null || active === document.body) return true;
  if (!(active instanceof HTMLElement)) return false;
  if (active.closest(".armada-session-composer") === null && !active.matches(TYPES_WORDS)) return true;
  const typed = active instanceof HTMLTextAreaElement || active instanceof HTMLInputElement ? active.value : (active.textContent ?? "");
  return typed.trim() === "";
}

const TYPES_WORDS = "input, textarea, select, [contenteditable]";

/** What the arrows move between in the list: a question's row, and an action's one press. */
const LIST_KEYS = ".armada-needs-panel__list :is(.armada-session-ledger__open, [data-aside], .armada-prose__run-acts button)";

/** The actions, by what their one press does, in the order they are drawn. */
const ACTIONS: readonly { label: string; holds: (row: LedgerEntry) => boolean }[] = [
  { label: "Walks", holds: (row) => row.act === "walk" },
  { label: "Commands", holds: (row) => row.act === "run" || row.act === "steps" },
  { label: "PRs", holds: (row) => row.act === "approve_pr" },
];

const acts = (row: LedgerEntry): boolean => row.restart === undefined && row.options === undefined && ACTIONS.some((group) => group.holds(row));

/** A row that asks him something: choices, a reply, or a press that goes to a question or a message. A restart is the band's own key. */
const asks = (row: LedgerEntry): boolean => row.restart === undefined && !acts(row);

/** One question waiting, as the list draws it and the detail opens it. */
type Thing = { key: string; Glyph: LucideIcon; said: string; tip: string; text: ReactNode; name: string; row?: LedgerEntry; content?: ReactNode; press?: () => void; /** A sentence Fleet read from a reply: `no` is Not a question, `yes` is A question and is absent once Fleet has it confirmed. Both are thumbs at the row's end and items of its menu. */ misread?: { no: () => void; yes?: () => void } };

/** The two thumbs at a read line's end, in the list and opened. Confirmed, Fleet has nothing left to ask, so there are none. `keep` is where focus goes when A question takes the thumbs away. */
function Thumbs({ misread, keep }: { misread: NonNullable<Thing["misread"]>; keep: (thumb: HTMLElement) => HTMLElement | null }) {
  if (misread.yes === undefined) return null;
  return (
    <span className="armada-needs-panel__thumbs">
      <Tooltip label="A question">
        <Button variant="ghost" size="sm" iconOnly data-aside="" aria-label="A question" onClick={(event) => (keep(event.currentTarget)?.focus(), misread.yes?.())}>
          <ThumbsUp size={12} strokeWidth={2} aria-hidden />
        </Button>
      </Tooltip>
      <Tooltip label="Not a question">
        <Button variant="ghost" size="sm" iconOnly data-aside="" aria-label="Not a question" onClick={misread.no}>
          <ThumbsDown size={12} strokeWidth={2} aria-hidden />
        </Button>
      </Tooltip>
    </span>
  );
}

/** A question answered in the panel's place: the question, a judge's concern, or a row with choices or a reply box. The rest go where they point. */
const opensHere = (one: Thing): boolean => one.press === undefined && (one.row === undefined || one.row.options !== undefined || one.row.onReply !== undefined);

/**
 * Everything the agent needs from the person, in one panel joined to the composer. Folded it is a trigger on the composer's top edge, a glyph
 * per thing waiting named on hover, the questions' first, and a restart onto a build is its own key beside it, pressed folded or open.
 * Opened it is a list: Questions, then Walks, Commands and PRs. A press on a question opens it in the panel's place to be answered, and Back
 * returns to the list; anything else is done in one press where it stands. A lone question opens straight away, and Back folds the panel.
 * It has no height of its own: the host places it in a box sitting on the composer and keeps the room for the trigger clear.
 *
 * **A question that arrives opens the panel once**, by its `id`, and again each time he comes into its Session, and moves focus into it only when that would not interrupt
 * typing. A plan waiting for approval is not here: the thread's card shows it whole and takes Approve and Not yet. Needs you alone never opens it. `open` is the host's, so a ledger row can raise it too.
 */
export function NeedsYouPanel({
  rows,
  question,
  judges,
  open: wanted,
  onOpen,
}: {
  rows: readonly LedgerEntry[];
  /** `label` is the question's header, `text` the line its row reads, and `children` the card that answers it. */
  question?: { id: string; label: string; text: string; children: ReactNode };
  /** A judge's concerns, after the question and before the items: `text` is the claim its row reads, `children` the card that sends or dismisses it. */
  judges?: readonly { id: string; text: string; children: ReactNode }[];
  open: boolean;
  onOpen: (next: boolean) => void;
}) {
  const [menu, setMenu] = useState<{ key: string; x: number; y: number; from: HTMLElement | null }>();
  const section = useRef<HTMLElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const [at, setAt] = useState<string | undefined>();
  // Questions already opened once, by id, so folding one away is not undone by the next render. The host keys the panel by Session, so coming back to one starts it afresh and the question opens again.
  const opened = useRef(new Set<string>());
  // Where focus goes when the panel next draws its list: the first row, or the row just left.
  const landing = useRef<string | undefined>(undefined);
  // A question that opened the panel while he was typing leaves focus where it is.
  const quiet = useRef(false);
  const host = useRef(onOpen);
  host.current = onOpen;
  const misreadOf = (row: LedgerEntry): Pick<Thing, "misread"> => (row.onMisread === undefined ? {} : { misread: { no: row.onMisread, ...(row.confirmed === true || row.onConfirmed === undefined ? {} : { yes: row.onConfirmed }) } });
  const questions: Thing[] = [
    ...(question === undefined ? [] : [{ key: `question:${question.id}`, Glyph: MessageCircleQuestion, said: question.label, tip: question.label, text: question.text, name: question.text, content: question.children }]),
    ...(judges ?? []).map((one) => ({ key: `judge:${one.id}`, Glyph: Scale, said: "Judge", tip: "Judge", text: one.text, name: one.text, content: one.children })),
    ...rows.filter(asks).map((row) => ({ key: row.key, ...waitingMark(row), tip: row.name, text: row.text, name: row.name, row, ...misreadOf(row) })),
  ];
  const menuOf = menu === undefined ? undefined : questions.find((one) => one.key === menu.key)?.misread;
  const actions = ACTIONS.map((group) => ({ label: group.label, rows: rows.filter((row) => acts(row) && group.holds(row)) })).filter((group) => group.rows.length > 0);
  const restarts = rows.filter((row) => row.restart !== undefined);
  const things: Thing[] = [...questions, ...actions.flatMap((group) => group.rows.map((row) => ({ key: row.key, ...waitingMark(row), tip: row.name, text: row.text, name: row.name, row })))];
  // What opens the panel on arriving: the question.
  const arrived = [question?.id].filter((one) => one !== undefined).join("\n");
  // A band with nothing to list (a restart alone) is never drawn open: the host may still hold it open from an earlier arrival.
  const open = wanted && things.length > 0;
  const only = things.length === 1 && questions.length === 1 && opensHere(questions[0]!) ? questions[0]!.key : undefined;
  const shown = at ?? (open ? only : undefined);
  const current = shown === undefined ? undefined : questions.find((one) => one.key === shown);
  const listed = open && current === undefined;
  const focusRow = (key: string | undefined) => {
    const buttons = [...(section.current?.querySelectorAll<HTMLElement>(LIST_KEYS) ?? [])];
    (key === undefined ? buttons[0] : buttons.find((one) => one.dataset.thing === key))?.focus();
  };
  useEffect(() => {
    const fresh = arrived.split("\n").filter((one) => one !== "" && !opened.current.has(one));
    if (fresh.length === 0) return;
    for (const one of fresh) opened.current.add(one);
    host.current(true);
    if (focusIsFree()) landing.current = "";
    else quiet.current = true;
  }, [arrived]);
  // The panel put away, or the thing opened gone (answered, run, removed), returns to the list; a list with nothing in it folds the panel.
  useEffect(() => {
    if (!open || (at !== undefined && current === undefined)) setAt(undefined);
  }, [open, at, current]);
  const empty = things.length === 0;
  // Folded on the host too, where `open` already reads false because nothing waits: else the next thing to arrive finds it open.
  // Not on a mount that has never listed anything, or it would overwrite a host that already opens the panel for what waits (a delivery question).
  const hadRows = useRef(false);
  useEffect(() => {
    if (!empty) hadRows.current = true;
    if (empty && hadRows.current) host.current(false);
  }, [empty]);
  useEffect(() => {
    const body = section.current;
    if (quiet.current && open) {
      quiet.current = false;
      landing.current = undefined;
      return;
    }
    if (current !== undefined) {
      landing.current = undefined;
      // A detail with nothing to press or type in still takes the keys: focus goes to Back, so Esc and Backspace reach the panel.
      window.requestAnimationFrame(() => (body?.querySelector<HTMLElement>(".armada-needs-panel__content :is(input, textarea, [role='radio'], [role='checkbox'], button)") ?? body?.querySelector<HTMLElement>(".armada-needs-panel__detail button[aria-label='Back']"))?.focus());
    } else if (listed && landing.current !== undefined) {
      const to = landing.current === "" ? undefined : landing.current;
      landing.current = undefined;
      window.requestAnimationFrame(() => focusRow(to));
    }
  }, [current?.key, listed, open]);
  // Dimmed, the thread is not there to be pressed: a press on it folds the panel.
  useScrimFold(section, open, () => host.current(false));
  useEscapeFold(section, open && menu === undefined, () => host.current(false));
  if (empty && restarts.length === 0) return null;
  const fold = () => {
    onOpen(false);
    trigger.current?.focus();
  };
  const back = () => {
    if (only !== undefined) return fold();
    quiet.current = false;
    landing.current = at;
    setAt(undefined);
  };
  const choose = (key: string) => {
    quiet.current = false;
    setAt(key);
  };
  const key = (event: KeyboardEvent) => {
    if (!open || event.defaultPrevented || menu !== undefined) return;
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      if (current !== undefined) return back();
      fold();
    } else if (event.key === "Backspace" && current !== undefined && !(event.target instanceof Element && event.target.closest(TYPES_WORDS) !== null)) {
      event.preventDefault();
      back();
    } else if ((event.key === "ArrowDown" || event.key === "ArrowUp") && current === undefined) {
      const buttons = [...(section.current?.querySelectorAll<HTMLElement>(LIST_KEYS) ?? [])];
      const here = buttons.findIndex((one) => one === document.activeElement);
      if (here < 0) return;
      event.preventDefault();
      buttons[Math.min(buttons.length - 1, Math.max(0, here + (event.key === "ArrowDown" ? 1 : -1)))]?.focus();
    }
  };
  const press = () => {
    quiet.current = false;
    if (!open) landing.current = "";
    onOpen(!open);
  };
  return (
    <section ref={section} className="armada-needs-panel" aria-label="Needs you" data-open={open} onKeyDown={key} data-keeps-escape={open ? "" : undefined}>
      <div className="armada-needs-panel__band">
        <button ref={trigger} type="button" className="armada-needs-panel__trigger" aria-label="Needs you" aria-expanded={open} aria-disabled={empty || undefined} onClick={empty ? undefined : press}>
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
          {empty ? null : <ChevronUp className="armada-needs-panel__fold" size={12} strokeWidth={2} aria-hidden />}
        </button>
        {restarts.map((row) => (row.restart === undefined ? null : <RestartKey key={row.key} className="armada-needs-panel__restart" name={row.name} restart={row.restart} />))}
      </div>
      <div className="armada-needs-panel__reveal" data-open={open}>
        <div className="armada-needs-panel__shell">
          <div className="armada-needs-panel__body" data-open={open}>
            {current === undefined ? (
              <div className="armada-needs-panel__list">
                {[
                  { label: "Questions", rows: questions },
                ].map((group) => group.rows.length === 0 ? null : (
                  <section key={group.label} className="armada-session-ledger__group" aria-label={group.label}>
                    <h3 className="armada-session-ledger__eyebrow">{group.label}</h3>
                    <ul className="armada-session-ledger__rows">
                      {group.rows.map((one) => (
                        <li
                          key={one.key}
                          className="armada-session-ledger__row"
                          aria-label={one.name}
                          onContextMenu={one.misread === undefined ? undefined : (event) => (event.preventDefault(), setMenu({ key: one.key, x: event.clientX, y: event.clientY, from: event.currentTarget.querySelector<HTMLElement>(".armada-session-ledger__open") }))}
                          onKeyDown={one.misread === undefined ? undefined : (event) => {
                            if (event.key !== "ContextMenu" && !(event.key === "F10" && event.shiftKey)) return;
                            event.preventDefault();
                            const box = event.currentTarget.querySelector<HTMLElement>(".armada-session-ledger__open")?.getBoundingClientRect() ?? event.currentTarget.getBoundingClientRect();
                            setMenu({ key: one.key, x: box.left + box.width / 2, y: box.bottom, from: event.currentTarget.querySelector<HTMLElement>(".armada-session-ledger__open") });
                          }}
                        >
                          <button type="button" className="armada-session-ledger__open" aria-label={`Open ${one.name}`} data-thing={one.key} onClick={() => (opensHere(one) ? choose(one.key) : (one.press ?? one.row?.onOpen)?.())}>
                            <Tooltip label={one.said}>
                              <span className="armada-session-mark" role="img" aria-label={one.said}>
                                <one.Glyph size={12} strokeWidth={2} aria-hidden />
                              </span>
                            </Tooltip>
                            <span className="armada-session-ledger__text">{one.text}</span>
                          </button>
                          {one.misread === undefined ? null : <Thumbs misread={one.misread} keep={(thumb) => thumb.closest("li")?.querySelector<HTMLElement>(".armada-session-ledger__open") ?? null} />}
                        </li>
                      ))}
                    </ul>
                  </section>
                ))}
                {actions.map((group) => (
                  <section key={group.label} className="armada-session-ledger__group" aria-label={group.label}>
                    <h3 className="armada-session-ledger__eyebrow">{group.label}</h3>
                    <LedgerRows kind="waiting" Glyph={BellRing} rows={group.rows} />
                  </section>
                ))}
              </div>
            ) : (
              <div className="armada-needs-panel__detail">
                <div className="armada-needs-panel__head">
                  <Tooltip label="Back" shortcut="Esc">
                    <Button variant="ghost" size="sm" iconOnly aria-label="Back" onClick={back}>
                      <ChevronLeft size={16} aria-hidden />
                    </Button>
                  </Tooltip>
                  {current.misread === undefined ? null : <Thumbs misread={current.misread} keep={(thumb) => thumb.closest(".armada-needs-panel__head")?.querySelector<HTMLElement>("button[aria-label='Back']") ?? null} />}
                </div>
                <div className="armada-needs-panel__content">
                  {current.row === undefined ? current.content : <LedgerRows kind="waiting" Glyph={BellRing} rows={[current.row]} expanded />}
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
      {menu === undefined ? null : (
        <DropdownMenu
          triggerLabel="Question"
          at={{ x: menu.x, y: menu.y }}
          open
          entries={[
            ...(menuOf?.yes === undefined ? [] : [{ kind: "item" as const, id: "yes", label: "A question", icon: ThumbsUp }]),
            { kind: "item", id: "no", label: "Not a question", icon: ThumbsDown },
          ]}
          onOpenChange={(next) => {
            if (next) return;
            setMenu(undefined);
            menu.from?.focus();
          }}
          onSelect={(id) => (id === "yes" ? menuOf?.yes : menuOf?.no)?.()}
        />
      )}
    </section>
  );
}
