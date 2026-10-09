// A Session shows a page: its window opens by itself, closes, opens again from the ledger row, and
// takes a note that goes to the Session. Over the `session-walk-window` scenario. Told twice: wide,
// and below the breakpoint. Picking an element in the page is left to the person; the test holds it.

import { button, dialog, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { NARROW, kit, toSessions } from "../sessions/walk-kit";

function steps(narrow: boolean): Step[] {
  const { ledger, message, opened, row, thread } = kit(narrow);
  const window = dialog("Bridge's window on Store clock findings");
  return [
    { press: inside(region("Sessions"), button("Store clock findings")), say: "A Session with nothing shown yet" },
    { type: "Show me what you found\n", into: message, say: "Asked to show it" },
    { look: window, say: "The window opened by itself, over the page" },
    { look: inside(thread, button("Open window Clock read report")), say: "A quiet row in the thread marks it" },
    ...opened([{ look: inside(ledger, role("img", "Shown in a window")), say: "And a row under Artifacts" }]),
    { press: button("Close window"), say: "Closed" },
    ...row("Open Shown in a window Clock read report", "The ledger row opens it again"),
    { look: window, say: "Back" },
    { look: inside(window, button("Store clock findings")), say: "Titled by the page's own name, and notes go to the Session by its own, which opens it" },
    { press: button("Capture"), say: "Capturing: a press in the page picks what is under it, and the note goes to the Session" },
  ];
}

const wide = walk("session-walk-window", [toSessions, ...steps(false)]);
const narrow = walk("session-walk-window", [toSessions, ...steps(true)], NARROW);

export { wide as "session-walk-window", narrow as "session-walk-window-narrow" };
