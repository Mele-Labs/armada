// What the Sessions walks share. Each walk is told twice, in the window the
// tests start from and in a narrow one. The Cockpit draws a Session's ledger in its
// right column at either width, so `kit` takes `narrow` for the walks that still
// pass it and acts on it in neither. Not under `walks/`, where every file is a walk.

import { button, inside, place, region, role } from "../walk";
import type { Step } from "../walk";

export const NARROW = { width: 900, height: 900 } as const;

/** The Cockpit, whose Work list holds a Session's row: pressed, it opens the Session on the stage. */
export const toSessions: Step = { press: place("Cockpit"), say: "Cockpit" };

export function kit(_narrow: boolean) {
  const ledger = region("Attachments");
  /** The Close of a sheet, and not "Close as superseded" or the like. */
  const close = (within: ReturnType<typeof role>) => inside(within, role("button", /^Close( panel)?( Esc)?$/));
  return {
    ledger,
    message: role("textbox", "Message"),
    thread: region("Thread"),
    /** A named button; the title bar's places are its tabs, and the Sessions the Cockpit's list. */
    rail: (name: string) => (name === "Sessions" ? place("Cockpit") : ["Cockpit", "Jobs"].includes(name) ? place(name) : role("button", name, { exact: true })),
    sheet: (name: string) => role("dialog", name),
    close,
    /** Looks at the ledger, which stands at the Session's side. */
    opened: (steps: Step[]): Step[] => steps,
    /** Presses a row of the ledger, which opens what it names. */
    row: (name: string, say: string): Step[] => [{ press: inside(ledger, button(name)), say }],
  };
}
