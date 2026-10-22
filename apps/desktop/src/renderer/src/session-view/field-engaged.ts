// Whether he is in a Job's Field panel: the pointer or focus within it. While he is, the Field's cards float over the app at full size;
// once both are gone they go back inside a grace later, so a move between the panel and a floated card does not drop the card, and at
// once on Esc. A floated card is drawn on the body but is a child of the Field in React's tree, so its pointer and focus events reach the
// Field's own handlers. Read from those events, never polled.

import { useEffect, useMemo, useRef, useState } from "react";

const graceMs = (): number => {
  const parsed = Number.parseInt(getComputedStyle(document.documentElement).getPropertyValue("--tooltip-grace"), 10);
  return Number.isNaN(parsed) ? 300 : parsed;
};

export type EngagedHandlers = { onPointerEnter: () => void; onPointerLeave: () => void; onFocus: () => void; onBlur: () => void };

/** `on` is false where the cards never float: the Field large already draws them at full size. */
export function useEngaged(on: boolean): { engaged: boolean; handlers: EngagedHandlers } {
  const [engaged, setEngaged] = useState(false);
  const pointer = useRef(false);
  const focus = useRef(false);
  const timer = useRef<number | undefined>(undefined);
  const handlers = useMemo<EngagedHandlers>(() => {
    const settle = () => {
      window.clearTimeout(timer.current);
      if (pointer.current || focus.current) setEngaged(true);
      else timer.current = window.setTimeout(() => setEngaged(false), graceMs());
    };
    return {
      onPointerEnter: () => ((pointer.current = true), settle()),
      onPointerLeave: () => ((pointer.current = false), settle()),
      onFocus: () => ((focus.current = true), settle()),
      onBlur: () => ((focus.current = false), settle()),
    };
  }, []);
  useEffect(() => () => window.clearTimeout(timer.current), []);
  // Esc puts the cards back inside; the pointer coming in again, or focus moving, brings them out.
  useEffect(() => {
    if (!engaged) return;
    const away = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      pointer.current = false;
      focus.current = false;
      window.clearTimeout(timer.current);
      setEngaged(false);
    };
    window.addEventListener("keydown", away);
    return () => window.removeEventListener("keydown", away);
  }, [engaged]);
  return { engaged: on && engaged, handlers };
}
