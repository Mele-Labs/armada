// The phone mock's walks. Kept beside the mock rather than in `walks/` because the
// `walks-*of4` tests mount `App` for every file there, and this is not it; no tests
// until the owner approves the look.

import { button, inside, region, role, tab, text, walk } from "../walk";
import type { Walk } from "../walk";

/** The scenario name `main.tsx` reads as "draw the phone, not Bridge". */
export const POCKET = "pocket/phone";

const phone = { width: 1100, height: 900 };
const open = (name: string) => button(new RegExp(name, "i"));

export const POCKET_WALKS: ReadonlyMap<string, Walk> = new Map<string, Walk>([
  ["pocket-phone", walk(POCKET, [
    { look: text("Code scanned"), say: "Pair: the code is read" },
    { press: button("Pair"), say: "Pair sends the request to the Mac" },
    { look: button("Waiting for Confirm"), say: "Waiting for Confirm on the Mac" },
    { press: button("Waiting for Confirm"), say: "Mock only: the Mac confirmed" },
    { look: tab("Needs you"), say: "Needs you: Blocked Jobs, approval, review, session asks, Terminal waiting" },
    { press: open("drone count"), say: "A stalled Job" },
    { look: text("Quiet"), say: "Read view: status, step, ages, Judge, Checks, last action" },
    { press: button("Redirect"), say: "Redirect: a box for dictation" },
    { type: "Check the count against the slot pool, not the store", into: role("textbox", "Redirect"), say: "Dictated text" },
    { press: button("Close"), say: "Closing keeps the draft" },
    { look: button("Kill"), say: "Kill is held, not tapped" },
    { press: button("Back to Needs you"), say: "Back" },
    { press: open("PR rows"), say: "A review" },
    { look: text("Pass, no objections"), say: "Judge verdict, Checks, pull request. No diff" },
    { press: button("Request changes"), say: "Request changes takes a reason" },
    { press: button("Close"), say: "Closed" },
    { press: button("Back to Needs you"), say: "Back" },
    { press: open("Trim the pool"), say: "A hosted Session asks" },
    { press: role("radio", "Now"), say: "Pick an answer" },
    { press: button("Answer"), say: "Answer" },
    { look: tab("Running"), say: "Back on Needs you" },
    { press: tab("Running"), say: "Running" },
    { press: tab("Done"), say: "Done" },
    { press: button("Dispatch"), say: "One-line dispatch" },
    { type: "Fix the drone count", into: role("textbox", "Job"), say: "One line" },
    { press: role("radio", "ledger"), say: "A repository" },
    { press: inside(region("Dispatch"), button("Dispatch")), say: "Dispatch" },
    { look: tab("Running"), say: "The new Job is under Running" },
  ], phone)],
]);
