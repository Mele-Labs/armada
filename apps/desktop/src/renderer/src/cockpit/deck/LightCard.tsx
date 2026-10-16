// The shell of a light on the merge queue that opens a card: hovering or focusing the light opens it, a press pins it, and Esc or a
// press outside puts it away. The card is drawn on the page, placed by the light. Main's red and a held checkout are two lights in it. #2301.

import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { Tooltip } from "@armada/components";

/** How long the card waits after the pointer leaves before it closes, so the pointer can cross the gap between the light and it. */
const LEAVE_MS = 150;

export function LightCard({
  said,
  chip,
  mark,
  summoned = false,
  onSummoned,
  children,
}: {
  /** The light's tooltip and the card's name. */
  said: string;
  /** What the chip draws inside itself. */
  chip: ReactNode;
  /** Attributes the chip's styling reads: main's, or a tone. */
  mark: Record<string, string | true>;
  /** A call asked for the card to open: it is pinned, and `onSummoned` lets go of the ask. */
  summoned?: boolean;
  onSummoned?: () => void;
  /** The card's body, given the way to put the card away. */
  children: (away: () => void) => ReactNode;
}) {
  // The light's own ref and handlers are the Tooltip's to replace, so the light is found, and listened to, from the span around it.
  const wrap = useRef<HTMLSpanElement>(null);
  const light = () => wrap.current?.querySelector<HTMLButtonElement>("button") ?? null;
  const card = useRef<HTMLDivElement>(null);
  const timer = useRef<number | undefined>(undefined);
  const [hover, setHover] = useState(false);
  const [focus, setFocus] = useState(false);
  const [pinned, setPinned] = useState(false);
  const open = hover || focus || pinned;
  const away = () => {
    window.clearTimeout(timer.current);
    setHover(false);
    setFocus(false);
    setPinned(false);
  };

  useEffect(() => () => window.clearTimeout(timer.current), []);

  useEffect(() => {
    if (!summoned) return;
    setPinned(true);
    onSummoned?.();
  }, [summoned]);

  // Where the card sits: above the light, which lives low on the deck, and below it where there is no room above.
  useLayoutEffect(() => {
    const layer = card.current;
    const from = light()?.getBoundingClientRect();
    if (!open || layer === null || from === undefined) return;
    const gap = 8;
    const own = layer.getBoundingClientRect();
    const left = Math.max(gap, Math.min(from.left, window.innerWidth - own.width - gap));
    const above = from.top - gap - own.height;
    layer.style.left = `${left}px`;
    layer.style.top = `${above >= gap ? above : from.bottom + gap}px`;
  });

  useEffect(() => {
    if (!open) return;
    // Esc puts the card away and stops there, so it does not also return the stage to what it was.
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      event.stopPropagation();
      if (card.current?.contains(document.activeElement) === true) light()?.focus({ preventScroll: true });
      away();
    };
    const onDown = (event: MouseEvent) => {
      const hit = event.target as Node;
      if (wrap.current?.contains(hit) !== true && card.current?.contains(hit) !== true) away();
    };
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("mousedown", onDown);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("mousedown", onDown);
    };
  }, [open]);

  const enter = () => {
    window.clearTimeout(timer.current);
    setHover(true);
  };
  const leave = () => {
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => setHover(false), LEAVE_MS);
  };
  const blur = (event: { relatedTarget: EventTarget | null }) => {
    const to = event.relatedTarget as Node | null;
    if (to !== null && (wrap.current?.contains(to) === true || card.current?.contains(to) === true)) return;
    setFocus(false);
    // Focus that moved on to somewhere else puts the card away; a window losing focus names no target and leaves it.
    if (to !== null) setPinned(false);
  };

  return (
    <>
      <span ref={wrap} className="deck-main-light" onMouseEnter={enter} onMouseLeave={leave} onFocus={() => setFocus(true)} onBlur={blur}>
        <Tooltip label={said} asChild>
          <button
            type="button"
            className="deck-chip"
            {...mark}
            data-card={open || undefined}
            aria-label={said}
            aria-haspopup="dialog"
            aria-expanded={open}
            onClick={(event) => {
              if (pinned) return away();
              setPinned(true);
              // The card is drawn at the end of the page, so a key press carries focus into it: Tab from the light would not reach it.
              if (event.detail === 0) window.requestAnimationFrame(() => card.current?.querySelector<HTMLElement>("button, a[href]")?.focus());
            }}
          >
            {chip}
          </button>
        </Tooltip>
      </span>
      {!open
        ? null
        : createPortal(
            <div
              ref={card}
              className="deck-main-card"
              role="dialog"
              aria-label={said}
              onMouseEnter={enter}
              onMouseLeave={leave}
              onMouseDown={() => setPinned(true)}
              onFocus={() => setFocus(true)}
              onBlur={blur}
            >
              {children(away)}
            </div>,
            document.body,
          )}
    </>
  );
}
