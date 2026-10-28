// Claude Code's own sessions as the first start: the folders it kept sessions for, ticked to add, and the Armada mod installed so a terminal
// session is a Job. Add opens a Job on the first folder, and where Armada configuration is kept, a card at the top of the Job's thread, is for all the folders. Over the `first-run`
// scenario. How my-app runs was found in those sessions.

import { button, inside, region, role, walk } from "../walk";

const stage = region("First Job");
const jobs = region("Jobs", { exact: true });
const picker = role("button", "Repository", { exact: true });
const find = role("searchbox", "Find a repository");
const row = role("menuitem", /^my-app/);
const layer = role("dialog", "Armada configuration for my-app");
const places = role("group", "Where Armada configuration is kept");

const firstRunClaude = walk("first-run", [
  { key: "3", on: role("group", "Start from"), say: "3 picks Import from Claude Code" },
  { press: inside(stage, role("checkbox", /scratch/)), say: "scratch is not a git repository, and is left out" },
  { press: inside(stage, role("switch", /^Install the Armada mod/)), say: "The mod shows terminal Claude Code sessions as Jobs" },
  { press: inside(stage, button("Add", { exact: true })), say: "Add opens a Job on my-app, the first folder" },
  { look: places, say: "Where Armada configuration is kept is a card at the top of its thread, for all the folders added" },
  { look: inside(jobs, button(/^Rate limit on the login route/)), say: "A terminal session is a Job" },
  { press: picker, say: "The repository picker opens" },
  { key: "ArrowDown", on: find, say: "↓ goes to the list" },
  { key: "ArrowDown", on: role("menuitem", "All repositories"), say: "my-app's row wears the laptop" },
  { key: "ArrowRight", on: row, say: "→ opens Armada configuration for my-app" },
  { look: inside(layer, role("img", "Tests: Kept on this Mac")), say: "What passed in the Claude Code sessions is kept on this Mac" },
]);

export { firstRunClaude as "first-run-claude" };
