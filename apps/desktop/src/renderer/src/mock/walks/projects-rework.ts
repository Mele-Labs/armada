// Projects, the rework, mock only: a + on each repository's head, a press on a head opening that repository's own tasks, a
// card's Dispatch, cards picked, the add field growing into a composer with its four buttons, and the Jobs list's heads in
// the Cockpit. Over the scenario `projects`, which the walk of the same name also plays.

import { button, inside, place, region, role, walk } from "../walk";
import type { Step } from "../walk";

const list = region("Projects", { exact: true });
const site = region("Tasks of site");
const armada = region("Tasks of armada");
const field = role("textbox", "Add a task");
const name = role("textbox", "Project name");

function steps(): Step[] {
  return [
    { press: place("Projects"), say: "Projects, a panel for each repository" },
    { hover: inside(list, button("New project in site")), say: "A + at the right of each repository's head" },
    { press: inside(list, button("New project in site")), say: "It asks the name in that repository" },
    { key: "Escape", on: name, say: "Esc puts the field away" },
    { key: "N", on: list, say: "⇧N asks in the repository you are in, else the first" },
    { key: "Escape", on: name, say: "Esc" },
    { press: inside(list, button("site", { exact: true })), say: "A press on the head opens site's own tasks" },
    { look: site, say: "The same field, cards and keys as a project. The repository is the title" },
    { hover: button("Dispatch Fix the sitemap dates"), say: "Dispatch on each task's card, with its key" },
    { press: button("Dispatch Fix the sitemap dates"), say: "It opens the sheet with that task" },
    { key: "Escape", on: role("textbox", "Request"), say: "Esc puts the sheet away" },
    { look: role("group", "Fix the sitemap dates"), say: "No select box. ⌘-click picks a card, ⇧-click picks the range from the last card clicked" },
    { key: " ", on: role("group", "Fix the sitemap dates"), say: "Space picks the card under focus" },
    { key: " ", on: role("group", "Check the 404 page"), say: "And this one" },
    { look: inside(site, button("Dispatch", { exact: true })), say: "The head's Dispatch sends the cards picked" },
    { press: inside(list, button("armada", { exact: true })), say: "armada keeps no tasks of its own yet" },
    { look: armada, say: "Its first task makes the list" },
    { press: field, say: "One line at rest. Focus opens the composer" },
    { type: "Fix the pairing timeout ", into: field, say: "Words" },
    { look: role("group", "Tag"), say: "Four buttons under the words: Job, file, issue, pull request" },
    { press: button("Job", { exact: true }), say: "Job puts @ at the caret and opens the Jobs" },
    { press: role("option", "Wire the pairing screen"), say: "Pick one" },
    { press: button("Issue", { exact: true }), say: "Issue puts # at the caret" },
    { press: role("option", /^#2815/), say: "Pick one" },
    { press: button("Pull request", { exact: true }), say: "Pull request puts # at the caret" },
    { press: role("option", /^#2874/), say: "Pick one" },
    { hover: button("File", { exact: true }), say: "File puts @ at the caret. A path typed after it is searched" },
    { key: "Enter", on: field, say: "Enter adds. ⇧Enter is a new line" },
    { look: role("group", "Fix the pairing timeout"), say: "The tags leave the title and join the card" },
    { press: place("Cockpit"), say: "The Cockpit" },
    { look: inside(region("Jobs", { exact: true }), role("heading", "armada")), say: "The Jobs list's repository head is the Projects head" },
  ];
}

const projectsRework = walk("projects", steps());

export { projectsRework as "projects-rework" };
