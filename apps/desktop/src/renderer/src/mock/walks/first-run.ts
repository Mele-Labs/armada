// From opening Armada to a first Job on a folder, and Armada keeping how the folder runs so the next agent does not guess. The place for the runs is
// asked once, after the folder. The Job tries npm test, which fails, then pnpm test; only what passed is kept. Over the `first-run` scenario.

import { button, inside, region, role, text, walk } from "../walk";

const stage = region("First Job");
const request = inside(stage, role("textbox", "Request"));
const places = inside(stage, role("group", "Where runs are kept"));
const thread = region("Thread");
const picker = role("combobox", "Repository");
const layer = role("dialog", "All of how my-app runs");

const firstRun = walk("first-run", [
  { key: "1", on: role("group", "Start from"), say: "1 picks Open a project" },
  { key: "Meta+o", on: request, say: "⌘O picks ~/code/my-app, a folder with no Manifest" },
  { look: places, say: "Where its runs are kept is asked once: this Mac, the repository, a synced folder, cloud storage" },
  { key: "Enter", on: places, say: "↵ picks On this Mac" },
  { type: "Add a dark mode toggle to settings", into: request, say: "The request" },
  { press: inside(stage, button("Start", { exact: true })), say: "Start opens the Job" },
  { later: thread, say: "The Job guesses the test command" },
  { look: inside(thread, text(/^npm test$/)), say: "npm test exited 1, then pnpm test exited 0" },
  { later: thread, say: "The Job ends" },
  { press: picker, say: "The repository picker opens" },
  { key: "ArrowDown", on: picker, say: "my-app's row wears the laptop, the place its runs are kept in" },
  { key: "ArrowRight", on: picker, say: "→ opens all of how my-app runs" },
  { look: inside(layer, role("img", "Tests: Kept on this Mac")), say: "pnpm test passed and is kept on this Mac. npm test failed and is not there" },
  { look: inside(layer, role("img", "Dev server: Kept on this Mac")), say: "The dev server, on :5173, is kept the same way" },
]);

export { firstRun as "first-run" };
