// From a Job that stopped to a Session that debugs it. Starts on the Board, on
// a Job stopped at its gate. "Open in a Session" starts a Session with the Job
// tagged and leaves its Drone as it was: it is not Pilot, which stops the
// Drone. The agent reads the Job with the tools Fleet gives it, says what it
// found, and on the person's ask redirects the Drone. Told twice, wide and
// below the breakpoint.

import { kit, NARROW } from "../sessions/walk-kit";
import { button, inside, role, text, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { ledger, message, thread, opened } = kit(narrow);
  return [
    { look: button("More for Cap the retry backoff"), say: "A Job stopped at its gate is on the Board" },
    { press: button("More for Cap the retry backoff"), say: "Its menu has an act for talking it through" },
    { look: role("menuitem", "Open in a Session"), say: "Open in a Session leaves the Drone as it is. Pilot stops it" },
    { press: role("menuitem", "Open in a Session"), say: "It starts a Session with the Job tagged" },
    { look: inside(role("group", "Attached"), text("Cap the retry backoff")), say: "The Job is a chip in the message box" },
    { type: "@", into: message, say: "An at sign lists Jobs too, grouped, each with its glyph" },
    { look: role("group", "Jobs"), say: "Jobs" },
    { look: role("group", "Pull requests"), say: "Pull requests, and branches and Sessions below them" },
    { type: "why did this fail?", into: message, say: "Asked as one would in a terminal" },
    { press: button("Send"), say: "The agent reads the Job with the tools Fleet gives it" },
    { look: inside(thread, text("examine_job 55")), say: "The Job" },
    { look: inside(thread, text("get_job_log 55")), say: "Its log" },
    { look: inside(thread, text("get_check_output 55 lint")), say: "The Check that failed" },
    { look: inside(thread, text("get_diff 55")), say: "And its diff" },
    { look: inside(thread, text("clippy flags the loop")), say: "Then the cause" },
    ...opened([
      { look: inside(ledger, role("img", "Looking at it, not dispatched from here")), say: "The Job is on the ledger as one the Session is looking at, marked apart from Jobs it dispatched" },
    ]),
    { type: "redirect it to cap the loop", into: message, say: "On the person's ask" },
    { press: button("Send"), say: "It redirects the Drone" },
    { look: inside(thread, text("redirect_drone 55")), say: "A call on Fleet's own tool" },
    ...opened([{ look: inside(ledger, role("img", "Running")), say: "The Job's state on the ledger moved from needs you to running" }]),
  ] satisfies Step[];
}

const wide = walk("sessions", steps(false));
const narrow = walk("sessions", steps(true), NARROW);

export { wide as "job-to-sessions", narrow as "job-to-sessions-narrow" };
