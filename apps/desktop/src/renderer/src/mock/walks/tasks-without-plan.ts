// Now with no plan lists the agent's own tasks in the order made, each its state mark, and the subagent at work under them.
// Over the `tasks-without-plan` scenario.

import { deckRow, inside, region, role, walk } from "../walk";

const now = region("Now", { exact: true });
const tasks = inside(now, role("list", "Tasks"));

export const tasksWithoutPlan = walk("tasks-without-plan", [
  { press: deckRow("Name the store migrations"), say: "It has no plan, and its agent keeps a task list" },
  { look: tasks, say: "Now lists the tasks in the order made, each its mark and its subject" },
  { hover: inside(tasks, role("img", "Working")), say: "Working" },
  { look: inside(now, role("list", "Subagents")), say: "The subagent at work stays under them" },
]);
