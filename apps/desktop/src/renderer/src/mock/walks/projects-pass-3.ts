// Projects, the third pass, mock only: a project with cards from two repositories, a repository view grouped by project with No
// project first, a task added on All tasks with the repository chip, the done toggle, two cards picked across repositories and
// dispatched one repository at a time, and the neutral cards. Over the scenario `projects`, which the walk of the same name also plays.

import { button, inside, place, region, role, walk } from "../walk";
import type { Step } from "../walk";

const list = region("Projects", { exact: true });
const all = region("View All tasks");
const site = region("Repository site");
const field = role("textbox", "Add a task");

function steps(): Step[] {
  return [
    { press: place("Projects"), say: "Projects" },
    { press: inside(list, button("Pocket launch", { exact: true })), say: "A project is his, so it can hold cards from two repositories" },
    { look: inside(role("group", "Pair a phone by QR code"), role("img", "Repository armada")), say: "A card from armada" },
    { look: inside(role("group", "Write the pairing page"), role("img", "Repository site")), say: "A card from site, in the same list. Each names its repository" },
    { press: inside(list, button("site", { exact: true })), say: "A repository is a view: all its tasks, grouped by project" },
    { look: inside(site, region("No project", { exact: true })), say: "No project first" },
    { look: inside(site, region("Docs refresh", { exact: true })), say: "Then each project that has cards here. One repository, so no chips on the cards" },
    { press: inside(list, button("All tasks", { exact: true })), say: "All tasks" },
    { type: "Order the hardware keys", into: field, say: "Adding works here too" },
    { look: inside(all, role("group", "Where")), say: "Two chips by the tag buttons: the repository, required, and the project, optional" },
    { press: button("Repository"), say: "The repository starts as the last one a task went to, else the first. A small menu changes it" },
    { press: role("menuitem", "site"), say: "site" },
    { key: "Enter", on: field, say: "Enter adds it" },
    { look: inside(inside(all, region("No project", { exact: true })), role("group", "Order the hardware keys")), say: "In no project, in site" },
    { look: button("Show done"), say: "Done tasks are hidden everywhere. One toggle in the head" },
    { press: button("Show done"), say: "Show done" },
    { look: role("checkbox", "Done: Rotate the signing key"), say: "Rotate the signing key, done" },
    { press: button("Hide done"), say: "Hide done" },
    { look: role("group", "Sign the build"), say: "A task Fleet asks about is not done and stays" },
    { key: " ", on: role("group", "Pair a phone by QR code"), say: "Space picks a card, in armada" },
    { key: " ", on: role("group", "Fix the sitemap dates"), say: "And one in site" },
    { look: inside(all, button("Dispatch", { exact: true })), say: "Dispatch appears once cards are picked, and not before" },
    { press: inside(all, button("Dispatch", { exact: true })), say: "One request for each repository. armada's first" },
    { key: "Escape", on: role("textbox", "Request"), say: "Starting or putting it away opens site's" },
    { look: role("textbox", "Request"), say: "site's request" },
    { key: "Escape", on: role("textbox", "Request"), say: "Esc" },
    { look: role("group", "Test on a small phone"), say: "Cards are neutral tiles. State is the glyph's colour and nothing else" },
    { hover: role("group", "Fix the sitemap dates"), say: "Hover and picked are the only other looks" },
  ];
}

const pass3 = walk("projects", steps());

export { pass3 as "projects-pass-3" };
