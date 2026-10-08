// Command Central with a good deal waiting on the owner: the queue on the left, the selected call on
// the right with what it was asked, where it is and why it asks. Running is the fleet board, each
// step named under its pip, a lane opening the same pane. Done is the list beneath. Over
// `dashboard-needs-you`.

import { button, inside, region, role, tab, text, walk } from "../walk";

const queue = role("listbox", "Needs you");
const call = (title: RegExp) => inside(queue, role("option", title));

const dashboardNeedsYou = walk("dashboard-needs-you", [
  { look: tab("Command Central"), say: "The tabs: a glyph each, a lit underline" },
  { look: queue, say: "Everything that needs you, one row each" },
  { look: button(/#1742/), say: "Where it came from: the issue, pressed to open it" },
  { press: button(/writer\.rs/), say: "The files it touches" },
  { look: text("Job diff"), say: "What it changed, in a panel" },
  { press: button("Close"), say: "Close" },
  { look: region("What was asked for"), say: "What the Job was asked for" },
  { look: region("Where it is"), say: "Where it is in its workflow" },
  { look: region("Why it asks"), say: "Why it asks" },
  { look: role("radiogroup", "Split the clock out of the writer, or wrap it in place?"), say: "The question" },
  { press: text("Split it out"), say: "Pick an answer" },
  { press: button("Answer"), say: "Answer" },
  { look: queue, say: "The next call steps up" },
  { press: call(/store: 2 failed/), say: "A failed Check" },
  { look: region("Check output"), say: "Its output" },
  { press: call(/No output for 14m/), say: "A Drone gone quiet" },
  { look: text("Tell the Drone"), say: "Tell it, restart the step, or start a fresh Drone" },
  { press: call(/Flaky store test/), say: "A Session" },
  { look: button("Open Session"), say: "The Session itself, answered here" },
  { press: call(/main is red/), say: "Main is red" },
  { look: region("What broke"), say: "What broke" },
  { press: call(/#1819/), say: "A failing pull request" },
  { press: button("Send a Drone"), say: "Send a Drone to look at it" },
  { look: text("What the Drone would change"), say: "What it would change" },
  { look: button("Push to #1819"), say: "It pushes only when you say" },
  { press: tab("Running"), say: "Running" },
  { look: region("Fleet"), say: "Each step named under its pip" },
  { press: inside(region("Fleet"), role("option", /Shorten the reconnect wait/)), say: "A lane" },
  { look: role("article", /Shorten the reconnect wait/), say: "Its pane" },
  { press: tab("Done"), say: "Done" },
  { look: tab("Done"), say: "Done" },
]);

export { dashboardNeedsYou as "dashboard-needs-you" };
