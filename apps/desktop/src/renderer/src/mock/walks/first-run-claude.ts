// Claude Code's own sessions as the first start: the folders it kept sessions for, ticked to add, and the Armada mod installed so a terminal
// session is a Job. Over the `first-run` scenario. my-app then holds how it runs, learned from those sessions, in the repository picker.

import { button, inside, region, role, walk } from "../walk";

const stage = region("First Job");
const jobs = region("Jobs", { exact: true });

const firstRunClaude = walk("first-run", [
  { key: "3", on: role("group", "Start from"), say: "3 picks My Claude Code sessions" },
  { press: inside(stage, role("checkbox", /scratch/)), say: "scratch is not a git repository, and is left out" },
  { press: inside(stage, role("switch", /^Install the Armada mod/)), say: "The mod shows terminal Claude Code sessions as Jobs" },
  { press: inside(stage, button("Add", { exact: true })), say: "Add takes the folders in" },
  { look: inside(jobs, button(/^Rate limit on the login route/)), say: "A terminal session is a Job" },
  { press: role("combobox", "Repository"), say: "The picker lists the folders" },
  { press: role("button", "How my-app runs"), say: "my-app carries a mark, from the past sessions" },
  { press: inside(role("group", "How my-app runs"), button(/^Remember/)), say: "Remember keeps what was seen" },
]);

export { firstRunClaude as "first-run-claude" };
