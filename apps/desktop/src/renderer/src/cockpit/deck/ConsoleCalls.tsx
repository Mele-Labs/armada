// The Calls HUD. The panels in it are the Job's own: `SessionPane` for the thread and its Needs you card, `StoryColumn` for Now. What is
// here is which Job they show, its summary where it has one, and what moves him to the next: the order is the Jobs list's.

import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  useSyncExternalStore,
  type ReactNode,
} from "react";
import { Button, Kbd } from "@armada/components";
import { holdsText } from "@armada/screens/src/keys";

import { deckNow } from "./store";
import { ConsoleHud, type Box } from "./ConsoleHud";

type Hud = {
  open: boolean;
  key: string | undefined;
};
const CLOSED: Hud = {
  open: false,
  key: undefined,
};
let hud: Hud = CLOSED;
const heard = new Set<() => void>();
const set = (next: Hud) => {
  hud = next;
  heard.forEach((one) => one());
};
export const useCallsHud = (): Hud =>
  useSyncExternalStore(
    (hear) => (heard.add(hear), () => void heard.delete(hear)),
    () => hud,
  );
const facetEl = () =>
  document.querySelector<HTMLElement>('.console-bay .deck-tile[data-tile="needs"]');
const closeCalls = () => {
  set(CLOSED);
  facetEl()?.focus();
};
export const toggleCalls = () =>
  hud.open
    ? closeCalls()
    : set({
        open: true,
        key: undefined,
      });
/** Bring a calling Job into the HUD. */
export const showCall = (key: string) =>
  set({
    open: true,
    key,
  });

/** All the stage's height, for the question to stand whole: the brackets clear of the title bar above and the beam's collar clear of the console below. */
const placeOf = (stage: Box): Box => {
  // A narrow stage has no width to give up: the box's selects need all of it.
  const inset = stage.width < 800 ? 0 : Math.min(56, stage.width * 0.05);
  return {
    left: Math.round(stage.left + inset),
    top: Math.round(stage.top + 6),
    width: Math.round(stage.width - inset * 2),
    height: Math.round(stage.height - 14),
  };
};

/** ⌥N, the family of the Needs you card's own ⌥E, ⌥X and ⌥↵. By the key's place: Option changes the character N types. */
const isNextKey = (event: KeyboardEvent) =>
  event.altKey && !event.metaKey && !event.ctrlKey && event.code === "KeyN";
/** Enter on anything that does not take Enter itself opens the Job. */
const TAKES_ENTER =
  "button, a[href], input, textarea, select, [role='textbox'], [role='radio'], [role='checkbox'], [role='menuitem'], [role='option']";

