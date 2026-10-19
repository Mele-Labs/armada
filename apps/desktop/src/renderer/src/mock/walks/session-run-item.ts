// A command the agent waits on the person to run: in the Needs you band it is the command's own block
// with Run, as in the thread, and a clean exit from either place settles its item and tells the agent.
// Over the `session-run-item` scenario. Told twice: wide, and below the breakpoint.

import { button, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { NARROW, toSessions } from "../sessions/walk-kit";

function steps(): Step[] {
  const sessions = region("Jobs", { exact: true });
  const needs = region("Needs you");
  const stop = inside(needs, role("group", "kill $(lsof -ti tcp:5788)"));
  const install = inside(region("Thread"), role("group", "pnpm install --frozen-lockfile"));
  return [
    { press: inside(sessions, button(/Debug Bridge on Merge line/)), say: "Opened" },
    { press: inside(needs, button(/^Needs you/)), say: "Needs you, opened" },
    { look: inside(stop, button("Run")), say: "Under Commands, a command is its own block with Run, as in the thread" },
    { press: inside(stop, button("Run")), say: "Run from the band" },
    { look: text("It ran clean."), say: "A clean exit settles the item, and the agent is told" },
    { press: inside(install, button("Run")), say: "The other command, run from the thread instead" },
    { look: region("Thread"), say: "It settles the same way, and the panel is gone with nothing left in it" },
  ];
}

const wide = walk("session-run-item", [toSessions, ...steps()]);
const narrow = walk("session-run-item", [toSessions, ...steps()], NARROW);

export { wide as "session-run-item", narrow as "session-run-item-narrow" };
