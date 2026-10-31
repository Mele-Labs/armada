// A project row's menu, mock only: the ContextMenu key opens Rename and Delete, Rename edits the name in the list, Delete takes the
// project away and the toast offers it back. Over the scenario `projects`.

import { button, inside, place, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

const list = region("Projects", { exact: true });
const menu = role("menu");

function steps(): Step[] {
  return [
    { press: place("Projects"), say: "Projects" },
    { key: "ContextMenu", on: inside(list, button("Pocket launch", { exact: true })), say: "A right press on a project, or the ContextMenu key" },
    { look: inside(menu, role("menuitem", "Rename")), say: "Rename, with its key" },
    { look: inside(menu, role("menuitem", "Delete")), say: "Delete, with its key" },
    { press: inside(menu, role("menuitem", "Rename")), say: "Rename" },
    { look: role("textbox", "Project name"), say: "The row as it is, with its words selected. Type, and Enter keeps it" },
    { key: "Escape", on: role("textbox", "Project name"), say: "Esc keeps the name" },
    { key: "ContextMenu", on: inside(list, button("Pocket launch", { exact: true })), say: "The menu again" },
    { press: inside(menu, role("menuitem", "Delete")), say: "Delete" },
    { look: role("alertdialog", "Delete Pocket launch?"), say: "A card beside the row asks first" },
    { press: inside(role("alertdialog", "Delete Pocket launch?"), button("Delete", { exact: true })), say: "Confirm" },
    { look: text("Deleted Pocket launch"), say: "The project is gone, with Undo for a few seconds" },
  ];
}

const one = walk("projects", steps());

export { one as "project-row-menu" };
