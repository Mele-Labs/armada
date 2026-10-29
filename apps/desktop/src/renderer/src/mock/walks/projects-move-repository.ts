// A task filed from the palette asks for its repository after its project, the last used first; then a task filed in the wrong
// repository is moved to the right one from its card, its subtasks with it, and its issue and pull request stay armada's. Over
// the scenario of the same name.

import { button, dialog, inside, place, role, text, walk } from "../walk";

const places = role("navigation", "Places");
const bar = inside(dialog("Command palette"), role("combobox"));
const added = role("group", "Link the search box from the home page", { exact: true });
const misfiled = role("group", "Move the docs search box to the header", { exact: true });

const projectsMoveRepository = walk("projects-move-repository", [
  { press: place("Projects"), say: "Projects" },
  { key: "Meta+k", on: places, say: "⌘K" },
  { type: "Link the search box from the home page", into: bar, say: "Words" },
  { key: "Tab", on: bar, say: "Tab steps Job, Autopilot, Task, Project" },
  { key: "Tab", on: bar, say: "Task" },
  { key: "Enter", on: bar, say: "It asks which project" },
  { press: role("option", "Docs refresh"), say: "Docs refresh. Then it asks which repository" },
  { look: text("File “Link the search box from the home page” in"), say: "File it in a repository" },
  { look: inside(role("option", /^armada/), text("last used")), say: "armada first, last used" },
  { press: role("option", "site", { exact: true }), say: "site" },
  { look: inside(added, button("Repository site", { exact: true })), say: "The task is on Projects, in site" },
  { press: inside(misfiled, button("Repository armada", { exact: true })), say: "A task filed in armada that belongs in site. Its repository is a menu" },
  { look: text("Move task and its 2 subtasks to"), say: "Its subtasks go with it" },
  { press: role("menuitem", "site", { exact: true }), say: "site" },
  { look: inside(misfiled, button("Repository site", { exact: true })), say: "The task is in site now" },
  { look: inside(misfiled, role("img", "Linked in armada", { exact: true })), say: "Its issue and pull request were linked in armada, so they are muted and name armada" },
]);

export { projectsMoveRepository as "projects-move-repository" };
