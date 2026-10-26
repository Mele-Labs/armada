// Projects, mock only, in two takes of the select control and the card: `projects` (reticle) and `projects-plate`. Open
// the tab on All and Pocket launch; rows as cards with the Job holding each as a chip and a task's issues as chips; a task
// and a subtask added from the field under the head; an issue and a subtask added from a card; a delete undone; a card
// dragged; three rows dispatched; the done ask on the row and on the Job; then the palette adds a task and a project from
// another page. Over the scenario of the same name.

import { button, dialog, inside, place, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

const places = role("navigation", "Places");
const list = region("Projects", { exact: true });
const pocket = region("Project Pocket launch");
const bar = inside(dialog("Command palette"), role("combobox"));
const needs = region("Needs you");

function steps(take: string): Step[] {
  return [
    { press: place("Projects"), say: "Projects, a panel for each repository: armada's and site's on All" },
    { key: "l", on: list, say: "Keys: l the list, r the cards, a add a task, A a subtask, i link issues, I to the task, x select, d tick, Backspace delete, s dispatch, ⌥↑ ⌥↓ move" },
    { press: inside(list, button("Pocket launch")), say: "Pocket launch: every row a card" },
    { look: role("checkbox", "Select Pair a phone by QR code"), say: `${take}: the select control` },
    { look: role("list", "Subtasks of Pair a phone by QR code"), say: "Two subtasks under it, each with its own glyph and tick" },
    { look: role("img", /^#2815 /), say: "An issue linked to the task, as a chip" },
    { look: button("Open Wire the pairing screen"), say: "The Job holding it, on its own line under the title. A press opens it" },
    { type: "Draft the App Store text\n", into: role("textbox", "Add a task"), say: "The field under the head. Enter adds a task at the top" },
    { key: "Tab", on: role("textbox", "Add a task"), say: "Tab indents it under the task just written" },
    { type: "Write the short description\n", into: role("textbox", "Add a subtask to Draft the App Store text"), say: "Enter adds a subtask" },
    { key: "Shift+Tab", on: role("textbox", "Add a subtask to Draft the App Store text"), say: "⇧Tab takes it back out" },
    { hover: text("Test on a small phone"), say: "Under the pointer a card shows its acts at its right edge: link, subtask, delete" },
    { press: button("Link issues to Test on a small phone"), say: "Link issues to this task" },
    { press: button(/^#2819 /), say: "#2819 joins the card as a chip" },
    { key: "Escape", on: role("textbox", "Search issues"), say: "Esc puts the search away" },
    { press: button("Add a subtask to Test on a small phone"), say: "A subtask, from the card" },
    { type: "Check the narrow layout\n", into: role("textbox", "Add a subtask to Test on a small phone"), say: "Enter adds it" },
    { key: "Escape", on: role("textbox", "Add a subtask to Test on a small phone"), say: "Esc puts the field away" },
    { press: button("Delete Pick the app icon"), say: "Delete asks nothing" },
    { press: button("Undo", { exact: true }), say: "Undo, in the toast, puts it back" },
    { drag: text("Test on a small phone"), by: { x: 0, y: -220 }, say: "Drag a card to move it, as ⌥↑ and ⌥↓ do" },
    { press: role("checkbox", "Select Pair a phone by QR code"), say: "Select a task: its subtasks go with it" },
    { press: role("checkbox", /^Select #2801 /), say: "And an issue" },
    { press: inside(pocket, button("Dispatch", { exact: true })), say: "Dispatch: the task with its subtasks under it, its issue and #2801 as tags. Starting is the sheet's" },
    { key: "Escape", on: role("textbox", "Request"), say: "Esc puts the sheet away" },
    { hover: role("checkbox", "Done?: Write the install guide"), say: "This one's Job has ended, and the card still asks whether it is done" },
    { press: button("Open Sign the release build"), say: "Sign the build asks too. Its Job finished and is live" },
    { press: inside(needs, button(/^Needs you/)), say: "Needs you on the Job asks the same" },
    { look: inside(needs, text("Mark Sign the build done?")), say: "Mark Sign the build done?" },
    { press: inside(needs, role("radio", "Mark done", { exact: true })), say: "Mark done" },
    { press: inside(needs, button("Answer")), say: "Answered here, so the card is answered too" },
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
  ];
}

const projects = walk("projects", steps("Reticle"));
const projectsPlate = walk("projects-plate", steps("Plate"));

export { projects as "projects", projectsPlate as "projects-plate" };
