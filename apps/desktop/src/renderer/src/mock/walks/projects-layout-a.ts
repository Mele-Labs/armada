// Projects, take A of the layout: three views on top of the left column, a hairline, then a quiet row for each repository that
// folds. A view lists tasks from every repository under a head for each project. Over the scenario `projects`; take B is the walk
// `projects-layout-b`.

import { button, inside, place, region, role, walk } from "../walk";
import type { Step } from "../walk";

const list = region("Projects", { exact: true });
const view = (name: string) => inside(list, button(name, { exact: true }));
const all = region("View All tasks");

function steps(): Step[] {
  return [
    { press: place("Projects"), say: "Projects: three views on top, a row for each repository under them" },
    { look: view("All tasks"), say: "All tasks, Needs you and In a Job" },
    { look: all, say: "The tasks of every repository, under a head for each project" },
    { look: region("site · Docs refresh"), say: "A head names the repository and the project" },
    { hover: view("Needs you"), say: "Each view names itself and its key" },
    { key: "2", on: list, say: "2 shows Needs you" },
    { look: role("group", "Sign the build"), say: "The tasks Fleet asks about as done" },
    { press: view("In a Job"), say: "In a Job" },
    { look: role("group", "Pair a phone by QR code"), say: "The tasks a Job holds" },
    { key: "1", on: list, say: "1 is back on All tasks" },
    { hover: view("armada"), say: "A repository is one quiet row with a chevron" },
    { press: view("armada"), say: "It folds" },
    { press: view("armada"), say: "And opens" },
    { look: view("Tasks of site"), say: "Tasks is the repository's own tasks" },
    { press: view("Pocket launch"), say: "A project is a name only" },
    { look: region("Project Pocket launch"), say: "It shows as it did" },
    { hover: view("New project in site"), say: "The + is always drawn" },
    { press: view("New project in site"), say: "It asks the name in that repository" },
    { type: "Billing\n", into: role("textbox", "Project name"), say: "Enter adds it and opens it" },
    { look: region("Project Billing"), say: "Billing, in site" },
    { press: view("All tasks"), say: "All tasks again" },
    { hover: button("Dispatch Fix the sitemap dates"), say: "A card's Dispatch" },
    { press: button("Dispatch Fix the sitemap dates"), say: "It opens the sheet with that task" },
    { key: "Escape", on: role("textbox", "Request"), say: "Esc puts the sheet away" },
    { key: " ", on: role("group", "Fix the sitemap dates"), say: "Space picks a card. ⌘-click and ⇧-click pick too" },
    { key: " ", on: role("group", "Write the pairing page"), say: "And one from another project of site" },
    { press: inside(all, button("Dispatch", { exact: true })), say: "The head's Dispatch sends the cards picked to one Job" },
    { look: role("textbox", "Request"), say: "Both tasks, on site" },
    { key: "Escape", on: role("textbox", "Request"), say: "Esc puts the sheet away" },
  ];
}

const layoutA = walk("projects", steps());

export { layoutA as "projects-layout-a" };
