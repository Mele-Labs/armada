// Tasks filed from the palette on one ↵, the row's chips naming where each goes: the last used repository and No project, then a
// repository Tab chose, then one typed as @site with a project typed as +Docs. Then a task filed in the wrong repository is moved
// to the right one from its card, its subtasks with it, and its issue and pull request stay armada's. Over the scenario of the
// same name.

import { button, dialog, inside, place, region, role, text, walk } from "../walk";

const places = role("navigation", "Places");
const bar = inside(dialog("Command palette"), role("combobox"));
const row = role("option", /^Add a task /);
const all = region("View All tasks");
const card = (title: string) => inside(all, role("group", title, { exact: true }));
const misfiled = card("Move the docs search box to the header");

const projectsMoveRepository = walk("projects-move-repository", [
  { press: place("Projects"), say: "Projects" },
  { key: "Meta+k", on: places, say: "⌘K" },
  { type: "Link the search box from the home page", into: bar, say: "Words" },
  { key: "Tab", on: bar, say: "Tab steps Job, Autopilot, Task, Project" },
  { key: "Tab", on: bar, say: "Task" },
  { look: inside(row, button("Repository armada", { exact: true })), say: "The row names its repository before ↵: armada, the last used" },
  { look: inside(row, button("Project No project", { exact: true })), say: "And its project: No project" },
  { key: "Enter", on: bar, say: "↵ files it at once" },
  { look: inside(card("Link the search box from the home page"), button("Repository armada", { exact: true })), say: "On Projects, in armada, in no project" },
  { key: "Meta+k", on: places, say: "⌘K" },
  { type: "Add the search box to the 404 page", into: bar, say: "Another task" },
  { key: "Tab", on: bar, say: "Tab" },
  { key: "Tab", on: bar, say: "Task" },
  { key: "Tab", on: bar, say: "Tab opens the repository chip's list" },
  { look: inside(role("option", /^armada/), text("last used")), say: "armada first, last used" },
  { key: "ArrowDown", on: bar, say: "Down to site" },
  { key: "Enter", on: bar, say: "↵ chooses it, and the words have the keys again" },
  { look: inside(row, button("Repository site", { exact: true })), say: "The chip reads site" },
  { key: "Enter", on: bar, say: "↵ files it" },
  { look: inside(card("Add the search box to the 404 page"), button("Repository site", { exact: true })), say: "On Projects, in site" },
  { key: "Meta+k", on: places, say: "⌘K" },
  { key: "Tab", on: bar, say: "Tab" },
  { key: "Tab", on: bar, say: "Task" },
  { type: "Fix the search results order @si", into: bar, say: "@ and a repository's first letters" },
  { look: role("option", /^site/), say: "The repositories that match" },
  { key: "Tab", on: bar, say: "Tab takes site" },
  { look: inside(row, button("Repository site", { exact: true })), say: "The repository chip reads site" },
  { type: "Fix the search results order @site +Doc", into: bar, say: "+ and a project's first letters" },
  { look: role("option", /^Docs refresh/), say: "The projects that match" },
  { key: "Enter", on: bar, say: "↵ takes Docs refresh" },
  { look: role("option", /^Add a task “Fix the search results order”/), say: "The tokens leave the title" },
  { look: inside(row, button("Repository site", { exact: true })), say: "The repository chip reads site" },
  { look: inside(row, button("Project Docs refresh", { exact: true })), say: "The project chip reads Docs refresh" },
  { key: "Enter", on: bar, say: "↵ files it" },
  { look: inside(inside(all, region("Docs refresh", { exact: true })), role("group", "Fix the search results order", { exact: true })), say: "On Projects, under Docs refresh" },
  { press: inside(misfiled, button("Repository armada", { exact: true })), say: "A task filed in armada that belongs in site. Its repository is a menu" },
  { look: text("Move task and its 2 subtasks to"), say: "Its subtasks go with it" },
  { press: role("menuitem", "site", { exact: true }), say: "site" },
  { look: inside(misfiled, button("Repository site", { exact: true })), say: "The task is in site now" },
  { look: inside(misfiled, role("img", "Linked in armada", { exact: true })), say: "Its issue and pull request were linked in armada, so they are muted and name armada" },
]);

export { projectsMoveRepository as "projects-move-repository" };
