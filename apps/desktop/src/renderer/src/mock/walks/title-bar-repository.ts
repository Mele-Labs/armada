// Off the Cockpit the title row names the picked repository, or "All repositories". The Cockpit has the
// picker and draws no tag. Over `sessions-by-repository`, which serves two.

import { place, role, text, walk } from "../walk";

const titleBarRepository = walk("sessions-by-repository", [
  { look: place("Cockpit"), say: "The Cockpit, where the picker names the repository and the title row draws no tag" },
  { press: place("Logbook"), say: "Logbook" },
  { look: text("All repositories"), say: "On Retros the title row says All repositories" },
  { press: place("Cockpit"), say: "Back to the Cockpit, where the picker is" },
  { press: text("All repositories"), say: "Pick one repository" },
  { press: role("menuitem", "storefront"), say: "storefront" },
  { press: place("Logbook"), say: "Logbook" },
  { look: text("storefront"), say: "On Retros the title row names storefront" },
  { press: place("Studios"), say: "Studios" },
  { look: text("storefront"), say: "It names storefront there too" },
]);

export { titleBarRepository as "title-bar-repository" };
