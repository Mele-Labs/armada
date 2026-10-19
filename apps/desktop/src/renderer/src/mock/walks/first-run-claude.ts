// Claude Code's own sessions as the first start: the folders it kept sessions for, ticked to add, and the Armada mod installed so a terminal
// session is a Job. The place for the runs is asked once for all the folders. Over the `first-run` scenario. How my-app runs was found in those sessions.

import { button, inside, region, role, walk } from "../walk";

const stage = region("First Job");
const jobs = region("Jobs", { exact: true });
const places = inside(stage, role("group", "Where runs are kept"));
const picker = role("combobox", "Repository");
const layer = role("dialog", "All of how my-app runs");

const firstRunClaude = walk("first-run", [
  { key: "3", on: role("group", "Start from"), say: "3 picks Import from Claude Code" },
  { press: inside(stage, role("checkbox", /scratch/)), say: "scratch is not a git repository, and is left out" },
  { press: inside(stage, role("switch", /^Install the Armada mod/)), say: "The mod shows terminal Claude Code sessions as Jobs" },
  { press: inside(stage, button("Add", { exact: true })), say: "Add asks where runs are kept, once for the folders being added" },
  { key: "Enter", on: places, say: "↵ picks On this Mac" },
  { look: inside(jobs, button(/^Rate limit on the login route/)), say: "A terminal session is a Job" },
  { press: picker, say: "The repository picker opens" },
  { key: "ArrowDown", on: picker, say: "my-app's row wears the laptop" },
  { key: "ArrowRight", on: picker, say: "→ opens all of how my-app runs" },
  { look: inside(layer, role("img", "Tests: Kept on this Mac")), say: "What passed in the Claude Code sessions is kept on this Mac" },
]);

export { firstRunClaude as "first-run-claude" };
