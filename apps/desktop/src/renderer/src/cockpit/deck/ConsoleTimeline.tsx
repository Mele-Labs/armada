// The timeline: what is stuck, and what came of each Job since he last closed it, in a HUD over the map. Closing it is what
// marks the list read, so nothing goes unseen; going into a Job from a row is not closing it, and Back comes back to that row.

import { useEffect, useMemo, useRef, useSyncExternalStore, type RefObject } from "react";
import { ChevronRight } from "lucide-react";
import { Stamp, Tooltip, isPressed, timeSaid, useTimeFormat } from "@armada/components";
import type { MergeLineView } from "@armada/screens";

import { useSessions } from "../../sessions-draft";
import { useDrawnEvents } from "../drawn-events";
import { ConsoleHud, panelOf } from "./ConsoleHud";
import { focusPanel } from "./keys";
import type { Ship } from "./ship";
import { deckNow, openWithBack, select, selOfSubject, useDeck } from "./store";
import { movedRows, stuckRows, type HudRow } from "./timeline-rows";

/** The strip the name stands in. */
const HEAD = 40;
/** How far back the list reaches the first time, with nothing kept. */
const FIRST_READ = 2 * 60 * 60_000;
const SINCE_KEY = "armada.bridge.timeline-since";

type Hud = { open: boolean; at: string | undefined };
const CLOSED: Hud = { open: false, at: undefined };
let hud: Hud = CLOSED;
let since: number | undefined;
const heard = new Set<() => void>();
const tell = () => heard.forEach((one) => one());
const hear = (one: () => void) => (heard.add(one), () => void heard.delete(one));
const set = (next: Hud) => {
  hud = next;
  tell();
};
/** When he last closed the HUD, or how far the list reaches the first time. */
export const timelineSince = (): number => {
  if (since !== undefined) return since;
  try {
    const kept = Number(window.localStorage.getItem(SINCE_KEY));
    since = kept > 0 ? kept : Date.now() - FIRST_READ;
  } catch {
    since = Date.now() - FIRST_READ;
  }
  return since;
};
const useSince = (): number => useSyncExternalStore(hear, timelineSince);
export const useTimelineHud = (): Hud => useSyncExternalStore(hear, () => hud);
/** A window that remembers nothing, as `forgetDeck` starts one: what is kept is read again. */
export const forgetTimeline = () => {
  since = undefined;
  set(CLOSED);
};

const facetEl = () => document.querySelector<HTMLElement>('.console-bay .deck-tile[data-tile="timeline"]');
/** Read when the HUD is placed, so one opened as the Job closes finds the key drawn in the same commit. */
const FACET: RefObject<Element | null> = {
  get current() {
    return facetEl();
  },
};

const closeTimeline = () => {
  since = Date.now();
  try {
    window.localStorage.setItem(SINCE_KEY, String(since));
  } catch {
    // Unremembered, so the list reaches back further next time, which loses nothing.
  }
  set(CLOSED);
  facetEl()?.focus();
};
export const openTimeline = () => set({ open: true, at: hud.at });
export const toggleTimeline = () => (hud.open ? closeTimeline() : openTimeline());
/** Up over the map, whatever the stage holds. */
export const showTimeline = () => {
  select(undefined);
  openTimeline();
};

function useRows(ship: Ship, views: readonly MergeLineView[], from: number): { stuck: HudRow[]; moved: HudRow[] } {
  const events = useDrawnEvents(views);
  const sessions = useSessions();
  const format = useTimeFormat();
  return useMemo(() => {
    const stuck = stuckRows(ship.subjects, sessions, events);
    return { stuck, moved: movedRows(events, from, stuck, ship.subjects, (at) => timeSaid(at, format)) };
  }, [ship.subjects, sessions, events, from, format]);
}

/** The key's lamp: something came in since he last closed the HUD. */
export function useTimelineLamp(ship: Ship, views: readonly MergeLineView[]): boolean {
  const from = useSince();
  const { stuck, moved } = useRows(ship, views, from);
  return moved.length > 0 || stuck.some((one) => one.at !== undefined && Date.parse(one.at) >= from);
}

