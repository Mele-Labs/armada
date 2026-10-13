// A command a Session waits on is run from its call card, where the Session works, by the path its
// ledger runs it by. A clean exit settles the item and the card goes; a failure stays with its output.
// Over `call-card-runs-command`.

import { button, inside, region, role, text, walk } from "../walk";

const cockpit = region("Dashboard");
const card = region(/^Session: Debug Bridge on Merge line/);

const callCardRunsCommand = walk("call-card-runs-command", [
  { look: inside(card, text("kill $(lsof -ti tcp:5788)")), say: "A command he runs: the command on the card, in the case it was written" },
  { look: button(/Run/), say: "Run is the key, and Copy stays beside it" },
  { look: button(/Copy/), say: "Copy" },
  { key: "Enter", on: cockpit, say: "Enter runs it in the Session's folder" },
  { look: inside(card, role("log", "Output")), say: "The output streams on the card under the command" },
  { look: inside(card, role("img", "Running")), say: "A running mark while it goes, and the card's other keys held" },
  { look: inside(card, role("img", "Passed")), say: "Exit 0: the mark turns to passed, and the card stays a moment" },
  { hover: inside(card, role("img", "Passed")), say: "The tooltip names the exit" },
  { look: inside(card, text("Run the components tests")), say: "The card has gone, the agent is told, and the next command is in front" },
  { press: button(/Run/), say: "Run it" },
  { look: inside(card, role("img", "Failed")), say: "Exit 1: the card stays with the output and the exit" },
  { hover: inside(card, role("img", "Failed")), say: "The tooltip names the exit" },
  { look: button(/Run again/), say: "Run becomes Run again, and the agent is told nothing" },
  { look: inside(card, role("textbox", "Message")), say: "The Session's own box under it, to say what happened" },
]);

export { callCardRunsCommand };
