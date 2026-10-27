// Projects, mock only. A left column of three views, his projects and the repositories; the cards of a view or a repository grouped
// by project with the tasks in no project first; a project that holds cards from two repositories, in his order; the add field
// with its repository and project; done tasks hidden behind a toggle; cards picked across repositories and dispatched, one request
// for each; the done ask on the card and on the Job; then the palette adds a task and a project from another page. Over the
// scenario of the same name.

import { button, dialog, inside, place, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

const places = role("navigation", "Places");
const list = region("Projects", { exact: true });
const all = region("View All tasks");
const pocket = region("Project Pocket launch");
const site = region("Repository site");
const field = role("textbox", "Add a task");
const bar = inside(dialog("Command palette"), role("combobox"));
const needs = region("Needs you");
const picked = (pane: ReturnType<typeof region>) => inside(pane, button("Dispatch", { exact: true }));

function steps(): Step[] {
  return [
    { press: place("Projects"), say: "Projects: three views, his projects and the repositories" },
    { look: inside(list, button("All tasks", { exact: true })), say: "All tasks, Needs you and In a Job" },
    { look: inside(list, button("Pocket launch", { exact: true })), say: "A project is a name with a folder. It spans repositories" },
    { look: inside(list, button("armada", { exact: true })), say: "A repository is a name with a branch. It is a view of its tasks, not a list added to" },
    { look: inside(all, region("No project", { exact: true })), say: "All tasks: the tasks in no project come first" },
    { look: inside(all, region("Pocket launch", { exact: true })), say: "Then each project under a small head" },
    { look: inside(role("group", "Fix the sitemap dates"), role("img", "Repository site")), say: "A card names its repository, since the list holds more than one" },
    { key: "l", on: list, say: "Keys: l r column and cards, j k move, 1 2 3 views, Space select, Enter edit, a A task and subtask, h done, o open the Job, u U unlink, d tick, D K answer, Backspace delete, e rename, s dispatch, ⌥↑ ⌥↓ move, Esc back" },
    { key: "2", on: list, say: "2 shows Needs you" },
    { look: role("group", "Sign the build"), say: "The tasks Fleet asks about as done" },
    { key: "3", on: list, say: "3 shows the tasks a Job holds" },
    { look: role("group", "Pair a phone by QR code"), say: "In a Job" },
    { key: "1", on: list, say: "1 is back on All tasks" },
    { press: inside(list, button("site", { exact: true })), say: "A repository: all its tasks, grouped by project" },
    { look: inside(site, region("No project", { exact: true })), say: "No project first, then Docs refresh and Pocket launch" },
    { press: inside(list, button("Pocket launch", { exact: true })), say: "A project: its cards from both repositories, in his order" },
    { look: inside(role("group", "Write the pairing page"), role("img", "Repository site")), say: "A card from site among armada's" },
    { look: role("list", "Subtasks of Pair a phone by QR code"), say: "Two subtasks under a task, each with its own glyph and tick" },
    { look: role("img", /^#2815 /), say: "An issue linked to the task, as a quiet chip" },
    { look: button("Open Wire the pairing screen"), say: "The Job holding it, on its own line under the title. A press opens it" },
    { look: button("Show done"), say: "Done tasks are hidden. This shows them" },
    { press: button("Show done"), say: "Show done" },
    { look: role("checkbox", "Done: Pick the app icon"), say: "Pick the app icon is done" },
    { press: button("Hide done"), say: "Hide done" },
    { press: field, say: "The field under the head: one line at rest, a composer on focus" },
    { type: "Review the store screenshots", into: field, say: "Words" },
    { look: inside(pocket, role("group", "Where")), say: "Two chips beside the tag buttons: the repository, and the project, which is fixed here" },
    { press: button("Repository"), say: "The repository is required. It starts as the last one a task went to, else the first" },
    { press: role("menuitem", "site"), say: "site" },
    { key: "Enter", on: field, say: "Enter adds it to Pocket launch, in site" },
    { look: inside(role("group", "Review the store screenshots"), role("img", "Repository site")), say: "Added at the top, with its repository" },
    { press: button("Repository"), say: "The next task starts in the repository the last one went to. This one is armada's" },
    { press: role("menuitem", "armada"), say: "armada" },
    { type: "Fix the pairing timeout #2815 @", into: field, say: "Tags, as in the request box: # an issue or pull request, @ a Job. They read the repository chosen" },
    { press: role("option", "Wire the pairing screen"), say: "Pick a Job. ↑ ↓ and Enter pick too" },
    { key: "Enter", on: field, say: "Enter adds it" },
    { look: role("group", "Fix the pairing timeout"), say: "The tags leave the title and join the card: #2815 as a chip, the Job holding it" },
    { press: button("Unlink #2874"), say: "A chip's own x unlinks a pull request" },
    { key: "Enter", on: role("group", "Test on a small phone"), say: "Keys only. Enter on a card edits its title in place" },
    { key: "Escape", on: role("textbox", "Title of Test on a small phone"), say: "Esc leaves it as it was" },
    { press: button("Add a subtask to Test on a small phone"), say: "A subtask: the card's own field, inside it" },
    { type: "Check the narrow layout\n", into: role("textbox", "Add a subtask to Test on a small phone"), say: "Enter adds it, and the field stays for the next" },
    { key: "Escape", on: role("textbox", "Add a subtask to Test on a small phone"), say: "Esc, or Enter on it empty, puts it away" },
    { press: button("Delete Fix the pairing timeout"), say: "Delete asks nothing" },
    { press: button("Undo", { exact: true }), say: "Undo, in the toast, puts it back" },
    { drag: text("Test on a small phone"), by: { x: 0, y: -220 }, say: "Drag a card to move it, as ⌥↑ and ⌥↓ do. Only in a project" },
    { press: inside(list, button("All tasks", { exact: true })), say: "All tasks" },
    { look: field, say: "The field is here too, and so is the repository chip" },
    { type: "Order the hardware keys", into: field, say: "Words" },
    { look: inside(all, role("group", "Where")), say: "The project chip starts at No project" },
    { press: button("Repository"), say: "The repository, from a small menu" },
    { press: role("menuitem", "armada"), say: "armada" },
    { key: "Enter", on: field, say: "Enter adds it" },
    { look: inside(inside(all, region("No project", { exact: true })), role("group", "Order the hardware keys")), say: "A task in no project" },
    { hover: button("Dispatch Fix the sitemap dates"), say: "A card's Dispatch" },
    { press: button("Dispatch Fix the sitemap dates"), say: "It opens the sheet with that task" },
    { key: "Escape", on: role("textbox", "Request"), say: "Esc puts the sheet away" },
    { look: all, say: "Nothing is picked, so the head holds no Dispatch" },
    { key: " ", on: role("group", "Pair a phone by QR code"), say: "Space picks a card. ⌘-click and ⇧-click pick too" },
    { key: " ", on: role("group", "Fix the sitemap dates"), say: "And one from the other repository" },
    { look: picked(all), say: "Dispatch appears once cards are picked" },
    { press: picked(all), say: "One request for each repository. armada's first" },
    { look: role("textbox", "Request"), say: "Pair a phone by QR code and its subtasks, on armada" },
    { key: "Escape", on: role("textbox", "Request"), say: "Esc puts it away, and the next opens" },
    { look: role("textbox", "Request"), say: "Fix the sitemap dates, on site" },
    { key: "Escape", on: role("textbox", "Request"), say: "Esc" },
    { hover: role("checkbox", "Done?: Write the install guide"), say: "This one's Job has ended, and the card still asks whether it is done" },
    { press: button("Open Sign the release build"), say: "Sign the build asks too. Its Job finished and is live" },
    { press: inside(needs, button(/^Needs you/)), say: "Needs you on the Job asks the same" },
    { look: inside(needs, text("Mark Sign the build done?")), say: "Mark Sign the build done?" },
    { press: inside(needs, role("radio", "Mark done", { exact: true })), say: "Mark done" },
    { press: inside(needs, button("Answer")), say: "Answered here, so the card is answered too" },
    { press: place("Projects"), say: "Back on Projects" },
    { press: button("Show done"), say: "Done tasks are hidden, so Show done" },
    { look: role("checkbox", "Done: Sign the build"), say: "Sign the build is done" },
    { press: place("Logbook"), say: "Another page" },
    { key: "Meta+k", on: places, say: "⌘K" },
    { type: "Check the pairing copy", into: bar, say: "Words" },
    { key: "Tab", on: bar, say: "Tab steps Job, Autopilot, Task, Project" },
    { key: "Tab", on: bar, say: "Task" },
    { look: role("option", /^Add a task /), say: "Task: the row moves to the top, so Enter adds" },
    { key: "Enter", on: bar, say: "It asks which project, last used first, or none" },
    { press: role("option", "Pocket launch"), say: "Filed in Pocket launch. The page stays on the Logbook" },
    { key: "Meta+k", on: places, say: "⌘K again" },
    { type: "Release notes", into: bar, say: "Words" },
    { key: "Tab", on: bar, say: "Tab" },
    { key: "Tab", on: bar, say: "Tab" },
    { key: "Tab", on: bar, say: "To Project" },
    { key: "Enter", on: bar, say: "A project is his: it asks for no repository" },
    { press: place("Projects"), say: "Projects" },
    { look: inside(list, button("Release notes", { exact: true })), say: "Release notes is in the list" },
  ];
}

const projects = walk("projects", steps());

export { projects as "projects" };
