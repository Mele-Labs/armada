// What Autopilot answered and he has not yet reviewed, in a HUD over the map, opened from the lamp on the Autopilot switch's plate.
// He goes down it: Agree keeps the answer, Change sends the Job his own words, and a row goes into its Job with Back coming back to that row.

import { useEffect, useRef, useState, useSyncExternalStore, type KeyboardEvent as ReactKeyboardEvent, type RefObject } from "react";
import { Check, ChevronRight, Pencil } from "lucide-react";
import { Button, Stamp, Textarea, Tooltip, forgetKept, isPressed, keyFor } from "@armada/components";
import type { AutopilotDecided } from "@armada/protocol";

import { correctable, useAutopilotDecided, useAutopilotDecisionsSource } from "../../autopilot-decisions";
import { ConsoleHud, panelOf } from "./ConsoleHud";
import { focusPanel } from "./keys";
import type { Ship } from "./ship";
import { deckNow, openWithBack, select, selOfSubject, useDeck } from "./store";
import "./autopilot-answers.css";

/** The strip the name stands in. */
const HEAD = 40;
const ROW = ".console-ap__row";

type Hud = { open: boolean; at: string | undefined; changing: string | undefined };
const CLOSED: Hud = { open: false, at: undefined, changing: undefined };
let hud: Hud = CLOSED;
/** Where his keys were before the HUD came up, so closing it puts them back. Kept while a Job opened from a row stands on the stage. */
let returnTo: HTMLElement | undefined;
const heard = new Set<() => void>();
const hear = (one: () => void) => (heard.add(one), () => void heard.delete(one));
const set = (next: Hud) => {
  hud = next;
  heard.forEach((one) => one());
};
export const useAnswersHud = (): Hud => useSyncExternalStore(hear, () => hud);
/** A window that remembers nothing, as `forgetDeck` starts one. */
export const forgetAnswers = () => {
  returnTo = undefined;
  set(CLOSED);
};

const plateEl = () => document.querySelector<HTMLElement>(".console-bay > .gs");
/** Read when the HUD is placed, so one opened as the Job closes finds the plate drawn in the same commit. */
const FACET: RefObject<Element | null> = {
  get current() {
    return plateEl();
  },
};
const rowEls = () => [...document.querySelectorAll<HTMLElement>(`[data-hud="autopilot-answers"] ${ROW}`)];

/** The HUD is put away, and his keys go back to where they were. */
export const closeAnswers = () => {
  const back = returnTo;
  returnTo = undefined;
  set(CLOSED);
  const lamp = plateEl()?.querySelector<HTMLElement>('[data-lamp="answers"]');
  (back !== undefined && back !== document.body && back.isConnected ? back : lamp)?.focus();
};
/** Another HUD takes the field: this one gives way, and keeps his keys where they are. */
export const putAnswersAway = () => {
  if (hud.open) closeAnswers();
};
/** Esc steps back one: an open box first, then the HUD. */
const leave = () => {
  const was = hud.changing;
  if (was === undefined) return closeAnswers();
  set({ ...hud, changing: undefined });
  rowEls().find((one) => one.dataset.key === was)?.focus();
};
export const openAnswers = () => {
  if (!hud.open && returnTo === undefined && document.activeElement instanceof HTMLElement) returnTo = document.activeElement;
  set({ open: true, at: hud.at, changing: undefined });
};
export const toggleAnswers = () => (hud.open ? closeAnswers() : showAnswers());
/** Up over the map, whatever the stage holds. */
export function showAnswers() {
  select(undefined);
  openAnswers();
}

/** Whether anything Autopilot answered waits on him: the lamp's light. */
export const useAnswersWaiting = (): boolean => useAutopilotDecided().length > 0;

