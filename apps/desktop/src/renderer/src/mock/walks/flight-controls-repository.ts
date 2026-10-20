// An action kept only in one repository: in a lendinghome-monolith Job, Edit adds a Shell action "Run specs" marked Only in
// lendinghome-monolith; Done keeps it in that repository's file, drawn after the owner's own tiles and before Pause; a Job in armada does
// not draw it. The owner's decision of 19 Oct 2026.

import { button, dialog, inside, region, role, walk } from "../walk";

const list = region("Jobs", { exact: true });
const controls = region("Flight controls");
const tile = (name: string) => inside(controls, button(name));
const editing = inside(controls, role("listbox", "Flight controls, editing"));
const option = (name: string) => inside(editing, role("option", name));
const field = (kind: string, name: string) => inside(dialog("Add action"), role(kind, name));

const flightControlsRepository = walk("flight-controls-repository", [
  { press: inside(list, button("Upgrade the loan pricing gem")), say: "A Job in lendinghome-monolith" },
  { press: tile("Edit"), say: "Edit" },
  { press: option("Add action"), say: "The + tile adds an action" },
  { type: "shell", into: field("combobox", "Kind"), say: "A Shell command" },
  { type: "Run specs", into: field("textbox", "Name"), say: "Its name" },
  { type: "bin/rspec", into: field("textbox", "Shell line"), say: "It runs in the Job's bay" },
  { press: field("checkbox", "Only in lendinghome-monolith"), say: "Only in lendinghome-monolith keeps it with that repository, for you alone" },
  { press: inside(dialog("Add action"), button("Save")), say: "Saved" },
  { look: option("Run specs"), say: "The repository's actions sit together at the end of Edit, and move only among themselves" },
  { press: tile("Done"), say: "Done saves it to lendinghome-monolith's file, where its runs are kept: On this Mac" },
  { look: tile("Run specs"), say: "The tile is drawn after your own and before Pause" },
  { key: "Escape", on: region(/^Job: Upgrade the loan pricing gem/), say: "Close the Job" },
  { press: inside(list, button("Tidy the deck stylesheet")), say: "A Job in armada" },
  { look: tile("Pause"), say: "Its tiles are your own alone: Run specs is not here" },
]);

export { flightControlsRepository as "flight-controls-repository" };
