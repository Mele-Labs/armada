// The Job browser: a page the Job's agent drives with Armada's tools. Flight control's Browser opens it in the side panel, the
// Job's thread beside it, as Requests opens. Its head says who is driving; Take over hands the page to the person and Hand back
// returns it. Annotate pins a note on a part of the page, and Send puts it to the Job as a message. Open in a window moves it to a
// window of its own with the Job's thread beside the page, as the walk window draws it, and Move to the side panel brings it back.
// When the Job reaches for Playwright's browser instead, its thread says so, with Open Job browser.
// Mock only: nothing is wired. Over the `job-browser` scenario.

import { button, deckRow, dialog, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const job = region(/^Job: /);
const thread = inside(job, region("Thread"));
const tile = inside(region("Flight controls"), button("Browser", { exact: true }));
const panel = dialog("Job browser");
const pane = inside(panel, role("group", "Job browser", { exact: true }));
const page = inside(pane, role("article", "Fernwood Supply checkout"));
const popOut = dialog("Job browser: Fix the checkout total");
const inWindow = inside(popOut, role("group", "Job browser", { exact: true }));
const windowThread = inside(inside(popOut, job), region("Thread"));
const note = dialog("Note on Total $84.00");

const steps: Step[] = [
  toSessions,
  { press: deckRow("Fix the checkout total"), say: "A Job fixing the checkout total" },
  { look: tile, say: "Browser, in Flight control beside Terminal and Requests" },
  { press: tile, say: "Browser" },
  { look: pane, say: "The Job browser, in the side panel" },
  { look: inside(thread, text(/the total stays at \$84\.00/)), say: "The Job's thread stays beside it" },
  { look: inside(pane, role("textbox", "Address", { exact: true })), say: "The page's address. The agent opened it" },
  { hover: inside(pane, role("img", "Agent driving")), say: "The agent is driving" },
  { look: inside(thread, text("browser_snapshot")), say: "Each act the agent took on the page is a call in the thread" },
  { look: inside(page, text("Discount $8.40")), say: "The code it typed, applied" },
  { press: inside(pane, button("Take over")), say: "Take over" },
  { hover: inside(pane, role("img", "You're driving")), say: "You're driving" },
  { type: "SPRING20", into: inside(page, role("textbox", "Discount code")), say: "The page takes keys from you now" },
  { press: inside(pane, button("Annotate")), say: "Annotate" },
  { press: inside(page, text("Total $84.00")), say: "A press on part of the page pins a note there" },
  { type: "The total still includes the discount", into: inside(note, role("textbox", "Note")), say: "The note" },
  { press: inside(note, button("Send")), say: "Send" },
  { look: inside(thread, text(/^Note on Total \$84\.00 at http:\/\/localhost:5173\/checkout/)), say: "The note went to the Job as a message, with the part and the page it was on" },
  { hover: inside(pane, role("img", "Note on Total $84.00")), say: "The pin stays on the page, the note in its tooltip" },
  { press: inside(pane, button("Hand back")), say: "Hand back" },
  { look: inside(pane, role("img", "Agent driving")), say: "The agent is driving again" },
  { press: inside(pane, button("Open in a window")), say: "Open in a window, as the terminal's pane has" },
  { look: inside(inWindow, role("textbox", "Address", { exact: true })), say: "The page in a window of its own" },
  { look: inside(windowThread, text(/^Note on Total \$84\.00/)), say: "The Job's thread beside the page, as the walk window draws it" },
  { look: inside(popOut, role("textbox", "Message")), say: "Answered from the window" },
  { press: inside(inWindow, button("Move to the side panel")), say: "Move to the side panel" },
  { look: pane, say: "Back in the side panel, on the same page" },
  { press: inside(panel, button("Close")), say: "Close. The Job keeps its browser" },
  { later: inside(thread, text("browser_snapshot")), say: "Time passes" },
  { look: inside(thread, text(/browser_navigate/)), say: "The agent reached for Playwright's own browser" },
  { look: inside(thread, role("note", "Browser outside Bridge: Playwright")), say: "The thread says so, and that the Job has a browser of its own" },
  { press: inside(thread, button("Open Job browser")), say: "Open Job browser" },
  { look: inside(pane, role("img", "Agent driving")), say: "The Job browser, in the side panel again" },
];

const jobBrowser = walk("job-browser", steps);

export { jobBrowser as "job-browser" };
