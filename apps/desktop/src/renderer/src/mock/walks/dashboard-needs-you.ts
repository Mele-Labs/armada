// The calls a Dashboard can raise at once. An agent's question carries the standing answers; a call
// Fleet raises about a state keeps its own act; and a pull request or main's red that a Session or a
// Job already owns sends him to that owner instead of offering a new Drone. Over `dashboard-needs-you`.

import { inside, region, role, walk } from "../walk";

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
  { look: call(/^Session/), say: "A Session waiting on a command is an agent asking, so the standing answers stand" },
  { key: "l", on: cockpit, say: "Later" },
  { look: call(/^Main/), say: "Main is red, and a Job's pull request broke it" },
  { look: answer(/^Main/, /Open the Job/), say: "That Job is on it: the answer is to open it, and no Drone is offered" },
  { key: "l", on: cockpit, say: "Later" },
  { look: call(/^Pull request: #1819/), say: "A failing pull request that a Job opened" },
  { look: answer(/^Pull request: #1819/, /Open the Job/), say: "The Job is named, and opening it is the answer" },
  { key: "l", on: cockpit, say: "Later" },
  { look: call(/^Pull request: #1822/), say: "A failing pull request a Session holds" },
  { look: inside(call(/^Pull request: #1822/), role("group", "Owner")), say: "The Session that owns it is named, with its glyph" },
  { look: answer(/^Pull request: #1822/, /Open the Session/), say: "Opening it is the answer, so he can see it is being addressed. o still opens the pull request" },
  { key: "l", on: cockpit, say: "Later" },
  { look: call(/^Pull request: #1823/), say: "A person's pull request, with nobody on it" },
  { look: answer(/^Pull request: #1823/, /Send a Drone/), say: "Nobody owns it, so a Drone is what he can send" },
]);

export { dashboardNeedsYou as "dashboard-needs-you" };
