// A command a Session waits on is run from the Needs you panel its row opens on the composer, by the path
// its ledger runs it by. A clean exit settles the item; a failure keeps the command, whose Run becomes Run again,
// and the agent is told nothing. Over `call-card-runs-command`.

import { button, deckRow, inside, region, role, walk } from "../walk";

const needs = region("Needs you");
const strip = (command: string) => inside(needs, role("group", command));

const callCardRunsCommand = walk("call-card-runs-command", [
  { press: deckRow(/Debug Bridge on Merge line/), say: "A Job waits on commands to run: its row pulses, and pressing it opens the Job" },
  { press: inside(needs, button(/^Needs you/)), say: "The commands are in its Needs you panel" },
  { look: inside(strip("kill $(lsof -ti tcp:5788)"), button("Run")), say: "Under Commands, each command has its own Run" },
  { press: inside(strip("kill $(lsof -ti tcp:5788)"), button("Run")), say: "Run runs it in the Job's folder" },
  { press: inside(strip("pnpm vitest run packages/components"), button("Run")), say: "Exit 0 settled the first, and the agent is told. Run on the second command" },
  { look: inside(strip("pnpm vitest run packages/components"), button("Run again")), say: "Exit 1: the command stays, Run becomes Run again, and the agent is told nothing" },
]);

export { callCardRunsCommand };
