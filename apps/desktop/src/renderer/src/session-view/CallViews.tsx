// The takes' views of a call (`calls.ts`): the line above the composer's input, the card (whole in the thread's tail and under More, compact in the
// right column), the chip that jumps back to a thread's last card, and the keys. Every answer is a numbered key: a digit presses it where focus is
// not in a field with words in it. Colour is the theme's; a card is 8px and a control 6px; nothing glows.

import { useEffect, useState, useSyncExternalStore, type KeyboardEvent, type RefObject } from "react";
import { ArrowDown, ChevronLeft, ChevronRight, Ellipsis } from "lucide-react";
import { Button, Kbd, Textarea, Tooltip } from "@armada/components";

import type { Call } from "./calls";

// ---- Which call the line shows ------------------------------------------------------------------------------

const at = new Map<string, number>();
const hear = new Set<() => void>();

/** Which of a Session's calls the composer's line shows, and how to step it. The first, until he steps; it never runs past the ends. */
export function useCallAt(session: string, calls: readonly Call[]): [number, (by: 1 | -1) => void] {
  const now = useSyncExternalStore(
    (listener) => (hear.add(listener), () => void hear.delete(listener)),
    () => at.get(session) ?? 0,
  );
  const here = Math.min(now, Math.max(0, calls.length - 1));
  const step = (by: 1 | -1) => {
    at.set(session, Math.min(calls.length - 1, Math.max(0, here + by)));
    for (const one of hear) one();
  };
  return [here, step];
}

/** Whether a key is for the field it is typed in: a field with words in it takes the digits and the arrows. */
const typing = (target: HTMLElement) => {
  if (target.matches("input, textarea")) return (target as HTMLInputElement).value !== "";
  if (target.getAttribute("role") === "textbox") return (target.textContent ?? "") !== "";
  return false;
};
const isField = (target: HTMLElement) => target.matches("input, textarea") || target.getAttribute("role") === "textbox";

/**
 * The keys of a call: a digit presses that answer, unless it is typed into a field with words in it. In the composer, which is a field, an empty
 * box lets the arrows step through the calls, and a digit there is an answer: typing anything else is Other.
 */
export function callKeys(call: Call | undefined, step?: (by: 1 | -1) => void) {
  return (event: KeyboardEvent<HTMLElement>) => {
    if (call === undefined || event.metaKey || event.ctrlKey || event.altKey || event.defaultPrevented) return;
    const target = event.target as HTMLElement;
    if (typing(target)) return;
    if (/^[1-9]$/.test(event.key) && (!isField(target) || target.getAttribute("role") === "textbox")) {
      const option = call.options[Number(event.key) - 1];
      if (option === undefined) return;
      event.preventDefault();
      option.run();
    } else if (step !== undefined && (event.key === "ArrowLeft" || event.key === "ArrowRight") && target.getAttribute("role") === "textbox") {
      event.preventDefault();
      step(event.key === "ArrowLeft" ? -1 : 1);
    }
  };
}

// ---- Take: the composer answers --------------------------------------------------------------------------------

/** The line over the composer's input: what it is and the words, with the answers as numbered keys under it. */
export function CallLine({ calls, at: here, step, onMore }: { calls: readonly Call[]; at: number; step: (by: 1 | -1) => void; onMore: () => void }) {
  const call = calls[here];
  if (call === undefined) return null;
  return (
    <div className="sv-callline" role="group" aria-label="Waiting on you">
      <div className="sv-callline__head">
        <span className="sv-callline__word">{call.word}</span>
        <Tooltip label={call.text}>
          <span className="sv-callline__text">{call.text}</span>
        </Tooltip>
        {calls.length < 2 ? null : (
          <>
            <Tooltip label="Previous" shortcut="←">
              <Button type="button" variant="ghost" size="sm" iconOnly aria-label="Previous call" onClick={() => step(-1)}>
                <ChevronLeft size={16} aria-hidden="true" />
              </Button>
            </Tooltip>
            <Tooltip label="Next" shortcut="→">
              <Button type="button" variant="ghost" size="sm" iconOnly aria-label="Next call" onClick={() => step(1)}>
                <ChevronRight size={16} aria-hidden="true" />
              </Button>
            </Tooltip>
          </>
        )}
        <Tooltip label="The whole call">
          <Button type="button" variant="ghost" size="sm" iconOnly aria-label="More" onClick={onMore}>
            <Ellipsis size={16} aria-hidden="true" />
          </Button>
        </Tooltip>
      </div>
      <div className="sv-callline__options">
        {call.options.map((option, i) => {
          const press = (
            <button key={option.key} type="button" className="sv-callline__option" onClick={option.run}>
              <Kbd aria-hidden="true">{i + 1}</Kbd>
              <span>{option.label}</span>
            </button>
          );
          return option.hint === undefined ? press : (
            <Tooltip key={option.key} label={option.hint}>
              {press}
            </Tooltip>
          );
        })}
      </div>
    </div>
  );
}