export function CallsHud({
  keys,
  calls,
  nameOf,
  summaryOf,
  pane,
  onOpen,
}: {
  keys: readonly string[];
  calls: readonly string[];
  nameOf: (key: string) => string | undefined;
  summaryOf: (key: string) => string | undefined;
  pane: (key: string, side: HTMLElement | null) => ReactNode;
  onOpen: (key: string) => void;
}) {
  const { open, key } = useCallsHud();
  const facet = useRef<Element | null>(null);
  facet.current = open ? facetEl() : null;
  const [side, setSide] = useState<HTMLElement | null>(null);
  const body = useRef<HTMLDivElement>(null);
  const shown = useRef<string | undefined>(undefined);
  // Which Job shows: the one under the list's cursor if it is calling, else the first. One answered gives way to the next below it in the list.
  useLayoutEffect(() => {
    if (!open) {
      shown.current = undefined;
      return;
    }
    if (key !== undefined && calls.includes(key)) {
      shown.current = key;
      return;
    }
    const cursor = deckNow().cursor;
    const from = shown.current === undefined ? -1 : keys.indexOf(shown.current);
    const next =
      from === -1
        ? cursor !== undefined && calls.includes(cursor)
          ? cursor
          : calls[0]
        : (calls.find((one) => keys.indexOf(one) > from) ?? calls[0]);
    shown.current = next;
    if (next !== key)
      set({
        open: true,
        key: next,
      });
  }, [
    open,
    key,
    calls,
    keys,
  ]);
  // Going into a Job puts the HUD away with the Cockpit it stood over.
  useEffect(() => {
    if (open && facetEl() === null) set(CLOSED);
  });
  // The HUD is kept outside this component, and the next Cockpit drawn must not open on it.
  useEffect(() => () => set(CLOSED), []);
  // On each Job that comes in, the keys go to its question, so a number or an arrow answers it as in the Job.
  useEffect(() => {
    if (!open) return;
    // Its card takes a moment to open, and cannot be focused until it has.
    let tries = 0;
    const land = window.setInterval(() => {
      const within = body.current;
      const panel = document.querySelector<HTMLElement>('[data-hud="calls"] .console-hud__panel');
      if (tries++ === 0 && panel?.contains(document.activeElement) !== true) panel?.focus();
      const card = within?.querySelector(".armada-needs-panel");
      if (tries > 20 || card?.contains(document.activeElement) === true)
        return window.clearInterval(land);
      card
        ?.querySelector<HTMLElement>(":is([role='radio'], [role='checkbox'], input, textarea)")
        ?.focus();
    }, 100);
    return () => window.clearInterval(land);
  }, [
    open,
    key,
  ]);
  // The card takes its height from the thread here, which a thread at its end does not hear of: it keeps its latest line in view as it shrinks.
  useEffect(() => {
    if (!open) return;
    const rows = body.current?.querySelector<HTMLElement>(".armada-session-thread__rows");
    if (rows === null || rows === undefined) return;
    let atEnd = true;
    let was = rows.scrollTop;
    const follow = () => {
      if (atEnd) rows.scrollTop = rows.scrollHeight;
    };
    // Only a scroll back up leaves the end: one that lands short while the card is still opening does not.
    const read = () => {
      if (rows.scrollTop + rows.clientHeight >= rows.scrollHeight - 2) atEnd = true;
      else if (rows.scrollTop < was) atEnd = false;
      was = rows.scrollTop;
    };
    follow();
    rows.addEventListener("scroll", read, {
      passive: true,
    });
    const watch = new ResizeObserver(follow);
    watch.observe(rows);
    return () => {
      rows.removeEventListener("scroll", read);
      watch.disconnect();
    };
  }, [
    open,
    key,
  ]);
  const next = useCallback(() => {
    const at = key === undefined ? -1 : keys.indexOf(key);
    const to = calls.find((one) => keys.indexOf(one) > at) ?? calls[0];
    if (to !== undefined && to !== key)
      set({
        open: true,
        key: to,
      });
  }, [
    calls,
    key,
    keys,
  ]);
  const openJob = useCallback(() => {
    if (key !== undefined) onOpen(key);
  }, [
    key,
    onOpen,
  ]);
  const onKey = useCallback(
    (event: KeyboardEvent) => {
      if (isNextKey(event)) {
        event.preventDefault();
        event.stopPropagation();
        next();
        return true;
      }
      const target = event.target;
      if (
        event.key === "Enter" &&
        !event.altKey &&
        !event.metaKey &&
        !event.shiftKey &&
        !(target instanceof Element && (holdsText(target) || target.closest(TAKES_ENTER) !== null))
      ) {
        event.preventDefault();
        event.stopPropagation();
        openJob();
        return true;
      }
      return false;
    },
    [
      next,
      openJob,
    ],
  );
  if (!open) return null;
  const others = calls.some((one) => one !== key);
  const summary = key === undefined ? undefined : summaryOf(key);
  return (
    <ConsoleHud
      label="Calls"
      name={key === undefined ? undefined : nameOf(key)}
      facet={facet}
      onClose={closeCalls}
      place={placeOf}
      head={44}
      onKey={onKey}
      landsItself
      kind="calls"
      role="region"
      acts={
        key === undefined ? null : (
          <>
            {others ? (
              <Button variant="ghost" size="sm" onClick={next}>
                Next call
                <Kbd>⌥N</Kbd>
              </Button>
            ) : null}
            <Button variant="secondary" size="sm" onClick={openJob}>
              Open Job
              <Kbd>↵</Kbd>
            </Button>
          </>
        )
      }
    >
      {() =>
        key === undefined ? null : (
          <div ref={body} className="console-calls">
            <div className="console-calls__thread">{pane(key, side)}</div>
            <div className="console-calls__side">
              {summary === undefined ? null : <p className="console-calls__so-far">{summary}</p>}
              <div ref={setSide} className="console-calls__now" />
            </div>
          </div>
        )
      }
    </ConsoleHud>
  );
}
