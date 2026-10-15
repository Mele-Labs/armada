// What a Session's waiting line offers the owner follows its act. A slash command is the Session's own
// and is sent to it, as a key on the rail beside the card; a shell command is run on the card or copied; a pull request opens; a plain line is a text box; only
// a question with options carries the standing answers. Over `session-errands`.

import { button, inside, region, role, text, walk } from "../walk";

const cockpit = region("Jobs and Sessions");
const line = region(/^Session: Land the speed fixes/);
const question = region(/^Session question: Land the speed fixes/);
const reply = inside(line, role("textbox", "Message"));

const sessionErrands = walk("session-errands", [
  { look: button(/Run/), say: "A slash command is for the Session itself: the key names it and sends it as the reply" },
  { look: reply, say: "A text box under it, and no standing answers" },
  { key: "l", on: cockpit, say: "Later" },
  { look: button(/Copy/), say: "A shell command is for him: Run runs it where the Session works, and Copy is beside it" },
  { look: inside(line, text("kill $(lsof -ti tcp:5788) && scripts/preview --restart --adopt")), say: "The command itself is a line on the card, in the case it was written" },
  { key: "l", on: cockpit, say: "Later" },
  { look: button(/Open #2118/), say: "A pull request to approve: the key opens it" },
  { key: "l", on: cockpit, say: "Later" },
  { look: reply, say: "A plain line has a text box and nothing else" },
  { look: button(/Open Session/), say: "The key opens the Session until something is typed" },
  { type: "Main", into: reply, say: "Typed words" },
  { look: inside(line, button(/Send/)), say: "Send takes them" },
  { key: "Escape", on: reply, say: "Leaves the box" },
  { key: "l", on: cockpit, say: "Later" },
  { look: inside(question, role("radio", /Make the best decision/)), say: "A question with options keeps the standing answers" },
]);

export { sessionErrands as "session-errands" };