/** The words he sends the Job. Enter sends them, and what was typed stays until it is sent. */
function ChangeBox({ row, onSend }: { row: AutopilotDecided; onSend: (words: string) => void }) {
  const [text, setText] = useState("");
  const keep = `autopilot:change:${row.id}`;
  const send = () => {
    if (text.trim() === "") return;
    forgetKept(keep);
    onSend(text.trim());
  };
  const onKeyDown = (event: ReactKeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key !== "Enter" || event.shiftKey) return;
    event.preventDefault();
    send();
  };
  return (
    <div className="console-ap__change">
      <Textarea aria-label="Change" rows={2} keep={keep} autoFocus value={text} onChange={(event) => setText(event.currentTarget.value)} onKeyDown={onKeyDown} />
      <Button variant="primary" size="sm" disabled={text.trim() === ""} onClick={send}>
        Send
      </Button>
    </div>
  );
}

type Acts = { agree: () => void; change: () => void; open: () => void };

function Row({ row, changing, acts, onSend }: { row: AutopilotDecided; changing: boolean; acts: Acts; onSend: (words: string) => void }) {
  return (
    <li className="console-ap__item">
      <div className="console-ap__row" role="group" aria-label={row.who} tabIndex={0} data-key={row.id}>
        <span className="console-ap__who">{row.who}</span>
        <span className="console-ap__asked">{row.asked}</span>
        <span className="console-ap__chose">{row.chose}</span>
        <span className="console-ap__because">{row.because}</span>
        <Stamp still first="age" at={row.at} className="console-ap__age" />
        <span className="console-ap__acts">
          <Tooltip label="Agree" shortcut="↵" asChild>
            <Button variant="ghost" size="sm" iconOnly aria-label="Agree" tabIndex={-1} onClick={acts.agree}>
              <Check size={14} aria-hidden="true" />
            </Button>
          </Tooltip>
          {correctable(row) ? (
            <Tooltip label="Change" shortcut={keyFor("call_reply")} asChild>
              <Button variant="ghost" size="sm" iconOnly aria-label="Change" tabIndex={-1} onClick={acts.change}>
                <Pencil size={14} aria-hidden="true" />
              </Button>
            </Tooltip>
          ) : (
            <span />
          )}
          {row.session_id === undefined ? (
            <span />
          ) : (
            <Tooltip label="Open the Job" shortcut={keyFor("open")} asChild>
              <Button variant="ghost" size="sm" iconOnly aria-label="Open the Job" tabIndex={-1} onClick={acts.open}>
                <ChevronRight size={14} aria-hidden="true" />
              </Button>
            </Tooltip>
          )}
        </span>
      </div>
      {changing ? <ChangeBox row={row} onSend={onSend} /> : null}
    </li>
  );
}

