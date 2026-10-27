// Projects, mock only. Open
// the tab on All and Pocket launch; rows as cards with the Job holding each as a chip and a task's issues as chips; a task
// and a subtask added from the field under the head, one tagged with an issue and a Job; a Job tagged onto a title being edited,
// a subtask added from a card, a pull request unlinked; a delete undone; a card
// dragged; three rows picked and dispatched; the done ask on the row and on the Job; then the palette adds a task and a project from
// another page. Over the scenario of the same name.

import { button, dialog, inside, place, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

const places = role("navigation", "Places");
const list = region("Projects", { exact: true });
const pocket = region("Project Pocket launch");
const bar = inside(dialog("Command palette"), role("combobox"));
const needs = region("Needs you");

function steps(): Step[] {
  return [
    { press: place("Projects"), say: "Projects, a panel for each repository: armada's and site's on All" },
    { key: "l", on: list, say: "Keys: l r list and cards, j k move, Space select, Enter edit, a A task and subtask, o open the Job, u U unlink, d tick, D K answer, Backspace delete, e rename, s dispatch, ⌥↑ ⌥↓ move, Esc back" },
    { press: inside(list, button("Pocket launch")), say: "Pocket launch: every row a card" },
    { look: role("list", "Subtasks of Pair a phone by QR code"), say: "Two subtasks under it, each with its own glyph and tick" },
    { look: role("img", /^#2815 /), say: "An issue linked to the task, as a chip" },
    { look: button("Open Wire the pairing screen"), say: "The Job holding it, on its own line under the title. A press opens it" },
    { type: "Draft the App Store text\n", into: role("textbox", "Add a task"), say: "The field under the head. Enter adds a task at the top" },
    { type: "Fix the pairing timeout #2815 @", into: role("textbox", "Add a task"), say: "Tags, as in the request box: # an issue or pull request, @ a Job" },
    { press: role("option", "Wire the pairing screen"), say: "Pick a Job. ↑ ↓ and Enter pick too" },
    { key: "Enter", on: role("textbox", "Add a task"), say: "Enter adds it" },
    { look: role("group", "Fix the pairing timeout"), say: "The tags leave the title and join the card: #2815 as a chip, the Job holding it" },
    { look: button("Dispatch Test on a small phone"), say: "Every card's acts, always at its right edge: dispatch, add a subtask, delete" },
    { look: button(/^Pull request #2874 /), say: "A pull request linked to the task, on its second line. Its glyph says open" },
    { press: button("Unlink #2874"), say: "A chip's own x unlinks it" },
    { key: "Enter", on: role("group", "Draft the App Store text"), say: "Keys only. Enter on a card edits its title in place" },
    { type: " @", into: role("textbox", "Title of Draft the App Store text"), say: "Tags work in a title too: @ a Job already at work on it" },
    { press: role("option", "Write the App Store listing"), say: "Pick one" },
    { key: "Enter", on: role("textbox", "Title of Draft the App Store text"), say: "Enter keeps it. The Job holds the task now, as Dispatch would: its chip on the second line, and the done ask when it finishes" },
    { key: "u", on: role("group", "Draft the App Store text"), say: "u unlinks its Job" },
    { key: " ", on: role("group", "Draft the App Store text"), say: "Space picks it" },
    { press: button("Add a subtask to Test on a small phone"), say: "A subtask: the card's own field, inside it" },
    { type: "Check the narrow layout\n", into: role("textbox", "Add a subtask to Test on a small phone"), say: "Enter adds it, and the field stays for the next" },
    { key: "Escape", on: role("textbox", "Add a subtask to Test on a small phone"), say: "Esc, or Enter on it empty, puts it away" },
    { press: button("Delete Pick the app icon"), say: "Delete asks nothing" },
    { press: button("Undo", { exact: true }), say: "Undo, in the toast, puts it back" },
    { drag: text("Test on a small phone"), by: { x: 0, y: -220 }, say: "Drag a card to move it, as ⌥↑ and ⌥↓ do" },
    { key: " ", on: role("group", "Check the narrow layout"), say: "A subtask is picked on its own, a size down" },
    { key: " ", on: role("group", "Pair a phone by QR code"), say: "Pick a task: its subtasks go with it" },
    { key: " ", on: role("group", /^#2801 /), say: "And an issue" },
    { press: inside(pocket, button("Dispatch", { exact: true })), say: "Dispatch: the tasks, each with its subtasks under it, the subtask on its own, the issues as tags. Starting is the sheet's" },
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

const projects = walk("projects", steps());

export { projects as "projects" };