function Row({ row, onOpen }: { row: HudRow; onOpen: (row: HudRow) => void }) {
  const inside = (
    <>
      <Tooltip label={row.said}>
        <span className="console-tl__glyph">
          <row.Glyph size={14} aria-hidden="true" />
        </span>
      </Tooltip>
      <span className="console-tl__title">{row.title}</span>
      <span className="console-tl__fact">{row.fact}</span>
      {row.at === undefined ? <span /> : <Stamp still first="age" at={row.at} className="console-tl__age" />}
      {row.opens === undefined ? <span /> : <ChevronRight size={14} className="console-tl__go" aria-hidden="true" />}
    </>
  );
  return (
    <li>
      {row.opens === undefined ? (
        <div className="console-tl__row" data-tone={row.tone}>
          {inside}
        </div>
      ) : (
        <button type="button" className="console-tl__row" data-tone={row.tone} data-key={row.key} onClick={() => onOpen(row)}>
          {inside}
        </button>
      )}
    </li>
  );
}

export function TimelineHud({ ship, views }: { ship: Ship; views: readonly MergeLineView[] }) {
  const { open, at } = useTimelineHud();
  const sel = useDeck((deck) => deck.sel);
  const from = useSince();
  const format = useTimeFormat();
  const { stuck, moved } = useRows(ship, views, from);
  const body = useRef<HTMLDivElement>(null);
  // A Job opened from the list while it is up takes the stage, and the HUD steps aside unread.
  useEffect(() => {
    if (open && sel !== undefined) set(CLOSED);
  }, [open, sel]);
  useEffect(() => () => set(CLOSED), []);
  // The keys land on the row he came back to, else the first.
  useEffect(() => {
    if (!open) return;
    const land = window.requestAnimationFrame(() => {
      const rows = [...(body.current?.querySelectorAll<HTMLElement>("button.console-tl__row") ?? [])];
      (rows.find((one) => one.dataset.key === at) ?? rows[0] ?? document.querySelector<HTMLElement>('[data-hud="timeline"] .console-hud__panel'))?.focus();
    });
    return () => window.cancelAnimationFrame(land);
  }, [open, at]);
  const onKey = (event: KeyboardEvent): boolean => {
    // The deck stands down while a HUD is up, so `t` is heard here: it closes what it opened. Taken already is the press that opened it.
    if (!event.defaultPrevented && isPressed("deck_timeline", event)) {
      event.preventDefault();
      event.stopPropagation();
      closeTimeline();
      return true;
    }
    // The press, taken here as the deck takes its own, so a walk playing does not also step on it.
    if (event.key === "Enter" && document.activeElement instanceof HTMLElement && document.activeElement.matches("button.console-tl__row")) {
      event.preventDefault();
      event.stopPropagation();
      document.activeElement.click();
      return true;
    }
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp" && event.key !== "Home" && event.key !== "End") return false;
    const rows = [...(body.current?.querySelectorAll<HTMLElement>("button.console-tl__row") ?? [])];
    if (rows.length === 0) return false;
    const was = rows.indexOf(document.activeElement as HTMLElement);
    const next = event.key === "Home" ? 0 : event.key === "End" ? rows.length - 1 : event.key === "ArrowDown" ? Math.min(was + 1, rows.length - 1) : Math.max(was - 1, 0);
    event.preventDefault();
    event.stopPropagation();
    rows[next]?.focus();
    return true;
  };
  const jump = (row: HudRow) => {
    if (row.opens === undefined) return;
    set({ open: false, at: row.key });
    openWithBack(selOfSubject(row.opens), openTimeline);
    window.requestAnimationFrame(() => focusPanel("stage", deckNow().cursor));
  };
  if (!open) return null;
  return (
    <ConsoleHud label="Timeline" name={`Since ${timeSaid(from, format)}`} facet={FACET} onClose={closeTimeline} place={panelOf} head={HEAD} onKey={onKey} landsItself kind="timeline">
      {() => (
        <div ref={body} className="console-tl">
          {stuck.length === 0 ? null : (
            <ul className="console-tl__list" data-stuck aria-label="Stuck">
              {stuck.map((row) => (
                <Row key={row.key} row={row} onOpen={jump} />
              ))}
            </ul>
          )}
          {moved.length === 0 ? null : (
            <ul className="console-tl__list" aria-label={`Since ${timeSaid(from, format)}`}>
              {moved.map((row) => (
                <Row key={row.key} row={row} onOpen={jump} />
              ))}
            </ul>
          )}
        </div>
      )}
    </ConsoleHud>
  );
}
