// A new project as the first start: a name and a place, a folder made, where its runs are kept, then a request and a Job whose first event is git init. Over the
// `first-run-new` scenario, whose folder picker answers ~/Developer.

import { button, inside, region, role, text, walk } from "../walk";

const stage = region("First Job");
const request = inside(stage, role("textbox", "Request"));
const places = inside(stage, role("group", "Where runs are kept"));
const thread = region("Thread");

const firstRunNew = walk("first-run-new", [
  { key: "2", on: role("group", "Start from"), say: "2 picks New project" },
  { type: "my-app", into: inside(stage, role("textbox", "Name")), say: "The name" },
  { press: inside(stage, button(/^Location/)), say: "The location opens the picker, and ~/Developer is chosen" },
  { press: inside(stage, button("Create", { exact: true })), say: "Create makes ~/Developer/my-app" },
  { key: "Enter", on: places, say: "↵ picks On this Mac, asked once for the new folder" },
  { type: "Start a settings page with a dark mode toggle", into: request, say: "The request" },
  { press: inside(stage, button("Start", { exact: true })), say: "Start opens the Job" },
  { press: inside(thread, text("git, Write, Edit")), say: "The first calls are one closed row, and it opens" },
  { look: inside(thread, text(/^git init/)), say: "The first event is git init" },
]);

export { firstRunNew as "first-run-new" };
