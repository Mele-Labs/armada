// A terminal Session's message box and its mod. `/re` lists only the commands that start with it,
// then the ones that hold it, one row each in a column that scrolls; a send the terminal is not
// listening for is a toast that stays; and a Session whose mod is older than the repository's is
// marked on its row and in its header. Told twice: wide, and below the breakpoint.

import { kit, NARROW } from "../sessions/walk-kit";
import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";

const MOD = "Mod out of date: run /reload-plugins";

function steps(narrow: boolean): Step[] {
  const { message } = kit(narrow);
  const sessions = region("Sessions");
  const header = role("region", /^Session s4/);
  return [
    { look: inside(sessions, role("img", MOD)), say: "A Session whose mod is older than the repository's is marked on its row" },
    { press: inside(sessions, button("CI timeout hunt")), say: "Opened" },
    { hover: inside(header, role("img", MOD)), say: "And in its header, where the mark says what to run" },
    { type: "/re", into: message, say: "A slash and two letters" },
    { look: role("listbox", "Skills and commands"), say: "Only what starts with them, then what holds them: one row each, and the list scrolls" },
    { press: role("option", "/review"), say: "One picked" },
    { look: message, say: "Written into the message with a space after it" },
    { press: kit(narrow).rail("Sessions"), say: "Back to the list" },
    { press: inside(sessions, button("Notes check")), say: "Another terminal Session" },
    { type: "Draft the notes", into: message, say: "A message for it" },
    { press: button("Send"), say: "Its terminal is not listening" },
    { look: role("button", "Copy debug info"), say: "A toast stays, with what to run, and the thread says nothing" },
  ];
}

const wide = walk("composer-fixes", steps(false));
const narrow = walk("composer-fixes", steps(true), NARROW);

export { wide as "composer-fixes", narrow as "composer-fixes-narrow" };
