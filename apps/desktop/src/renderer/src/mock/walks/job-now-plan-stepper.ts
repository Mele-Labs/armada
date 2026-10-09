// A Plan question asked one decision at a time, with marks for where it stands and the answer at the last.

import { button, inside, region, role, text, walk } from "../walk";

const now = region("Now");

const w = walk("job-now-plan", [
  { look: inside(now, text("Plan question")), say: "The decision has its header" },
  { look: inside(now, role("radiogroup", "Split the clock out of the writer, or wrap it in place?")), say: "The first decision, nothing preselected" },
  { look: inside(now, role("list", "Decisions")), say: "Marks for where it stands among the decisions" },
  { press: inside(now, role("radio", "Wrap it in place")), say: "Pick an answer" },
  { press: inside(now, button("Next")), say: "Next goes to the following decision" },
  { press: inside(now, role("radio", "Add a fake clock")), say: "The second" },
  { press: inside(now, button("Next")), say: "and the third" },
  { press: inside(now, role("radio", "Take it here")), say: "The last carries the answer" },
  { press: inside(now, button("Answer")), say: "and sends all three" },
  { look: inside(now, text("Judge question")), say: "The questions of a Judge and a Drone sit under their own headers" },
]);

export { w as "job-now-plan-stepper" };
