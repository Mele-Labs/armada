// A Session run in a terminal, opened in Bridge. Its conversation is the
// terminal's own transcript, read live, and the message box sends it words that
// start a turn there. The permission mode, model and effort stay with the
// terminal, so the box has none of them. Told twice: wide, and below the
// breakpoint, where the ledger folds into a sheet.

import { kit, NARROW } from "../sessions/walk-kit";
import { button, inside, region, text, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { message, thread } = kit(narrow);
  const sessions = region("Sessions");
  return [
    { press: inside(sessions, button("CI timeout hunt")), say: "A Session from a terminal is in the list with the others" },
    { look: inside(thread, text("It sleeps 50 ms")), say: "A long conversation opens at its newest message, with no scrolling" },
    { look: inside(thread, text("Why does the store test fail only in CI?")), say: "What was typed in the terminal is the first row" },
    { look: inside(thread, text("Read crates/store/src/tests/ledger.rs")), say: "Calls are one line each, as they are in a Session Bridge started" },
    { look: inside(thread, text("It sleeps 50 ms")), say: "And what the agent answered" },
    { look: message, say: narrow ? "A message box, with words only" : "A message box, with words only: the terminal holds the mode, the model and the effort" },
    { type: "which line is it?", into: message, say: "A message for the terminal's agent" },
    { press: button("Send"), say: "It reaches the terminal as if it had been typed there" },
    { look: inside(thread, text("which line is it?")), say: "It is in the thread" },
    { later: thread, say: "The terminal's agent works, and the thread follows" },
    { look: inside(thread, text("Line 88")), say: "Its answer" },
  ];
}

const wide = walk("sessions", steps(false));
const narrow = walk("sessions", steps(true), NARROW);

export { wide as "terminal-session", narrow as "terminal-session-narrow" };
