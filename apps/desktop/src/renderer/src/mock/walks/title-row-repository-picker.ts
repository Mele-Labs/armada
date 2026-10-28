// The repository picker lives in the title row: the name is the picker, the Manifest's icon follows it, and both stay on every screen.

import { button, place, role, walk } from "../walk";

const titleRowRepositoryPicker = walk("default", [
  { look: button("Repository"), say: "On the Cockpit the title row's name is the picker" },
  { look: button("Open the Manifest", { exact: true }), say: "The Manifest's icon follows it" },
  { press: button("Repository"), say: "It opens the list" },
  { press: role("menuitem", "armada"), say: "Pick a repository" },
  { press: button("Open the Manifest", { exact: true }), say: "The Manifest" },
  { press: place("Logbook"), say: "Another screen" },
  { look: button("Repository"), say: "The picker is still there" },
  { look: button("Open the Manifest", { exact: true }), say: "And the Manifest's icon" },
]);

export { titleRowRepositoryPicker as "title-row-repository-picker" };