export function AutopilotAnswersHud({ ship, onOpenSession }: { ship: Ship; onOpenSession: (id: string) => void }) {
  const { open, at, changing } = useAnswersHud();
  const rows = useAutopilotDecided();
  const source = useAutopilotDecisionsSource();
  const sel = useDeck((deck) => deck.sel);
  const body = useRef<HTMLDivElement>(null);
  /** The row he was on, so the one that takes its place gets his keys when it leaves. */
  const place = useRef(0);
  const was = useRef(0);
  // A Job opened from the list while it is up takes the stage, and the HUD steps aside.
  useEffect(() => {
    if (open && sel !== undefined) set(CLOSED);
  }, [open, sel]);
  useEffect(() => () => set(CLOSED), []);
  // The keys land on the row he came back to, else the first.
  useEffect(() => {
    if (!open) return;
    const land = window.requestAnimationFrame(() => {
      const list = rowEls();
      (list.find((one) => one.dataset.key === at) ?? list[0] ?? document.querySelector<HTMLElement>('[data-hud="autopilot-answers"] .console-hud__panel'))?.focus();
    });
    return () => window.cancelAnimationFrame(land);
  }, [open, at]);
  // A row that left takes his keys with it: the one standing where it did gets them, and the last one leaving puts the HUD away.
  useEffect(() => {
    const before = was.current;
    was.current = rows.length;
    if (!open || rows.length >= before) return;
    if (rows.length === 0) return closeAnswers();
    const here = document.activeElement;
    if (here === null || here === document.body) rowEls()[Math.min(place.current, rows.length - 1)]?.focus();
  }, [open, rows]);

  const rowOf = (element: Element | null): HTMLElement | undefined => element?.closest<HTMLElement>(ROW) ?? undefined;
  const decided = (key: string | undefined) => rows.find((one) => one.id === key);
  const agree = (row: AutopilotDecided) => source?.review(row.id);
  const change = (row: AutopilotDecided) => correctable(row) && set({ ...hud, changing: row.id });
  const send = (row: AutopilotDecided, words: string) => {
    source?.override(row.id, words);
    set({ ...hud, changing: undefined });
  };
  const jump = (row: AutopilotDecided) => {
    const id = row.session_id;
    if (id === undefined) return;
    set({ open: false, at: row.id, changing: undefined });
    const held = ship.subjects.find((one) => one.kind === "session" && one.id === id);
    if (held === undefined) return onOpenSession(id);
    openWithBack(selOfSubject(held), openAnswers);
    window.requestAnimationFrame(() => focusPanel("stage", deckNow().cursor));
  };

  // His keys, taken here before the deck's. The deck stands down while a HUD is up, so none of them is also heard there.
  // A key that opened this HUD is never one of these acts: it arrives as a press on the lamp, and an auto-repeat is not a second act.
  const onKey = (event: KeyboardEvent): boolean => {
    if (event.metaKey || event.ctrlKey || event.altKey || (event.target instanceof Element && event.target.closest("textarea") !== null)) return false;
    const list = rowEls();
    // Where the press is aimed, which is where his keys are unless a walk sends it elsewhere.
    const from = event.target instanceof Element && event.target !== document.body ? event.target : document.activeElement;
    const current = rowOf(from) ?? rowOf(document.activeElement);
    const row = decided(current?.dataset.key);
    const take = (go: () => void): true => {
      event.preventDefault();
      event.stopPropagation();
      go();
      return true;
    };
    const step = event.key === "ArrowDown" || event.key === "j" ? 1 : event.key === "ArrowUp" || event.key === "k" ? -1 : 0;
    if (step !== 0 || event.key === "Home" || event.key === "End") {
      if (list.length === 0) return false;
      const to = event.key === "Home" ? 0 : event.key === "End" ? list.length - 1 : Math.min(list.length - 1, Math.max(0, (current === undefined ? (step > 0 ? -1 : list.length) : list.indexOf(current)) + step));
      return take(() => list[to]?.focus());
    }
    if (event.repeat || row === undefined) return false;
    // Enter on a button inside the row is the button's own press.
    if (event.key === "Enter") return from instanceof HTMLButtonElement ? false : take(() => agree(row));
    if (isPressed("call_reply", event)) return take(() => change(row));
    if (isPressed("open", event)) return take(() => jump(row));
    return false;
  };

  if (!open) return null;
  return (
    <ConsoleHud label="What Autopilot answered" name="What Autopilot answered" facet={FACET} onClose={leave} place={panelOf} head={HEAD} onKey={onKey} landsItself kind="autopilot-answers">
      {() => (
        <div
          ref={body}
          className="console-ap"
          onFocusCapture={(event) => {
            const at = rowEls().indexOf(rowOf(event.target as Element) as HTMLElement);
            if (at !== -1) place.current = at;
          }}
        >
          <div className="console-ap__head" aria-hidden="true">
            <span />
            <span>Asked</span>
            <span>Chose</span>
            <span>Because</span>
            <span />
            <span />
          </div>
          <ul className="console-ap__list" aria-label="Answers">
            {rows.map((row) => (
              <Row key={row.id} row={row} changing={changing === row.id} acts={{ agree: () => agree(row), change: () => change(row), open: () => jump(row) }} onSend={(words) => send(row, words)} />
            ))}
          </ul>
        </div>
      )}
    </ConsoleHud>
  );
}
