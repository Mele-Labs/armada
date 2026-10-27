// Projects, the fourth pass, mock only: a subtask dispatched on its own with its parent as context, the add key's hint inside the
// field, the search at the top of the left column, and the palette finding a task by its title. Over the scenario `projects`.

import { button, dialog, inside, place, region, role, walk } from "../walk";
import type { Step } from "../walk";

const places = role("navigation", "Places");
const list = region("Projects", { exact: true });
const pocket = region("Project Pocket launch");
const results = region("Search results");
const search = role("textbox", "Search the tasks");
const bar = inside(dialog("Command palette"), role("combobox"));

function steps(): Step[] {
  return [
    { press: place("Projects"), say: "Projects" },
    { press: inside(list, button("Pocket launch", { exact: true })), say: "Pocket launch" },
    { hover: button("Dispatch Scan it on a small screen"), say: "A subtask has Dispatch, with the same glyph and key as a task" },
    { press: button("Dispatch Scan it on a small screen"), say: "It opens the sheet with the subtask as the work" },
    { look: role("textbox", "Request"), say: "The subtask, then Part of and its parent, the parent's notes and its issue as a tag" },
    { key: "Escape", on: role("textbox", "Request"), say: "Esc puts it away. The Job holds the subtask only" },
    { look: inside(pocket, role("textbox", "Add a task")), say: "The key that goes to this field sits inside it, at its right edge" },
    { key: "/", on: list, say: "/ goes to the search at the top of the column" },
    { type: "pair", into: search, say: "Words" },
    { look: results, say: "Every task and subtask that matches, in all projects and repositories" },
    { look: inside(results, region("Pocket launch", { exact: true })), say: "Grouped by project, as a view is" },
    { look: inside(results, role("group", "Write the pairing page")), say: "A card from site among armada's" },
    { key: "ArrowDown", on: search, say: "↓ moves to the first result" },
    { key: "Escape", on: role("group", "Pair a phone by QR code"), say: "Esc clears the search" },
    { look: pocket, say: "Pocket launch, the project that was open, is back" },
    { press: place("Logbook"), say: "Another page" },
    { key: "Meta+k", on: places, say: "⌘K" },
    { type: "sitemap", into: bar, say: "A task's title" },
    { look: role("option", /Fix the sitemap dates/), say: "The task, with its project and repository under it" },
    { press: role("option", /Fix the sitemap dates/), say: "Choosing it" },
    { look: inside(region("View All tasks"), role("group", "Fix the sitemap dates")), say: "A task in no project opens All tasks, with its card in focus" },
    { key: "Meta+k", on: places, say: "⌘K" },
    { type: "tunnel address", into: bar, say: "A done subtask" },
    { press: role("option", /Draw the code from the tunnel address/), say: "Choosing it" },
    { look: inside(pocket, role("checkbox", "Done: Draw the code from the tunnel address")), say: "Its project opens with done shown, and its card in focus" },
  ];
}

const pass4 = walk("projects", steps());

export { pass4 as "projects-pass-4" };
