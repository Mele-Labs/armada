// A Workspace is listed in the repository picker and edited in System > Workspaces, with the autonomy defaults of each level (armada-cwj). Opens on store-api, which is in my-store and in platform. A mock, no Fleet behind it.

import { button, inside, role, systemPage, walk } from "../walk";

const AUTONOMY = role("list", "Autonomy", { exact: true });
const LEVEL = (workspace: string, name: string) => inside(role("group", workspace, { exact: true }), button(name, { exact: true }));

export const workspacesHome = walk(
  "workspaces-and-autonomy",
  [
    { press: button("Repository"), say: "my-store, with store-web and store-api under it" },
    { press: button("Open my-store", { exact: true }), say: "The gear opens my-store in System" },
    { press: role("radio", "Ticket writes: Off", { exact: true }), say: "Ticket writes set to Off" },
    { press: button("Use Everywhere's: Asks", { exact: true }), say: "Set by reads my-store. Reset" },
    { press: LEVEL("my-store", "store-api"), say: "store-api, in my-store and platform" },
    { look: inside(AUTONOMY, role("img", "my-store and platform differ", { exact: true })), say: "Commands: my-store and platform differ" },
    { press: LEVEL("platform", "platform"), say: "platform" },
    { look: button("Remove", { exact: true }), say: "Remove is off: platform lives in store-docs" },
    { press: button("New Workspace", { exact: true }), say: "New Workspace" },
    { look: inside(role("form", "New Workspace", { exact: true }), role("textbox", "Name", { exact: true })), say: "An empty form" },
    { press: systemPage("Workflows"), say: "Workflows, on store-api" },
    { look: inside(role("group", "Runs in", { exact: true }), role("tab", "my-store", { exact: true })), say: "One tab for my-store" },
  ],
  { width: 1600, height: 1000 },
);

export { workspacesHome as "workspaces-home" };
