// The Job browser as a tab: a page the Job's agent drives with Armada's tools, in a Browser tab like any other, with its own page for each tab. The mark after the address says who is driving, and a press on it
// takes the page over or hands it back. Back, Forward and Reload are a browser's, and the address takes an address while the person drives. Capture, the walk window's note control, pins a note on a part of the
// page, and Send puts it to the Job as a message in its thread. The tab's menu moves the page to a window of its own, and Move to the pane brings it back. Mock only: nothing is wired. Over the `job-layouts` scenario.

import { button, deckRow, dialog, inside, region, role, tab, text, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const pane = (name: string) => region(`Pane: ${name}`, { exact: true });
const empty = region("Empty pane", { exact: true });
const item = (name: string | RegExp) => role("menuitem", name);
const thread = inside(pane("Thread"), tab("Thread", { exact: true }));
const host = "localhost:5173";
const browser = inside(pane(host), role("group", "Job browser", { exact: true }));
const page = inside(browser, role("article", "Fernwood Supply checkout"));
const address = inside(browser, role("textbox", "Address", { exact: true }));
const note = dialog("Note on Total $84.00");
const popOut = dialog("Job browser: Retry the slot lease");
const inWindow = inside(popOut, role("group", "Job browser", { exact: true }));

const steps: Step[] = [
  toSessions,
  { press: deckRow("Retry the slot lease"), say: "A Job with a thread, and Done, Now and Open beside it" },
  { key: "Meta+d", on: thread, say: "⌘D splits the pane" },
  { press: inside(empty, button("Add tab")), say: "+" },
  { press: item("Browser"), say: "Browser, named by its page" },
  { look: page, say: "The Job browser, in a tab of its own" },
  { hover: inside(browser, button(/^Agent driving/)), say: "The agent is driving. The mark is the toggle" },
  { press: inside(browser, button(/^Agent driving/)), say: "A press takes the page over" },
  { hover: inside(browser, button(/^You're driving/)), say: "You're driving. A press hands it back" },
  { press: inside(browser, button("Back", { exact: true })), say: "Back" },
  { look: inside(browser, role("article", "Fernwood Supply cart")), say: "The cart, where the agent started" },
  { press: inside(browser, button("Forward", { exact: true })), say: "Forward, to the checkout" },
  { type: "SPRING20", into: inside(page, role("textbox", "Discount code")), say: "The page takes keys from you" },
  { press: inside(browser, button("Reload", { exact: true })), say: "Reload draws the page again" },
  { look: page, say: "The checkout, reloaded" },
  { type: "http://localhost:5173/cart\n", into: address, say: "The address takes one while you drive. Enter goes there" },
  { look: inside(browser, role("article", "Fernwood Supply cart")), say: "The cart" },
  { press: inside(browser, button("Back", { exact: true })), say: "Back to the checkout" },
  { hover: inside(browser, button("Capture", { exact: true })), say: "Capture, the walk window's own note control, with its key" },
  { press: inside(browser, button("Capture", { exact: true })), say: "Capture" },
  { press: inside(page, text("Total $84.00")), say: "A press on part of the page pins a note there" },
  { type: "The total still includes the discount", into: inside(note, role("textbox", "Note")), say: "The note" },
  { press: inside(note, button("Send")), say: "Send" },
  { look: inside(pane("Thread"), text(/^Note on Total \$84\.00 at http:\/\/localhost:5173\/checkout/)), say: "The note went to the Job as a message, with the part and the page it was on" },
  { hover: inside(browser, role("img", "Note on Total $84.00")), say: "The pin stays on the page, the note in its tooltip" },
  { press: inside(browser, button(/^You're driving/)), say: "Hand back" },
  { look: inside(browser, button(/^Agent driving/)), say: "The agent is driving again" },
  { context: inside(pane(host), tab(host, { exact: true })), say: "The tab's menu" },
  { press: item("Move to window"), say: "Move to window" },
  { look: inside(inWindow, role("textbox", "Address", { exact: true })), say: "The page in a window of its own, with Back, Forward and Reload" },
  { look: inside(pane(host), button("Move to the pane")), say: "The pane keeps its tab and says where the page is" },
  { press: inside(inWindow, button("Move to the pane")), say: "Move to the pane" },
  { look: page, say: "Back in the tab" },
];

const jobBrowser = walk("job-layouts", steps);

export { jobBrowser as "job-browser" };
