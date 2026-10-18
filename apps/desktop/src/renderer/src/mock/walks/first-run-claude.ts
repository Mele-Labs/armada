// Claude Code's own sessions as the first start: the folders it kept sessions for, ticked to add, and the Armada mod installed so a terminal
// session is a Job. Over the `first-run` scenario. Dispatch for my-app then offers how it runs, learned from those sessions.

import { button, inside, region, role, walk } from "../walk";

const stage = region("First Job");
const jobs = region("Jobs", { exact: true });
const dispatch = role("region", "Dispatch", { exact: true });
const runs = inside(dispatch, role("group", "How my-app runs"));

const firstRunClaude = walk("first-run", [
  { key: "3", on: role("group", "Start from"), say: "3 picks Import from Claude Code" },
  { press: inside(stage, role("checkbox", /scratch/)), say: "scratch is not a git repository, and is left out" },
  { press: inside(stage, role("switch", /^Install the Armada mod/)), say: "The mod shows terminal Claude Code sessions as Jobs" },
  { press: inside(stage, button("Add", { exact: true })), say: "Add takes the folders in" },
  { look: inside(jobs, button(/^Rate limit on the login route/)), say: "A terminal session is a Job" },
  { key: "n", on: jobs, say: "N opens Dispatch" },
  { press: inside(dispatch, role("radio", "Repository my-app")), say: "my-app is the repository" },
  { look: runs, say: "How my-app runs, from the Claude Code sessions" },
  { press: inside(dispatch, button("Remember", { exact: true })), say: "Remember asks where" },
  { key: "Enter", on: inside(runs, role("group", "Where to remember")), say: "↵ picks On this Mac" },
]);

export { firstRunClaude as "first-run-claude" };
