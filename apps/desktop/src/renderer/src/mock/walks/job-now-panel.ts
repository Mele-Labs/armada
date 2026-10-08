// The Now panel beside a Job's Overview canvas: nothing in it for a quiet Job, then a Drone's live
// last action, Checks passing and failing as they finish, a Plan decision to answer, a Judge
// reading, and an issue. It hides from its head and shows again. Over the `job-now-panel` scenario.

import { button, card, inside, region, role, text, walk } from "../walk";

const now = region("Now");
const board = button("Overview", { exact: true });

const jobNowPanel = walk("job-now-panel", [
  { press: role("option", "Tidy the store fixtures"), say: "A Job with nothing going on" },
  { look: now, say: "The panel is its head alone" },
  { press: board, say: "Back to the Board" },
  { press: role("option", "Pin the store clock"), say: "A Job with a Drone at work" },
  { look: inside(now, role("heading", "Running")), say: "Running" },
  { look: inside(now, role("img", "Running")), say: "The Drone's mark breathes, and its tooltip names it" },
  { look: inside(now, role("button", /Edit crates\/store\/src\/clock.rs/)), say: "Its last action, live" },
  { look: inside(now, button("Show Implement on the canvas")), say: "The step it belongs to" },
  { press: inside(now, button("Show Implement on the canvas")), say: "A press highlights that step on the canvas" },
  { look: card("Implement"), say: "Implement, marked" },
  { press: inside(now, button("Output of Drone on Implement")), say: "Its output tail opens" },
  { look: inside(now, text(/Read crates\/store\/src\/writer.rs/)), say: "The last few lines, in mono" },
  { press: board, say: "Back to the Board" },
  { press: role("option", "Cache the manifest read"), say: "A Job whose Checks are landing" },
  { look: inside(now, role("img", "Passed")), say: "A Check that passed" },
  { look: inside(now, role("img", "Failed")), say: "and one that failed, each marked as it finished" },
  { press: board, say: "Back to the Board" },
  { press: role("option", "Split the writer from the clock"), say: "A Job with a Plan decision open" },
  { look: inside(now, role("heading", "Asks you")), say: "Asks you" },
  { look: inside(now, role("radiogroup", "Split the clock out of the writer, or wrap it in place?")), say: "The first decision, nothing preselected" },
  { look: inside(now, role("list", "Decisions")), say: "Marks for where it stands among the decisions" },
  { press: inside(now, role("radio", "Wrap it in place")), say: "Pick an answer" },
  { press: inside(now, button("Next")), say: "Next goes to the following decision" },
  { press: inside(now, role("radio", "Add a fake clock")), say: "The second" },
  { press: inside(now, button("Next")), say: "and the third" },
  { press: inside(now, role("radio", "Take it here")), say: "The last carries the answer" },
  { press: inside(now, button("Answer")), say: "and sends all three" },
  { look: inside(now, role("button", /Judge on Implement, asks you/)), say: "A Judge's question is a row that opens the Judge" },
  { look: inside(now, role("button", /Drone on Implement, asks you/)), say: "and a Drone's, the Drone" },
  { press: board, say: "Back to the Board" },
  { press: role("option", "Cap the retry backoff"), say: "A Job with a Judge reading" },
  { look: inside(now, role("button", /Judge on Implement, running/)), say: "The Judge, running" },
  { press: inside(now, button("Show Implement on the canvas")), say: "Its step, highlighted on the canvas" },
  { look: card("Implement"), say: "Implement, marked" },
  { press: board, say: "Back to the Board" },
  { press: role("option", "Shorten the reconnect wait"), say: "A Job with an issue" },
  { look: inside(now, role("heading", "Issues")), say: "Issues" },
  { look: inside(now, role("img", "Drone stuck")), say: "A stuck Drone" },
  { look: inside(now, role("img", "Check failed")), say: "and a failed Check, each a row that opens it" },
  { press: button("Hide now"), say: "The head's button hides the panel" },
  { press: button("Show now"), say: "and a button beside the cards shows it again" },
]);

export { jobNowPanel as "job-now-panel" };
