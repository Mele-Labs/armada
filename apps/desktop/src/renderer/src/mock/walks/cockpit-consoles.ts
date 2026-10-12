// The Cockpit as a ship's console. The deck of calls is drawn on Your move alone, and that tab carries a
// dot while anything needs him; a call that arrives while he is elsewhere raises a toast. The card keeps
// its content and its acts sit in consoles beside it as keycaps, in two layouts to choose between: side
// consoles, and one Helm rail. Dismiss is under a cover. A Job's box opens on `t`; a Session's is always
// open. Over `cockpit-consoles`.

import { button, inside, region, role, tab, text, walk } from "../walk";

const cockpit = region("Dashboard");
const check = region(/^Check failed: Cap the retry backoff/);
const stuck = region(/^Drone stuck: Raise the reconnect ceiling/);
const session = region(/^Session: Cap the retry ceiling/);
const waiting = role("region", "Waiting");

const cockpitConsoles = walk("cockpit-consoles", [
  { look: inside(tab("Your move"), role("img", "Needs you")), say: "Your move carries a dot while anything needs him, and never a figure" },
  { press: tab("Active"), say: "Active" },
  { look: role("listbox", "Tiles"), say: "Tiles and lines only: no deck of calls here, though a Job with a call is still its tile" },
  { hover: tab("Done"), say: "Done and Bays draw no deck either" },
  { later: cockpit, say: "Time passes, and nothing is new" },
  { later: cockpit, say: "A call arrives while he is somewhere else" },
  { look: text("“Raise the reconnect ceiling” needs you."), say: "A toast says so, wherever he is" },
  { press: button(/needs you/), say: "Pressing it goes to Your move, and the call comes forward" },
  { look: stuck, say: "Layout A. The card keeps its content; its acts moved out to the consoles beside it" },
  { look: inside(waiting, button(/Check failed/)), say: "Left: what waits behind the card, a strip of slim rows. Under it, where to go next" },
  { look: button(/Open Job/), say: "Open the Job and Later are keycaps on the left" },
  { look: button(/Retry step/), say: "The decision is on the right, the key printed on the cap" },
  { press: inside(waiting, button(/Check failed/)), say: "A strip brings its call forward" },
  { look: check, say: "A Check failed: the failing tests and its output stay on the card" },
  { hover: button(/^Skip$|^2 Skip/), say: "Skip says it ends the Check as skipped, not passed" },
  { look: button(/Skip all/), say: "Skip all, only because more than one Check is unfinished" },
  { look: button(/Dismiss/), say: "Dismiss sits at the foot of the right console, its cover down" },
  { press: button("Helm rail"), say: "Layout B" },
  { look: check, say: "One rail on the right: the decision on top as big keycaps, a rule, then Open and Later, then Dismiss under its cover" },
  { key: "t", on: cockpit, say: "A Job's box is shut until t" },
  { look: inside(check, role("textbox", "Message")), say: "Open: words here go to the Drone" },
  { key: "Escape", on: inside(check, role("textbox", "Message")), say: "Esc shuts it" },
  { press: button(/Dismiss/), say: "One press lifts the cover" },
  { look: button(/Dismiss/), say: "The key shows in the failed hue, and lets go on its own if left" },
  { press: button(/Dismiss/), say: "A second press dismisses it for good" },
  { press: inside(waiting, button(/^Session/)), say: "The Session's call, from the strip" },
  { look: inside(session, role("textbox", "Message")), say: "A Session's box is always open, the Session's own, with its own Send" },
]);

export { cockpitConsoles };
