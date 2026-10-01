// Text a model or a person wrote on the Plan draws as markdown: the approach,
// why a task stopped, its brief, what its work showed, and why it was dropped.

import { button, dialog, inside, region, role, tab, text, walk } from "../walk";

const T6 = "Open a Drone's Job from its row";
const T7 = "Say what holds the next Drone back";

export const markdownFromAgents = walk("arc/group-failed", [
  { press: tab("Plan"), say: "The plan" },
  { press: role("tab", "List", { exact: true }), say: "As its groups, in a list" },
  { look: region("The approach"), say: "The approach: its list, its bold and its code drawn rather than spelled" },
  { look: role("listitem", /^T6 /), say: "Why T6 stopped, under its row: a list, with code and bold" },
  { press: inside(role("listitem", /^T6 /), button(T6)), say: "T6's panel" },
  { look: inside(dialog(T6), text("ran on every row")), say: "Why it stopped, in its panel: the same list" },
  { look: inside(dialog(T6), text("Read the id from")), say: "The planner's brief: bold and code" },
  { look: inside(dialog(T6), text("in running-rows.test.tsx")), say: "What the work showed: bold and code" },
  { press: inside(dialog(T6), button("Close")), say: "Back to the list" },
  { press: inside(role("listitem", /^T7 /), button(T7)), say: "An open task's panel" },
  { press: button("Drop this task"), say: "Dropping it asks why" },
  {
    type: "Covered by **T5**: it reads `drones_running` already.",
    into: role("textbox", "Reason"),
    say: "A reason, written in markdown",
  },
  { press: button("Drop", { exact: true }), say: "Drop it" },
  { look: inside(dialog(T7), text("it reads drones_running already")), say: "Why it was dropped: bold and code" },
]);
