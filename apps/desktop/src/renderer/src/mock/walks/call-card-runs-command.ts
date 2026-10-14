// A command a Session waits on is run from its call card, where the Session works, by the path its
// ledger runs it by. A clean exit settles the item and the card goes; a failure stays with its output.
// Over `call-card-runs-command`.

import { button, inside, region, role, text, walk } from "../walk";

const cockpit = region("Dashboard");
const card = region(/^Session: Debug Bridge on Merge line/);

const callCardRunsCommand = walk("call-card-runs-command", [
  { key: "w", on: cockpit, say: "w opens the first call in the tray" },
  { look: inside(card, text("kill $(lsof -ti tcp:5788)")), say: "A command he runs: the command on the card, in the case it was written" },
  { hover: inside(card, text("kill $(lsof -ti tcp:5788)")), say: "Hover the command: the same play control as in the Session" },
  { look: inside(card, button("Run")), say: "Play, on the command's own block" },
  { look: button(/Run/), say: "Run is the key, and Copy stays beside it" },
  { look: button(/Copy/), say: "Copy" },
  { key: "Enter", on: cockpit, say: "Enter runs it in the Session's folder" },
  { look: inside(card, role("log", "Output")), say: "The output streams on the card under the command" },
  { look: inside(card, role("img", "Running")), say: "A running mark while it goes, and the card's other keys held" },
  { look: inside(card, role("img", "Passed")), say: "Exit 0: the mark turns to passed, and the card stays a moment" },
  { hover: inside(card, role("img", "Passed")), say: "The tooltip names the exit" },
  { look: text("It ran clean."), say: "The card has gone and the agent is told: its reply is on the Session behind" },
  { key: "w", on: cockpit, say: "The next command waits in the tray, and w opens it" },
  { look: inside(card, text("Run the components tests")), say: "The next command is in front" },
  { press: inside(inside(card, role("group", "pnpm vitest run packages/components")), button("Run")), say: "Play on the command runs it too, with the key's state" },
  { look: inside(card, role("img", "Failed")), say: "Exit 1: the card stays with the output and the exit" },
  { hover: inside(card, role("img", "Failed")), say: "The tooltip names the exit" },
  { look: button(/Run again/), say: "Run becomes Run again, and the agent is told nothing" },
  { look: inside(card, role("textbox", "Message")), say: "The Session's own box under it, to say what happened" },
]);

export { callCardRunsCommand };
