// A Session's retro card, in its thread, asks the app for Armada fixes, on the Logbook, with that retro open. A
// window event, as `open-session.ts` is, because the press sits in lists that do not hold the
// app's navigation. The ask is kept until the page mounts and takes it.

import { useEffect } from "react";

const OPEN_RETRO = "armada:open-retro";

/**
 * The Session whose retro Armada fixes opens on: its id, and the name it is read by. **`n` names
 * the retro** from the Session's thread card, which is one of several; absent, the newest.
 */
export type RetroAsked = { id: string; label: string; n?: number };

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

const OPEN_FIXES = "armada:open-fixes";

let fixesWaiting = false;

/** Asks the Logbook to bring Armada fixes into view: from a retro card, the palette, or a link to Retros. */
export function askForFixes(): void {
  fixesWaiting = true;
  window.dispatchEvent(new Event(OPEN_FIXES));
}

/** The ask a Logbook that is opening takes, once. */
export function takeFixesAsked(): boolean {
  const asked = fixesWaiting;
  fixesWaiting = false;
  return asked;
}

export function useFixesAsked(show: () => void): void {
  useEffect(() => {
    const heard = () => {
      fixesWaiting = false;
      show();
    };
    window.addEventListener(OPEN_FIXES, heard);
    return () => window.removeEventListener(OPEN_FIXES, heard);
  }, [show]);
}
