// The title row's name is the repository picker on every surface, the Cockpit too. Over `sessions-by-repository`, which serves two.

import { button, place, role, walk } from "../walk";

const titleBarRepository = walk("sessions-by-repository", [
  { look: button("Repository", { exact: true }), say: "The Cockpit, where the title row holds the picker" },
  { press: place("Logbook"), say: "Logbook" },
  { look: button("Repository", { exact: true }), say: "On the Logbook the picker is still there" },
  { press: place("Cockpit"), say: "Back to the Cockpit" },
  { press: button("Repository", { exact: true }), say: "Pick one repository" },
  { press: role("menuitem", "storefront"), say: "storefront" },
  { press: place("Logbook"), say: "Logbook" },
  { look: button("Repository", { exact: true }), say: "On the Logbook the picker names storefront" },
  { press: place("Studios"), say: "Studios" },
  { look: button("Repository", { exact: true }), say: "And on Studios" },
]);

export { titleBarRepository as "title-bar-repository" };
