// What the Sessions walks share. Each walk is told twice, in the window the
// tests start from and in a narrow one: below the layout breakpoint the ledger
// is a sheet behind a button, so a step that looks at it is bracketed by
// opening it, and a step that presses a row of it opens it first (the row's
// press closes it again). Not under `walks/`, where every file is a walk.

import { button, inside, region, role } from "../walk";
import type { Step } from "../walk";

export const NARROW = { width: 900, height: 900 } as const;

/** Sessions' own page, from the left column: the Dashboard lists no Sessions apart from its tabs. */
export const toSessions: Step = { press: role("button", "Sessions", { exact: true }), say: "Sessions" };

export function kit(narrow: boolean) {
  const ledger = narrow ? role("dialog", "Attachments") : region("Attachments");
  /** The Close of a sheet, and not "Close as superseded" or the like. */
  const close = (within: ReturnType<typeof role>) => inside(within, role("button", /^Close( panel)?( Esc)?$/));
  return {
    ledger,
    message: role("textbox", "Message"),
    thread: region("Thread"),
    rail: (name: string) => role("button", name, { exact: true }),
    sheet: (name: string) => role("dialog", name),
    close,
    /** Looks at the ledger: at its side where it stands there, in its sheet where it has folded. */
    opened: (steps: Step[]): Step[] =>
      narrow
        ? [{ press: button("Attachments"), say: "The ledger, opened" }, ...steps, { press: close(ledger), say: "Back to the conversation" }]
        : steps,
    /** Presses a row of the ledger, which opens what it names. */
    row: (name: string, say: string): Step[] => [
      ...(narrow ? [{ press: button("Attachments"), say: "The ledger, opened" } as Step] : []),
      { press: inside(ledger, button(name)), say },
    ],
  };
}
