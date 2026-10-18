// A command a Session waits on is run from the strip its call opens under the Session's head, by the path
// its ledger runs it by. A clean exit settles the item and the strip moves to the next command; a failure
// keeps the strip, whose Run becomes Run again, and the agent is told nothing. The strip draws the command
// and its keys, not the output: that is in the Session. Over `call-card-runs-command`.

import { button, inside, region, role, text, walk } from "../walk";

const strip = role("group", /^Call: Debug Bridge on Merge line/);
const session = region(/^Job: Debug Bridge on Merge line/);

const callCardRunsCommand = walk("call-card-runs-command", [
  { press: inside(region("Incoming calls"), button(/^Job: Debug Bridge on Merge line/)), say: "A Job waits on commands to run: its call opens the Job, with the command in a strip under its head" },
  { look: inside(strip, text("kill $(lsof -ti tcp:5788)")), say: "The command, in the case it was written" },
  { look: inside(strip, button("Run")), say: "Run is a key, and Copy stays beside it" },
  { look: inside(strip, button("Copy")), say: "Copy" },
  { press: inside(strip, button("Run")), say: "Run runs it in the Job's folder" },
  { look: inside(strip, text("pnpm vitest run packages/components")), say: "Exit 0: the strip has moved on to the next command, and the agent is told" },
  { press: inside(strip, button("Run")), say: "Run on the second command" },
  { look: inside(strip, button("Run again")), say: "Exit 1: the strip stays, Run becomes Run again, and the agent is told nothing" },
  { look: inside(session, role("textbox", "Message")), say: "The Job's own box under it, to say what happened" },
]);

export { callCardRunsCommand };
