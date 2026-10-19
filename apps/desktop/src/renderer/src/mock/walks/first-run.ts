// From opening Armada to a first Job on a folder, and Armada keeping how the folder runs so the next agent does not guess. Open a project asks for the
// folder and lands on the Cockpit with an empty Job and its box ready; where Armada configuration is kept is on the Job itself, by `?take=`: a chip in its
// header (the default), a card at the top of its thread, or a chip in the box's toolbar. The Job tries npm test, which fails, then pnpm test; only what
// passed is kept. A second Job shows the place as a glyph. Over the `first-run` scenario.

import { button, dialog, inside, region, role, text, walk } from "../walk";
import { chip, places, take, where } from "../place-walk";

const jobs = region("Jobs", { exact: true });
const message = role("textbox", "Message");
const glyph = button("Armada configuration kept on this Mac");
const dispatch = dialog("Dispatch");
const thread = region("Thread");
const picker = role("combobox", "Repository");
const layer = role("dialog", "Armada configuration for my-app");

const firstRun = walk("first-run", [
  { key: "1", on: role("group", "Start from"), say: "1 picks Open a project. The macOS folder dialog opens here; the mock skips it and picks ~/code/my-app" },
  { look: inside(jobs, button("my-app", { exact: true })), say: "A new Job is open on my-app, empty, with the caret in its box" },
  ...(take === "thread"
    ? [{ look: places, say: `Where its Armada configuration is kept is ${where}, On this Mac picked` } as const]
    : [
        { press: chip, say: `Where its Armada configuration is kept is ${where}, On this Mac` } as const,
        { look: places, say: "The four places: this Mac, the repository, a synced folder, cloud storage" } as const,
        { key: "Escape", on: places, say: "Esc keeps On this Mac and goes back to the box" } as const,
      ]),
  { type: "Add a dark mode toggle to settings", into: message, say: "The first message" },
  { press: button("Send", { exact: true }), say: "Send starts the work" },
  { later: thread, say: "The Job guesses the test command" },
  { look: inside(thread, text(/^npm test$/)), say: "npm test exited 1, then pnpm test exited 0" },
  { later: thread, say: "The Job ends" },
  { key: "n", on: jobs, say: "N opens Dispatch for a second Job" },
  { type: "Add a dark mode toggle to the docs", into: inside(dispatch, role("textbox", "Request")), say: "A request on my-app" },
  { key: "Enter", on: inside(dispatch, role("textbox", "Request")), say: "↵ starts the Job" },
  { hover: glyph, say: "A later Job keeps only the glyph, and its tooltip names the place" },
  { press: picker, say: "The repository picker opens" },
  { key: "ArrowDown", on: picker, say: "my-app's row wears the laptop, where its Armada configuration is kept" },
  { key: "ArrowRight", on: picker, say: "→ opens Armada configuration for my-app" },
  { look: inside(layer, role("img", "Tests: Kept on this Mac")), say: "pnpm test passed and is kept on this Mac. npm test failed and is not there" },
  { look: inside(layer, role("img", "Dev server: Kept on this Mac")), say: "The dev server, on :5173, is kept the same way" },
]);

export { firstRun as "first-run" };
