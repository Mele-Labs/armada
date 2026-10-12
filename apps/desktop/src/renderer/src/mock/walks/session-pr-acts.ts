// A press on a pull request in its panel is seen to be underway. Enable auto-merge shows the travelling
// bar on the pressed button while Fleet asks the forge, the other act is disabled, and the settled
// state replaces it. Review settles as the fact: "Review dispatched". Over the `sessions`
// scenario, whose mock takes a beat to answer. Told twice: wide, and below the breakpoint.

import { kit, NARROW, toSessions } from "../sessions/walk-kit";
import { button, inside, region, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { sheet, close, row } = kit(narrow);
  const sessions = region("Sessions");
  const card = sheet("Pull request #1850");
  return [
    { press: inside(sessions, button("Store migration spike")), say: "A Session holding a pull request whose Checks are still running" },
    ...row("Open Pull request #1850", "The pull request opens"),
    { look: inside(card, button("Enable auto-merge")), say: "Auto-merge is offered, and a Review beside it" },
    { press: inside(card, button("Enable auto-merge")), say: "Press it" },
    { look: inside(card, button("Enable auto-merge")), say: "Underway: the travelling bar on its edge, and Review disabled while Fleet asks the forge" },
    { look: inside(card, button("Auto-merge on")), say: "The forge answered: auto-merge is on" },
    { press: inside(card, button("Review", { exact: true })), say: "Press Review" },
    { look: inside(card, button("Review", { exact: true })), say: "Underway: the same bar, and the other button disabled" },
    { look: inside(card, button("Review dispatched")), say: "Dispatched, drawn as the fact and not pressable again" },
    { press: close(card), say: "Back to the Session" },
  ];
}

const wide = walk("sessions", [toSessions, ...steps(false)]);
const narrow = walk("sessions", [toSessions, ...steps(true)], NARROW);

export { wide as "session-pr-acts", narrow as "session-pr-acts-narrow" };
