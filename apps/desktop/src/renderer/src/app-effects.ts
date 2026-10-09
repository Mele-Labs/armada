// The window's own effects, out of `App.tsx`, which is at the length the gate refuses: the clock the
// elapsed figures read, Escape out of a Job, the cursor's way back to its row, and where a pressed
// notification lands. Each is App's, and each says why it is the way it is.

import { useEffect, useState } from "react";

/** How often the elapsed figures are redrawn. They are read, so they must move. */
const TICK_MS = 1000;

/** The time the window reads, moving once a second. */
export function useNow(): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const tick = setInterval(() => setNow(Date.now()), TICK_MS);
    return () => clearInterval(tick);
  }, []);
  return now;
}

/**
 * Escape closes the detail wherever the cursor is inside it, which is the one thing every reader
 * tries first. Bound while a Job is open and not before, so nothing listens for a key that means
 * nothing. Escape leaves the composer too — bound inside `Composing`, not here, because with
 * anything typed it asks first and what has been typed is known there.
 */
export function useEscapeLeavesJob(openJob: string | null, close: () => void): void {
  useEffect(() => {
    if (openJob === null) return;
    const pressed = (event: KeyboardEvent): void => {
      // One view to leave, since the turns stopped being a screen of their own:
      // Escape returns to the list from anywhere inside a Job.
      // A press a layer above already answered — the palette, a sheet — is not a second exit.
      if (event.key !== "Escape" || event.defaultPrevented) return;
      close();
    };
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  }, [openJob]);
}

/**
 * The row the cursor goes back to when the detail closes. The row is back in the document only
 * after the list re-renders, so the focus move is an effect rather than part of the click that
 * closed it.
 */
export function useReturnToRow(returning: string | null, done: () => void): void {
  useEffect(() => {
    if (returning === null) return;
    const row = document.querySelector<HTMLElement>(`[data-job-id="${CSS.escape(returning)}"]`);
    row?.focus();
    done();
  }, [returning]);
}

/**
 * **Where a pressed notification says to go, and it always goes somewhere.** A press that raised
 * the window and left it on whatever it was last showing is a press that did nothing, which is
 * the one outcome that teaches somebody to stop pressing them.
 *
 * One Job opens that Job. Several open the set they came from — the Needs-you tab — because
 * picking one of four for somebody is choosing on their behalf. Either way the overlays come down
 * first: the press asked for the Board or a Job, not for the composer that happened to be up.
 *
 * A Session named instead comes from the bar of a window that Session opened, and opens that
 * Session on the Sessions surface.
 */
export function useSummoned(go: (jobId: string | null) => void, openSession: (sessionId: string) => void): void {
  useEffect(() => window.armada.onSummoned((to) => (to.sessionId === undefined ? go(to.jobId) : openSession(to.sessionId))), []);
}
