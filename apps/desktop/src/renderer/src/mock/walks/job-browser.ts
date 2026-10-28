// The Job browser: a page the Job's agent drives with Armada's tools, docked in the Cockpit between the Job and the merge line
// as the Job's shell is. Its head says who is driving; Take over hands the page to the person and Hand back returns it. Annotate
// pins a note on a part of the page, and Send puts it to the Job as a message. Open in a window moves it to a window of its own,
// whose close puts it away. When the Job reaches for Playwright's browser instead, its thread says so, with Open Job browser.
// Mock only: nothing is wired. Over the `job-browser` scenario.

import { button, deckRow, dialog, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const job = region(/^Job: /);
const thread = inside(job, region("Thread"));
const docked = inside(region("Job browser", { exact: true }), role("group", "Job browser", { exact: true }));
const head = (target: ReturnType<typeof button>) => inside(docked, target);
const page = inside(docked, role("article", "Fernwood Supply checkout"));
const popOut = dialog("Job browser: Fix the checkout total");
const note = dialog("Note on Total $84.00");

const steps: Step[] = [
  toSessions,
  { press: deckRow("Fix the checkout total"), say: "A Job fixing the checkout total" },
  { look: docked, say: "Its Job browser, docked between the Job and the merge line, as its shell is" },
  { look: inside(docked, role("textbox", "Address", { exact: true })), say: "The page's address. The agent opened it" },
  { hover: inside(docked, role("img", "Agent driving")), say: "The agent is driving" },
  { look: inside(thread, text("browser_type SPRING10 into Discount code")), say: "Each act the agent took on the page is a call in the thread" },
  { look: inside(page, text("Discount $8.40")), say: "The code it typed, applied" },
  { look: inside(thread, text(/the total stays at \$84\.00/)), say: "What it read off the page" },
  { press: head(button("Take over")), say: "Take over" },
  { hover: inside(docked, role("img", "You're driving")), say: "You're driving" },
  { type: "SPRING20", into: inside(page, role("textbox", "Discount code")), say: "The page takes keys from you now" },
  { press: head(button("Annotate")), say: "Annotate" },
  { press: inside(page, text("Total $84.00")), say: "A press on part of the page pins a note there" },
  { type: "The total still includes the discount", into: inside(note, role("textbox", "Note")), say: "The note" },
  { press: inside(note, button("Send")), say: "Send" },
  { look: inside(thread, text(/^Note on Total \$84\.00 at http:\/\/localhost:5173\/checkout/)), say: "The note went to the Job as a message, with the part and the page it was on" },
  { hover: inside(docked, role("img", "Note on Total $84.00")), say: "The pin stays on the page, the note in its tooltip" },
  { press: head(button("Hand back")), say: "Hand back" },
  { look: inside(docked, role("img", "Agent driving")), say: "The agent is driving again" },
  { press: head(button("Open in a window")), say: "Open in a window, as the shell's pane has" },
  { look: inside(popOut, role("textbox", "Address", { exact: true })), say: "The same page in a window of its own" },
  { look: inside(popOut, button("Dock", { exact: true })), say: "Dock brings it back into the Job" },
  { press: inside(popOut, button("Close window")), say: "Close window. The Job keeps its browser" },
  { later: inside(thread, text("browser_snapshot")), say: "Time passes" },
  { look: inside(thread, text(/mcp__playwright__browser_navigate/)), say: "The agent reached for Playwright's own browser" },
  { look: inside(thread, role("note", "Browser outside Bridge: Playwright")), say: "The thread says so, and that the Job has a browser of its own" },
  { press: inside(thread, button("Open Job browser")), say: "Open Job browser" },
  { look: inside(docked, role("img", "Agent driving")), say: "Docked again, on the same page" },
];

const jobBrowser = walk("job-browser", steps);

export { jobBrowser as "job-browser" };
