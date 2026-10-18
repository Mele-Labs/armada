// What a Session's agent has running in the background, as the ledger's Background section: a mark
// pulses while a command runs, a hover gives its command and how long it has run, All adds the ones
// that ended, and a press opens the output in the reading sheet. Over the `ledger-background` scenario.
// Told twice: wide, and below the breakpoint.

import { NARROW, kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { ledger, sheet } = kit(narrow);
  const sessions = region("Jobs", { exact: true });
  const section = inside(ledger, region("Background"));
  const toggle = (one: "Open" | "All") => inside(section, role("radio", one, { exact: true }));
  return [
    { press: inside(sessions, button("Serve the Bridge mock")), say: "A Job whose agent has things running in the background" },
    { look: section, say: "Background: two commands and a monitor, each pulsing while it runs" },
    { hover: inside(section, button(/Open Background command Serve the Bridge mock/)), say: "The command in full, when it started and how long it has run" },
    { press: toggle("All"), say: "All" },
    { look: inside(section, button(/Open Background command Build the Bridge renderer, exit 0/)), say: "A command that ended, with its exit in the mark" },
    { look: inside(section, button(/Open Background command Follow the Fleet log, stopped/)), say: "And one that was stopped" },
    { press: inside(section, button(/Open Background command Build the Bridge renderer/)), say: "Its output opens in the reading sheet" },
    { look: sheet("Build the Bridge renderer"), say: "The last lines it wrote" },
  ];
}

const wide = walk("ledger-background", [toSessions, ...steps(false)]);
const narrow = walk("ledger-background", [toSessions, ...steps(true)], NARROW);

export { wide as "ledger-background", narrow as "ledger-background-narrow" };
