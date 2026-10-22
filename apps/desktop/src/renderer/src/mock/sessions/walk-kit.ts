// What the Sessions walks share. Each walk is told twice, in the window the
// tests start from and in a narrow one. A Job open on the Cockpit keeps what it made
// or used in the Artifacts layer, a key in Flight control, at either width, so `kit`
// takes `narrow` for the walks that still pass it and acts on it in neither. Not
// under `walks/`, where every file is a walk.

import { button, inside, place, region, role } from "../walk";
import type { Name, Step } from "../walk";

export const NARROW = { width: 900, height: 900 } as const;

/** The Cockpit, whose Work list holds a Session's row: pressed, it opens the Session on the stage. */
export const toSessions: Step = { press: place("Cockpit"), say: "Cockpit" };

export function kit(_narrow: boolean) {
  const artifacts = region("Artifacts", { exact: true });
  /** A key in Flight control, which opens its layer over the open Job's thread, or closes it. */
  const key = (name: string) => inside(region("Flight controls"), button(name, { exact: true }));
  const show: Step = { press: key("Artifacts"), say: "Artifacts" };
  const back: Step = { press: inside(artifacts, button("Back")), say: "Back to the thread" };
  /** The Close of a sheet, and not "Close as superseded" or the like. */
  const close = (within: ReturnType<typeof role>) => inside(within, role("button", /^Close( panel)?( Esc)?$/));
  return {
    artifacts,
    key,
    message: role("textbox", "Message"),
    thread: region("Thread"),
    /** A named button; the title bar's places are its tabs, and the Sessions the Cockpit's list. */
    rail: (name: string) => (name === "Sessions" ? place("Cockpit") : ["Cockpit", "Jobs"].includes(name) ? place(name) : role("button", name, { exact: true })),
    sheet: (name: string) => role("dialog", name),
    close,
    /** Opens the Artifacts layer, takes the steps in it, and goes back to the thread. */
    opened: (steps: Step[]): Step[] => [show, ...steps, back],
    /** Presses a row of the Artifacts layer, named by its title, which opens what it names; the layer closes behind it. */
    row: (name: Name, say: string): Step[] => [show, { press: inside(artifacts, button(name)), say }, back],
  };
}
