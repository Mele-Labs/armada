// The Job browser: a page the Job's agent drives with Armada's tools. Flight control's Browser opens it in the side panel, with the
// Job's thread and composer floating on the left over the app, which the first control on the browser's toolbar folds away and brings back. The mark after it
// says who is driving, and a press on it takes the page over or hands it back. Back, Forward and Reload are a browser's,
// and the address takes an address while the person drives. Capture, the walk window's note control, pins a note on a part of the
// page, and Send puts it to the Job as a message. Open in a window moves it to a window of its own with the thread beside the page,
// as the walk window draws it, and Move to the side panel brings it back. When the Job reaches for Playwright's browser instead, its
// thread says so, with Open Job browser. Mock only: nothing is wired. Over the `job-browser` scenario.

import { button, deckRow, dialog, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const job = region(/^Job: /);
const tile = inside(region("Flight controls"), button("Browser", { exact: true }));
const panel = dialog("Job browser");
const pane = inside(panel, role("group", "Job browser", { exact: true }));
const page = inside(pane, role("article", "Fernwood Supply checkout"));
// The Job floating beside the side panel is the last Job on screen, after the Cockpit's own.
const floating = job;
const thread = inside(floating, region("Thread"));
const popOut = dialog("Job browser: Fix the checkout total");
const inWindow = inside(popOut, role("group", "Job browser", { exact: true }));
const windowThread = inside(inside(popOut, job), region("Thread"));
const note = dialog("Note on Total $84.00");
const address = inside(pane, role("textbox", "Address", { exact: true }));

const steps: Step[] = [
  toSessions,
  { press: deckRow("Fix the checkout total"), say: "A Job fixing the checkout total" },
  { look: tile, say: "Browser, in Flight control beside Terminal and Requests" },
  { press: tile, say: "Browser" },
  { look: pane, say: "The Job browser, in the side panel" },
  { look: inside(floating, role("textbox", "Message")), say: "The Job's thread and composer float on the left, over the app" },
  { look: inside(thread, text(/the total stays at \$84\.00/)), say: "Its thread, as the Cockpit draws it" },
  { hover: inside(pane, button("Hide Job")), say: "Hide Job, the first control on the toolbar, pressed while the thread shows" },
  { press: inside(pane, button("Hide Job")), say: "Hide Job folds it away" },
  { look: inside(panel, page), say: "The page alone" },
  { press: inside(pane, button("Show Job")), say: "Show Job brings it back" },
  { look: inside(thread, text("browser_snapshot")), say: "Each act the agent took on the page is a call in the thread" },
  { hover: inside(pane, button(/^Agent driving/)), say: "The agent is driving. The mark is the toggle" },
  { press: inside(pane, button(/^Agent driving/)), say: "A press takes the page over" },
  { hover: inside(pane, button(/^You're driving/)), say: "You're driving. A press hands it back" },
  { press: inside(pane, button("Back", { exact: true })), say: "Back" },
  { look: inside(pane, role("article", "Fernwood Supply cart")), say: "The cart, where the agent started" },
  { press: inside(pane, button("Forward", { exact: true })), say: "Forward, to the checkout" },
  { type: "SPRING20", into: inside(page, role("textbox", "Discount code")), say: "The page takes keys from you" },
  { press: inside(pane, button("Reload", { exact: true })), say: "Reload draws the page again" },
  { look: page, say: "The checkout, reloaded" },
  { type: "http://localhost:5173/cart\n", into: address, say: "The address takes one while you drive. Enter goes there" },
  { look: inside(pane, role("article", "Fernwood Supply cart")), say: "The cart" },
  { press: inside(pane, button("Back", { exact: true })), say: "Back to the checkout" },
  { hover: inside(pane, button("Capture", { exact: true })), say: "Capture, the walk window's own note control, with its key" },
  { press: inside(pane, button("Capture", { exact: true })), say: "Capture" },
  { press: inside(page, text("Total $84.00")), say: "A press on part of the page pins a note there" },
  { type: "The total still includes the discount", into: inside(note, role("textbox", "Note")), say: "The note" },
  { press: inside(note, button("Send")), say: "Send" },
  { look: inside(thread, text(/^Note on Total \$84\.00 at http:\/\/localhost:5173\/checkout/)), say: "The note went to the Job as a message, with the part and the page it was on" },
  { hover: inside(pane, role("img", "Note on Total $84.00")), say: "The pin stays on the page, the note in its tooltip" },
  { press: inside(pane, button(/^You're driving/)), say: "Hand back" },
  { look: inside(pane, button(/^Agent driving/)), say: "The agent is driving again" },
  { press: inside(pane, button("Open in a window")), say: "Open in a window, as the terminal's pane has" },
  { look: inside(inWindow, role("textbox", "Address", { exact: true })), say: "The page in a window of its own" },
  { look: inside(inWindow, button("Reload", { exact: true })), say: "With Back, Forward and Reload" },
  { look: inside(inWindow, button("Capture", { exact: true })), say: "Capture" },
  { look: inside(inWindow, button(/^Agent driving/)), say: "And the mark that hands the page over" },
  { look: inside(windowThread, text(/^Note on Total \$84\.00/)), say: "The Job's thread beside the page, as the walk window draws it" },
  { press: inside(inWindow, button("Move to the side panel")), say: "Move to the side panel" },
  { look: pane, say: "Back in the side panel, the Job beside it" },
  { press: inside(panel, button("Close")), say: "Close. The Job keeps its browser" },
  { later: inside(job, region("Thread")), say: "Time passes" },
  { look: inside(inside(job, region("Thread")), text(/browser_navigate/)), say: "The agent reached for Playwright's own browser" },
  { look: inside(inside(job, region("Thread")), role("note", "Browser outside Bridge: Playwright")), say: "The thread says so, and that the Job has a browser of its own" },
  { press: inside(inside(job, region("Thread")), button("Open Job browser")), say: "Open Job browser" },
  { look: inside(pane, button(/^Agent driving/)), say: "The Job browser, in the side panel again" },
];

const jobBrowser = walk("job-browser", steps);

export { jobBrowser as "job-browser" };