// ---- Takes: the thread's last card, and the right column ------------------------------------------------------------

/** One call as a card: its words, its answers numbered, and Other under a question. Whole in the thread and under More, compact in the column. */
export function CallCard({ call, size, keep }: { call: Call; size: "full" | "compact"; keep: string }) {
  const [other, setOther] = useState("");
  const send = () => {
    const words = other.trim();
    if (words === "" || call.other === undefined) return;
    setOther("");
    call.other(words);
  };
  return (
    <section className="sv-call" data-size={size} data-kind={call.kind} aria-label={`${call.word}: ${call.text}`} onKeyDown={callKeys(call)}>
      <span className="sv-call__word">{call.word}</span>
      <p className="sv-call__text">{call.text}</p>
      {size === "compact" ? null : call.detail.map((line) => <p key={line} className="sv-call__detail">{line}</p>)}
      <ol className="sv-call__options">
        {call.options.map((option, i) => (
          <li key={option.key}>
            <button type="button" className="sv-call__option" data-layer-row="" onClick={option.run}>
              <Kbd aria-hidden="true">{i + 1}</Kbd>
              <span className="sv-call__label">{option.label}</span>
              {option.hint === undefined || size === "compact" ? null : <span className="sv-call__hint">{option.hint}</span>}
            </button>
          </li>
        ))}
      </ol>
      {call.other === undefined ? null : (
        <Textarea
          className="sv-call__other"
          aria-label="Other"
          rows={1}
          keep={keep}
          value={other}
          onChange={(event) => setOther(event.target.value)}
          onKeyDown={(event) => {
            if (event.key !== "Enter" || event.shiftKey || event.nativeEvent.isComposing) return;
            event.preventDefault();
            send();
          }}
        />
      )}
    </section>
  );
}

/** The right column's calls: a compact card for each, above Progress. The Field gives them room. */
export function CallsStack({ calls, session }: { calls: readonly Call[]; session: string }) {
  if (calls.length === 0) return null;
  return (
    <section className="sv-calls" aria-label="Waiting on you">
      {calls.map((call) => (
        <CallCard key={call.key} call={call} size="compact" keep={`call:${session}:${call.key}`} />
      ))}
    </section>
  );
}

/**
 * The chip over the composer when the thread is scrolled away from its last card: it names the call and jumps back to it, putting focus in the
 * card. Gone while the card is in view.
 */
export function CallChip({ calls, scope }: { calls: readonly Call[]; scope: RefObject<HTMLElement | null> }) {
  const [away, setAway] = useState(false);
  useEffect(() => {
    const scroller = scope.current?.querySelector<HTMLElement>(".armada-session-thread__rows");
    if (scroller === null || scroller === undefined) return;
    const read = () => {
      const tail = scroller.querySelector<HTMLElement>(".armada-session-thread__tail");
      if (tail === null) return setAway(false);
      const view = scroller.getBoundingClientRect();
      const box = tail.getBoundingClientRect();
      setAway(box.top > view.bottom - 24);
    };
    read();
    scroller.addEventListener("scroll", read, { passive: true });
    const watch = new ResizeObserver(read);
    watch.observe(scroller);
    return () => {
      scroller.removeEventListener("scroll", read);
      watch.disconnect();
    };
  }, [scope, calls.length]);
  const call = calls[0];
  if (call === undefined || !away) return null;
  const jump = () => {
    const tail = scope.current?.querySelector<HTMLElement>(".armada-session-thread__tail");
    tail?.scrollIntoView({ block: "end", behavior: window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth" });
    tail?.querySelector<HTMLElement>("button, textarea, input")?.focus({ preventScroll: true });
  };
  return (
    <div className="sv-callchip">
      <button type="button" className="sv-callchip__press" onClick={jump}>
        <span>{call.word}</span>
        <ArrowDown size={12} strokeWidth={2} aria-hidden="true" />
      </button>
    </div>
  );
}
