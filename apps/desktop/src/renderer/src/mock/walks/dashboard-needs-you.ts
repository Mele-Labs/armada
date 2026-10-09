// The calls a Dashboard can raise at once. An agent's question carries the standing answers; a call
// Fleet raises about a state keeps its own act; and a pull request or main's red that a Session or a
// Job already owns sends him to that owner instead of offering a new Drone. Over `dashboard-needs-you`.

import { inside, region, role, text, walk } from "../walk";

const cockpit = region("Dashboard");
const call = (kind: RegExp) => region(kind);
const answer = (card: RegExp, name: RegExp) => inside(call(card), role("radio", name));

const dashboardNeedsYou = walk("dashboard-needs-you", [
  { look: call(/^Plan question/), say: "Several calls wait. The first is an agent's question, in front of the panel" },
  { look: answer(/^Plan question/, /Make the best decision/), say: "An agent's question carries the two standing answers" },
  { key: "l", on: cockpit, say: "Later, down the stack" },
  { look: call(/^Judge question/), say: "A Judge's question: the standing answers again" },
  { key: "l", on: cockpit, say: "Later" },
  { look: call(/^Check failed/), say: "A failed Check is raised by Fleet about a state, and keeps only its own act" },
  { key: "l", on: cockpit, say: "Later" },
  { look: call(/^Drone stuck/), say: "A stuck Drone is the same" },
  { key: "l", on: cockpit, say: "Later" },
  { look: call(/^Job/), say: "A Job at its review" },
  { key: "l", on: cockpit, say: "Later" },
  { look: call(/^Session question/), say: "A Session's own question, whole, with each option and the agent's line under it" },
  { look: text(/Nothing you see changes either way. What should happen to it\?/), say: "The question is whole, down to its last sentence" },
  { look: inside(call(/^Session question/), role("radio", /File an issue/)), say: "The recommended option is marked, and nothing here is Allow or Deny" },
  { look: inside(call(/^Session question/), role("textbox", "Type something")), say: "Type something is a reply in words, on t" },
  { key: "l", on: cockpit, say: "Later" },
  { look: call(/^Session walk/), say: "A page the Session wants him to look at: Approve" },
  { look: answer(/^Session walk/, /Approve/), say: "A walk's act is Approve" },
  { key: "l", on: cockpit, say: "Later" },
  { look: inside(call(/^Session: Docs/), role("textbox", "Type something")), say: "A question with no options is a reply in words" },
  { key: "l", on: cockpit, say: "Later" },
  { look: answer(/^Session: Flaky/, /Allow once/), say: "A permission offers what it was offered" },
  { look: answer(/^Session: Flaky/, /Just get it done/), say: "And the standing answers hand the decision to the agent" },
  { key: "l", on: cockpit, say: "Later" },
  { look: call(/^Main/), say: "Main is red, and a Job's pull request broke it" },
  { look: answer(/^Main/, /Open the Job/), say: "That Job is on it: the answer is to open it, and no Drone is offered" },
  { look: answer(/^Main/, /Poke/), say: "Or poke it: a nudge that the checks failed" },
  { key: "l", on: cockpit, say: "Later" },
  { look: call(/^Pull request: #1819/), say: "A failing pull request that a Job opened" },
  { look: answer(/^Pull request: #1819/, /Open the Job/), say: "The Job is named, and opening it is the answer" },
  { key: "l", on: cockpit, say: "Later" },
  { look: call(/^Pull request: #1822/), say: "A failing pull request a Session holds" },
  { look: inside(call(/^Pull request: #1822/), role("group", "Owner")), say: "The Session that owns it is named, with its glyph" },
  { look: answer(/^Pull request: #1822/, /Open the Session/), say: "1 opens it, so he can see it is being addressed. o still opens the pull request" },
  { look: answer(/^Pull request: #1822/, /Poke/), say: "2 pokes it: a short message to the Session that the checks failed" },
  { key: "l", on: cockpit, say: "Later" },
  { look: call(/^Pull request: #1823/), say: "A person's pull request, with nobody on it" },
  { look: answer(/^Pull request: #1823/, /Send a Drone/), say: "1 sends a Drone" },
  { look: answer(/^Pull request: #1823/, /Attach/), say: "2 attaches it to a Job or a Session that is already on it" },
  { key: "2", on: cockpit, say: "Attach" },
  { key: "Enter", on: cockpit, say: "Enter opens the picker" },
  { look: inside(call(/^Pull request: #1823/), role("listbox", "Jobs and Sessions")), say: "Every live Job and Session, a filter over them" },
  { look: inside(call(/^Pull request: #1823/), role("img", "Job")), say: "A Job carries its glyph, and picking one claims the pull request for it" },
  { type: "Migration notes", into: role("textbox", "Attach to"), say: "Typing narrows it" },
  { key: "Enter", on: role("textbox", "Attach to"), say: "Enter attaches it" },
  { look: inside(call(/^Pull request: #1823/), role("group", "Owner")), say: "The card switches to its owned form, naming the Session" },
  { look: answer(/^Pull request: #1823/, /Open the Session/), say: "Opening it is the answer now" },
]);

export { dashboardNeedsYou as "dashboard-needs-you" };
