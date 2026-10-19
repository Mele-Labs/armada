// A new project as the first start: a name and a place, a folder made, then the Cockpit with an empty Job on it, where its runs are kept as a line above the
// box, and a Job whose first event is git init. Over the `first-run-new` scenario, whose folder picker answers ~/Developer.

import { button, inside, region, role, text, walk } from "../walk";

const stage = region("First Job");
const jobs = region("Jobs", { exact: true });
const message = role("textbox", "Message");
const thread = region("Thread");

const firstRunNew = walk("first-run-new", [
  { key: "2", on: role("group", "Start from"), say: "2 picks New project" },
  { type: "my-app", into: inside(stage, role("textbox", "Name")), say: "The name" },
  { press: inside(stage, button(/^Location/)), say: "The location opens the picker, and ~/Developer is chosen" },
  { press: inside(stage, button("Create", { exact: true })), say: "Create makes ~/Developer/my-app" },
  { look: inside(jobs, button("my-app", { exact: true })), say: "A new Job is open on it, with where its Armada configuration is kept in a line above the box" },
  { type: "Start a settings page with a dark mode toggle", into: message, say: "The first message" },
  { press: button("Send", { exact: true }), say: "Send starts the Job" },
  { press: inside(thread, text("git, Write, Edit")), say: "The first calls are one closed row, and it opens" },
  { look: inside(thread, text(/^git init/)), say: "The first event is git init" },
]);

export { firstRunNew as "first-run-new" };
