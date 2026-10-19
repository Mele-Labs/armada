// From opening Armada to a first Job on a folder, and Armada keeping how the folder runs so the next agent does not guess. Open a project asks for the
// folder and lands on the Cockpit with an empty Job and its box ready; where the runs are kept is a line above the box. The Job tries npm test, which fails,
// then pnpm test; only what passed is kept. Over the `first-run` scenario.

import { button, inside, region, role, text, walk } from "../walk";

const jobs = region("Jobs", { exact: true });
const message = role("textbox", "Message");
const line = button(/^Armada configuration kept on this Mac/);
const places = role("group", "Where runs are kept");
const thread = region("Thread");
const picker = role("combobox", "Repository");
const layer = role("dialog", "All of how my-app runs");

const firstRun = walk("first-run", [
  { key: "1", on: role("group", "Start from"), say: "1 picks Open a project, and the folder dialog answers ~/code/my-app" },
  { look: inside(jobs, button("my-app", { exact: true })), say: "A new Job is open on my-app, empty, with the caret in its box" },
  { press: line, say: "Where its runs are kept is one line above the box, On this Mac" },
  { look: places, say: "The four places: this Mac, the repository, a synced folder, cloud storage" },
  { key: "Escape", on: places, say: "Esc keeps On this Mac and goes back to the box" },
  { type: "Add a dark mode toggle to settings", into: message, say: "The first message" },
  { press: button("Send", { exact: true }), say: "Send starts the work, and the line goes" },
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
