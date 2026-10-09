// A Session's Retro press asks the app for the Retros page with that Session's retro open. A
// window event, as `open-session.ts` is, because the press sits in lists that do not hold the
// app's navigation. The ask is kept until the page mounts and takes it.

import { useEffect } from "react";

const OPEN_RETRO = "armada:open-retro";

/** The Session whose retro the Retros page opens on: its id, and the name it is read by. */
export type RetroAsked = { id: string; label: string };

let waiting: RetroAsked | undefined;

export function askToOpenRetro(asked: RetroAsked): void {
  waiting = asked;
  window.dispatchEvent(new CustomEvent<RetroAsked>(OPEN_RETRO, { detail: asked }));
}

/**
 * The ask the page opening takes. **Dropped after the render that took it**, not in it: React may
 * run a state initializer twice, and both runs must see the ask.
 */
export function takeRetroAsked(): RetroAsked | undefined {
  const asked = waiting;
  queueMicrotask(() => {
    if (waiting === asked) waiting = undefined;
  });
  return asked;
}

export function useOpenRetroAsked(open: () => void): void {
  useEffect(() => {
    window.addEventListener(OPEN_RETRO, open);
    return () => window.removeEventListener(OPEN_RETRO, open);
  }, [open]);
}
