// Projects, mock only: open the tab on All, open Pocket launch, add a task, link two issues, select three rows and dispatch
// them, a row a live Job holds, the done ask on the row and on the Job, then the palette adds a task and a project from
// another page. Over the `projects` scenario.

import { button, dialog, inside, place, region, role, text, walk } from "../walk";

const places = role("navigation", "Places");
const list = region("Projects", { exact: true });
const pocket = region("Project Pocket launch");
const bar = inside(dialog("Command palette"), role("combobox"));
const needs = region("Needs you");

const projects = walk("projects", [
  { press: place("Projects"), say: "Projects, a panel for each repository: armada's and site's on All" },
  { key: "l", on: list, say: "Keys: l the list, r the rows, a add a task, i link issues, x select, d tick, s dispatch, ⌥↑ ⌥↓ move a row" },
  { press: inside(list, button("Pocket launch")), say: "Pocket launch: tasks and issues in one list, in his order, each with the Job holding it" },
  { type: "Draft the App Store text\n", into: role("textbox", "Add a task"), say: "A task, written at the foot. Enter adds it" },
  { press: inside(pocket, button("Link issues")), say: "Link issues searches armada's open issues" },
  { press: button(/^#2812 /), say: "#2812 joins the list" },
  { press: button(/^#2815 /), say: "And #2815" },
  { key: "Escape", on: role("textbox", "Search issues"), say: "Esc puts the search away" },
  { press: role("checkbox", "Select Test on a small phone"), say: "Select three rows" },
  { press: role("checkbox", "Select Draft the App Store text"), say: "Two" },
  { press: role("checkbox", /^Select #2812 /), say: "Three" },
  { press: inside(pocket, button("Dispatch", { exact: true })), say: "Dispatch: the sheet on armada, the tasks as lines and the issue as a tag. Starting is the sheet's" },
  { key: "Escape", on: role("textbox", "Request"), say: "Esc puts the sheet away" },
  { hover: role("checkbox", "In a Job: Pair a phone by QR code"), say: "A live Job holds this task: its glyph, and the Job under the title" },
  { hover: role("checkbox", "Done?: Write the install guide"), say: "This one's Job has ended, and the row still asks whether it is done" },
  { press: inside(pocket, button("Sign the release build")), say: "Sign the build asks too. Its Job finished and is live: its title opens it" },
  { press: inside(needs, button(/^Needs you/)), say: "Needs you on the Job asks the same" },
  { look: inside(needs, text("Mark Sign the build done?")), say: "Mark Sign the build done?" },
  { press: inside(needs, role("radio", "Mark done", { exact: true })), say: "Mark done" },
  { press: inside(needs, button("Answer")), say: "Answered here, so the row is answered too" },
  { press: place("Projects"), say: "Back on Projects" },
  { look: role("checkbox", "Done: Sign the build"), say: "Sign the build is done" },
  { press: place("Logbook"), say: "Another page" },
  { key: "Meta+k", on: places, say: "⌘K" },
  { type: "Check the pairing copy", into: bar, say: "Words" },
  { key: "Tab", on: bar, say: "Tab steps Job, Autopilot, Task, Project" },
  { key: "Tab", on: bar, say: "Task" },
  { look: role("option", /^Add a task /), say: "Task: the row moves to the top, so Enter adds" },
  { key: "Enter", on: bar, say: "It asks which project, last used first" },
  { press: role("option", "Pocket launch"), say: "Filed in Pocket launch. The page stays on the Logbook" },
  { key: "Meta+k", on: places, say: "⌘K again, on All" },
  { type: "Release notes", into: bar, say: "Words" },
  { key: "Tab", on: bar, say: "Tab" },
  { key: "Tab", on: bar, say: "Tab" },
  { key: "Tab", on: bar, say: "To Project" },
  { key: "Enter", on: bar, say: "On All it asks which repository" },
  { press: role("option", "site"), say: "site" },
  { press: place("Projects"), say: "Projects" },
  { look: inside(list, button("Release notes")), say: "Release notes is under site" },
]);

export { projects as "projects" };
