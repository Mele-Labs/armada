// What a Session's waiting line offers the owner follows its act. A slash command is the Session's own
// and is sent to it; a shell command is copied; a restart is the Fleet panel's button; a pull request opens; a plain line is a text box; only
// a question with options carries the standing answers. Over `session-errands`.

import { button, inside, region, role, walk } from "../walk";

const cockpit = region("Dashboard");
const line = region(/^Session: Land the speed fixes/);
const question = region(/^Session question: Land the speed fixes/);
const reply = inside(line, role("textbox", "Type something"));

const sessionErrands = walk("session-errands", [
  { look: inside(line, role("radio", /Run \/land 2109 2118 2119/)), say: "A slash command is for the Session itself: the button names it and sends it as the reply" },
  { look: reply, say: "A text box under it, and no standing answers" },
  { key: "l", on: cockpit, say: "Later" },
  { look: inside(line, role("radio", /Copy pnpm -C .armada\/preview install/)), say: "A shell command is for him: the button copies it" },
  { key: "l", on: cockpit, say: "Later" },
  { look: inside(line, role("radio", /Refresh preview/)), say: "A restart onto the preview is the Fleet panel's Refresh preview" },
  { key: "l", on: cockpit, say: "Later" },
  { look: inside(line, role("radio", /Open #2118/)), say: "A pull request to approve: the button opens it" },
  { key: "l", on: cockpit, say: "Later" },
  { look: reply, say: "A plain line has a text box and nothing else" },
  { look: inside(line, button("Open Session")), say: "Send is the way into the Session until something is typed" },
  { type: "Main", into: reply, say: "Typed words" },
  { look: inside(line, button("Send")), say: "Send takes them" },
  { key: "Escape", on: reply, say: "Leaves the box" },
  { key: "l", on: cockpit, say: "Later" },
  { look: inside(question, role("radio", /Make the best decision/)), say: "A question with options keeps the standing answers" },
]);

export { sessionErrands as "session-errands" };
