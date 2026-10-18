// From opening Armada to a first Job on a folder, and Armada remembering how the folder runs so the next agent does not guess. The Job tries
// npm test, which fails, then pnpm test. Over the `first-run` scenario. Dispatch for the next Job offers how my-app runs, and the first Remember asks where.

import { button, inside, region, role, text, walk } from "../walk";

const stage = region("First Job");
const request = inside(stage, role("textbox", "Request"));
const thread = region("Thread");
const dispatch = role("region", "Dispatch", { exact: true });
const runs = inside(dispatch, role("group", "How my-app runs"));
const places = inside(runs, role("group", "Where to remember"));

const firstRun = walk("first-run", [
  { key: "1", on: role("group", "Start from"), say: "1 picks Open a project" },
  { key: "Meta+o", on: request, say: "⌘O picks ~/code/my-app, a folder with no Manifest" },
  { type: "Add a dark mode toggle to settings", into: request, say: "The request" },
  { press: inside(stage, button("Start", { exact: true })), say: "Start opens the Job" },
  { later: thread, say: "The Job guesses the test command" },
  { look: inside(thread, text(/^npm test$/)), say: "npm test exited 1, then pnpm test exited 0" },
  { later: thread, say: "The Job ends" },
  { key: "n", on: region("Jobs", { exact: true }), say: "N opens Dispatch for the next Job" },
  { look: runs, say: "Under the request, how my-app runs, from the last Job" },
  { press: inside(dispatch, button("Remember", { exact: true })), say: "Remember asks where. It asks once for my-app" },
  { look: places, say: "This Mac, the repository, a synced folder, cloud storage" },
  { key: "Enter", on: places, say: "↵ picks On this Mac" },
  { look: inside(runs, role("img", "Tests: Remembered on this Mac")), say: "Each row wears the laptop, kept in the Kit" },
]);

export { firstRun as "first-run" };
