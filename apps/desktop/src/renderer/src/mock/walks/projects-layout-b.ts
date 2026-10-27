// Projects, take B of the layout: a narrow rail of the three views and a glyph for each repository, beside one grouped list with
// a toolbar. Each group ends in its own Add a task. Over the scenario `projects`; take A is the walk `projects-layout-a`.

import { button, inside, place, region, role, walk } from "../walk";
import type { Step } from "../walk";

const list = region("Projects", { exact: true });
const pane = region("Tasks grouped");
const toolbar = inside(pane, role("toolbar", "List"));
const rail = (name: string) => inside(list, button(name, { exact: true }));
const group = (name: string) => inside(pane, region(name));
const field = (name: string) => inside(group(name), role("textbox", "Add a task"));

function steps(): Step[] {
  return [
    { press: place("Projects"), say: "Projects: a rail of views and repositories, one grouped list" },
    { hover: rail("Needs you"), say: "Each glyph names itself and its key" },
    { look: rail("armada"), say: "One glyph for each repository" },
    { look: group("armada · Pocket launch"), say: "A head for each project, naming its repository" },
    { look: group("armada · Tasks"), say: "A repository's own tasks are a group of their own" },
    { look: field("armada · Pocket launch"), say: "Each group ends in its own Add a task" },
    { type: "Fix the pairing timeout\n", into: field("armada · Pocket launch"), say: "It joins that project" },
    { look: inside(group("armada · Pocket launch"), role("group", "Fix the pairing timeout")), say: "Under the head it was written in" },
    { type: "Check the QR scan\n", into: field("armada · Tasks"), say: "And here, in armada's own tasks" },
    { hover: inside(group("armada · Pocket launch"), button("Rename Pocket launch")), say: "A project is renamed and deleted from its head" },
    { press: inside(toolbar, role("tab", "Repository", { exact: true })), say: "Group by repository" },
    { look: group("armada"), say: "A head for each repository, and each card names its project" },
    { press: inside(toolbar, role("tab", "State", { exact: true })), say: "Group by state" },
    { look: group("Needs you"), say: "Needs you, In a Job, Open and Done" },
    { key: "G", on: list, say: "G cycles the grouping" },
    { look: group("armada · Pocket launch"), say: "Back on project" },
    { key: "2", on: list, say: "2 narrows to Needs you" },
    { look: role("group", "Sign the build"), say: "The tasks Fleet asks about as done" },
    { key: "3", on: list, say: "3 narrows to In a Job" },
    { look: role("group", "Pair a phone by QR code"), say: "The tasks a Job holds" },
    { key: "1", on: list, say: "1 is back on All" },
    { press: rail("site"), say: "A repository glyph narrows the list to it" },
    { look: group("Docs refresh"), say: "Only site's groups" },
    { press: rail("site"), say: "Again for All" },
    { press: inside(toolbar, button("New project")), say: "On All, New project asks which repository" },
    { press: role("menuitem", "site"), say: "site" },
    { type: "Billing\n", into: role("textbox", "Project name"), say: "Enter adds it to site" },
    { look: group("site · Billing"), say: "Its group, with its own Add a task" },
    { hover: button("Dispatch Fix the sitemap dates"), say: "A card's Dispatch" },
    { press: button("Dispatch Fix the sitemap dates"), say: "It opens the sheet with that task" },
    { key: "Escape", on: role("textbox", "Request"), say: "Esc puts the sheet away" },
    { key: " ", on: role("group", "Fix the sitemap dates"), say: "Space picks a card. ⌘-click and ⇧-click pick too" },
    { key: " ", on: role("group", "Write the pairing page"), say: "And one from another project of site" },
    { press: inside(pane, button("Dispatch", { exact: true })), say: "The toolbar's Dispatch sends the cards picked to one Job" },
    { look: role("textbox", "Request"), say: "Both tasks, on site" },
    { key: "Escape", on: role("textbox", "Request"), say: "Esc puts the sheet away" },
  ];
}

const layoutB = walk("projects", steps());

export { layoutB as "projects-layout-b" };
