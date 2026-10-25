// A new project past the first run: New project in the Cockpit's repository picker, a name under the place offered, then the Cockpit with an empty Job on
// it, its message box focused. Over `new-project-from-picker`, whose folder dialog answers ~/Developer.

import { button, inside, role, walk } from "../walk";

const combobox = role("combobox", "Repository");
const dialog = role("dialog", "New project");

const newProjectFromPicker = walk("new-project-from-picker", [
  { press: combobox, say: "The repository picker" },
  { press: role("option", "New project"), say: "New project sits above Add a repository, and opens the dialog on a new project" },
  { look: inside(dialog, role("textbox", "Location")), say: "The location starts where most projects are kept, here ~/code" },
  { press: inside(dialog, button("Choose a folder")), say: "Choose a folder changes it, and ~/Developer is chosen" },
  { type: "my-app", into: inside(dialog, role("textbox", "Name")), say: "The name. The other two ways in stay under the fields" },
  { press: inside(dialog, button("Create project")), say: "Create project makes ~/Developer/my-app as a git repository" },
  { look: role("textbox", "Message"), say: "my-app is picked, and a new Job is open on it with its message box focused" },
]);

export { newProjectFromPicker as "new-project-from-picker" };
