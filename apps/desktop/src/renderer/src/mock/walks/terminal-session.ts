// A Session run in a terminal, opened in Bridge. Its conversation is the
// terminal's own transcript, read live, and the message box sends it messages,
// files and pictures that start a turn there. Model and effort are set from the
// box and run in the terminal as typed; the permission mode is the terminal's
// own and is shown and not set. Told twice: wide, and below the
// breakpoint, where the ledger folds into a sheet.

import { kit, NARROW } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { message, thread } = kit(narrow);
  const sessions = region("Sessions");
  return [
    { press: inside(sessions, button("CI timeout hunt")), say: "A Session from a terminal is in the list with the others" },
    { look: inside(thread, text("It sleeps 50 ms")), say: "A long conversation opens at its newest message, with no scrolling" },
    { look: inside(thread, text("Why does the store test fail only in CI?")), say: "What was typed in the terminal is the first row" },
    { press: inside(thread, text("Grep, Read")), say: "Calls that ran in a row are one closed row, and it opens" },
    { look: inside(thread, text("Read crates/store/src/tests/ledger.rs")), say: "Calls are one line each, as they are in a Session Bridge started" },
    { look: inside(thread, text("It sleeps 50 ms")), say: "And what the agent answered" },
    { look: role("combobox", "Model"), say: "The model is the terminal's, and it can be changed from here" },
    { type: "opus", into: role("combobox", "Model"), say: "Run in the terminal as /model opus" },
    { type: "high", into: role("combobox", "Effort"), say: "And the effort, as /effort high" },
    { look: role("combobox", "Permission mode"), say: "The permission mode is shown and cannot be set: the terminal holds it" },
    { type: "/", into: message, say: "A slash lists the terminal's own commands" },
    { look: role("listbox", "Skills and commands"), say: "The same list the terminal offers" },
    { press: role("option", "/review"), say: "One chosen" },
    { paste: message, say: "A screenshot pasted in" },
    { look: inside(role("group", "Attached"), text("Screenshot")), say: "Saved by Fleet and sent to the terminal as a path" },
    { type: "which line is it?", into: message, say: "A message for the terminal's agent" },
    { press: button("Send"), say: "It reaches the terminal as if it had been typed there" },
    { look: inside(thread, text("which line is it?")), say: "It is in the thread" },
    { later: thread, say: "The terminal's agent works, and the thread follows" },
    { look: inside(thread, text("Line 88")), say: "Its answer" },
  ];
}

const wide = walk("terminal-session", steps(false));
const narrow = walk("terminal-session", steps(true), NARROW);

export { wide as "terminal-session", narrow as "terminal-session-narrow" };
