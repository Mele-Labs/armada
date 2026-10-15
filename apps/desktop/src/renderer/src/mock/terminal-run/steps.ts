// The story both takes of the terminal tell, which differ only in where the pane opens. Not under `walks/`,
// where every file is a walk.

import { button, inside, region, role, text } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";
import type { Place } from "./place";

export function deployWalk(place: Place): Step[] {
  const needs = region("Needs you");
  const thread = region("Thread");
  const run = inside(needs, role("group", /wrangler deploy/));
  const pane = role("group", "Terminal", { exact: true });
  const screen = inside(pane, role("textbox", "Terminal input"));
  const typed = role("textbox", "Terminal input");
  return [
    toSessions,
    { press: inside(region("Sessions"), button(/Deploy the store/)), say: "Opened" },
    { press: inside(needs, button(/^Needs you/)), say: "Needs you, opened" },
    { look: inside(run, button("Run", { exact: true })), say: "The command, with Run" },
    { press: inside(run, button("Run", { exact: true })), say: "Run" },
    { look: screen, say: place === "inline" ? "The terminal opens under the block, and takes the keys" : "The terminal docks above the box, and takes the keys" },
    { look: inside(pane, text(/dash\.cloudflare\.com\/oauth2\/auth/)), say: "The sign in link" },
    { later: inside(pane, text("Successfully logged in.")), say: "Signed in" },
    { look: inside(pane, text("Would you like to register a workers.dev subdomain now?")), say: "The first prompt waits for a key" },
    { type: "y\n", into: typed, say: "y, Enter" },
    { look: inside(pane, text(/What would you like your workers\.dev subdomain/)), say: "The next prompt takes text" },
    { type: "mele-labs\n", into: typed, say: "mele-labs, Enter" },
    { look: inside(pane, text(/Ok to proceed\?/)), say: "The last prompt waits" },
    { type: "y\n", into: typed, say: "y, Enter" },
    { look: inside(pane, text("https://armada-store.mele-labs.workers.dev")), say: "The deployed address" },
    { look: inside(pane, role("img", "Passed")), say: "Exit 0, and the Session is told" },
    { look: inside(thread, button(/^Exit 0/)), say: "The Session was sent the exit" },
    { press: inside(thread, button(/^Exit 0/)), say: "The lines it was sent" },
    { look: inside(thread, role("log", "Sent to the Session")), say: "The last lines, as plain text" },
    { look: inside(thread, text(/The store is live at https:\/\/armada-store\.mele-labs\.workers\.dev/)), say: "The Session acts on it" },
    ...(place === "docked"
      ? ([
          { press: inside(pane, button("Close")), say: "Closed" },
          { press: inside(thread, button("Show terminal")), say: "The block's mark brings it back" },
          { look: pane, say: "The same run, ended" },
        ] satisfies Step[])
      : []),
  ];
}
