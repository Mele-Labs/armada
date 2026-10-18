// A command a Session waits on is run from the Needs you panel its row opens on the composer, by the path
// its ledger runs it by. A clean exit settles the item; a failure keeps the command, whose Run becomes Run again,
// and the agent is told nothing. Over `call-card-runs-command`.

import { button, deckRow, inside, region, role, walk } from "../walk";

const needs = region("Needs you");
const stop = inside(needs, button(/^Open Stop the mock server/));
const tests = inside(needs, button(/^Open Run the components tests/));
const strip = (command: string) => inside(needs, role("group", command));

const callCardRunsCommand = walk("call-card-runs-command", [
  { press: deckRow(/Debug Bridge on Merge line/), say: "A Session waits on commands to run: its row pulses, and pressing it opens the Session" },
  { press: inside(needs, button(/^Needs you/)), say: "The commands are in its Needs you panel" },
  { press: stop, say: "A press on a row opens the command" },
  { look: inside(strip("kill $(lsof -ti tcp:5788)"), button("Run")), say: "Run, on the command opened" },
  { press: inside(strip("kill $(lsof -ti tcp:5788)"), button("Run")), say: "Run runs it in the Session's folder" },
  { look: tests, say: "Exit 0: the item is settled, and the agent is told" },
  { press: tests, say: "The next command" },
  { press: inside(strip("pnpm vitest run packages/components"), button("Run")), say: "Run on the second command" },
  { look: inside(strip("pnpm vitest run packages/components"), button("Run again")), say: "Exit 1: the command stays, Run becomes Run again, and the agent is told nothing" },
]);

export { callCardRunsCommand };
