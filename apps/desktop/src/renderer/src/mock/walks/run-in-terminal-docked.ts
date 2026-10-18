// A command a Session gave the person opens a terminal that takes keys, docked above the Session's message
// box with a handle on its top edge. Over the `session-run-in-terminal` scenario.

import { button, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const needs = region("Needs you");
const thread = region("Thread");
const run = inside(needs, role("group", /wrangler deploy/));
const pane = role("group", "Terminal", { exact: true });
const screen = inside(pane, role("textbox", "Terminal input"));
const typed = role("textbox", "Terminal input");

const steps: Step[] = [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button(/Deploy the store/)), say: "Opened" },
  { press: inside(needs, button(/^Needs you/)), say: "Needs you, opened" },
  { press: inside(needs, button(/^Open Deploy/)), say: "A press on its row opens it" },
  { look: inside(run, button("Run", { exact: true })), say: "The command, with Run" },
  { press: inside(run, button("Run", { exact: true })), say: "Run" },
  { look: screen, say: "The terminal docks above the box, and takes the keys" },
  { look: inside(pane, text(/dash\.cloudflare\.com\/oauth2\/auth/)), say: "The sign in link" },
  { later: inside(pane, text("Successfully logged in.")), say: "Signed in" },
  { look: inside(pane, text("Would you like to register a workers.dev subdomain now?")), say: "The first prompt waits for a key" },
  { type: "y\n", into: typed, say: "y, Enter" },
  { look: inside(pane, text(/What would you like your workers\.dev subdomain/)), say: "The next prompt takes text" },
  { type: "mele-labs\n", into: typed, say: "mele-labs, Enter" },
  { look: inside(pane, text(/Ok to proceed\?/)), say: "The last prompt waits" },
  { type: "y\n", into: typed, say: "y, Enter" },
  { look: inside(pane, text("https://armada-store.mele-labs.workers.dev")), say: "The deployed address" },
  { look: inside(pane, role("img", "Passed")), say: "Exit 0, and the Job is told" },
  { look: inside(thread, button(/^Exit 0/)), say: "The Job was sent the exit" },
  { press: inside(thread, button(/^Exit 0/)), say: "The lines it was sent" },
  { look: inside(thread, role("log", "Sent to the Job")), say: "The last lines, as plain text" },
  { look: inside(thread, text(/The store is live at https:\/\/armada-store\.mele-labs\.workers\.dev/)), say: "The Job acts on it" },
  { press: inside(pane, button("Close")), say: "Closed" },
  { press: inside(thread, button("Show terminal")), say: "The block's mark brings it back" },
  { look: pane, say: "The same run, ended" },
];

const docked = walk("session-run-in-terminal", steps);

export { docked as "run-in-terminal-docked" };
