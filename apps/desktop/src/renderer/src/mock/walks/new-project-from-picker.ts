// A new project past the first run: New project in the Cockpit's repository picker, a name under the place offered, then the Cockpit with an empty Job on
// it under its own heading, its message box focused, and the picker left on All repositories. Over `new-project-from-picker`, whose folder dialog answers ~/Developer.

import { button, inside, region, role, walk } from "../walk";

const combobox = role("combobox", "Repository");
const dialog = role("dialog", "New project");

const newProjectFromPicker = walk("new-project-from-picker", [
  { press: combobox, say: "The repository picker" },
  { press: role("option", "New project"), say: "New project sits above Add a repository, and opens the dialog on a new project" },
  { look: inside(dialog, role("textbox", "Location")), say: "The location starts where most projects are kept, here ~/code" },
  { press: inside(dialog, button("Choose a folder")), say: "Choose a folder changes it, and ~/Developer is chosen" },
  { type: "my-app", into: inside(dialog, role("textbox", "Name")), say: "The name. The other two ways in stay under the fields" },
  { press: inside(dialog, button("Create project")), say: "Create project makes ~/Developer/my-app as a git repository" },
  { look: inside(region("Jobs", { exact: true }), role("heading", "my-app", { exact: true })), say: "The picker stays on All repositories, and the new Job sits under a my-app heading" },
  { look: role("textbox", "Message"), say: "The Job is open, its message box focused" },
]);

export { newProjectFromPicker as "new-project-from-picker" };
