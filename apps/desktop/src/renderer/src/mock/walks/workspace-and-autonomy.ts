// Making a Workspace and setting autonomy defaults from System > Workflows (armada-cwj, mock takes). The same steps play every take: `&take=sheet`, `&take=panel` or `&take=popover` on the address
// draws the Workspace form as a centred sheet, in the side panel or under its button, and the autonomy rows as radio columns, one segmented control each, or one button each that moves to the next answer.
// The last steps pick store-docs, whose Workspace platform is committed in the repository. A mock, no Fleet behind it.

import { button, either, inside, role, text, toSystemPage, walk } from "../walk";

const RUNS_IN = role("group", "Runs in", { exact: true });
const tab = (name: string) => inside(RUNS_IN, role("tab", name, { exact: true }));
const NAME = role("textbox", "Name", { exact: true });
const AUTONOMY = role("list", "Autonomy", { exact: true });
const TICKETS = (answer: string) =>
  either(role("radio", `Ticket writes: ${answer}`, { exact: true }), inside(role("radiogroup", "Ticket writes", { exact: true }), role("radio", answer, { exact: true })), button(`Ticket writes: Asks`, { exact: true }));

export const workspaceAndAutonomy = walk(
  "workspaces-and-autonomy",
  [
    ...toSystemPage("Workflows", "Workflows, from System"),
    { press: button("Repository"), say: "The repository picker" },
    { press: role("menuitem", "scratch"), say: "scratch, which is in no Workspace" },
    { look: tab("scratch"), say: "Its tabs are Default and scratch" },
    { press: button("Workspace", { exact: true }), say: "Workspace" },
    { type: "sandbox", into: NAME, say: "Named sandbox" },
    { press: role("checkbox", "store-mobile", { exact: true }), say: "store-mobile ticked" },
    { press: button("Create", { exact: true }), say: "Create" },
    { look: tab("All of sandbox"), say: "A tab for sandbox, and one for each repository in it" },
    { press: button("Autonomy", { exact: true }), say: "Autonomy for All of sandbox" },
    { look: inside(AUTONOMY, text("Armada")), say: "Every row is set by Armada" },
    { press: TICKETS("Off"), say: "Ticket writes set to Off" },
    { look: inside(AUTONOMY, text("sandbox")), say: "The row is set by sandbox" },
    { look: button("Use Everywhere's: Asks", { exact: true }), say: "The reset glyph appears on that row only" },
    { press: button("Use Everywhere's: Asks", { exact: true }), say: "Reset" },
    { press: button("Edit sandbox", { exact: true }), say: "The pencil on sandbox's tab" },
    { look: button("Remove Workspace", { exact: true }), say: "Save and Remove Workspace" },
    { key: "Escape", on: NAME, say: "Escape closes the form" },
    { press: button("Repository"), say: "The repository picker" },
    { press: role("menuitem", "store-docs"), say: "store-docs, in platform" },
    { press: button("Edit platform", { exact: true }), say: "The pencil on platform's tab" },
    { look: button("Remove Workspace", { exact: true }), say: "Remove Workspace is disabled: it lives in the repository" },
  ],
  { width: 1600, height: 1000 },
);

export { workspaceAndAutonomy as "workspace-and-autonomy" };
