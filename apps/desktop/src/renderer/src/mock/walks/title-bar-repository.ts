// Off the Cockpit the title row names the picked repository, or "All repositories". The Cockpit has the
// picker and draws no tag. Over `sessions-by-repository`, which serves two.

import { place, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const titleBarRepository = walk("sessions-by-repository", [
  { look: place("Cockpit"), say: "The Cockpit, where the picker names the repository and the title row draws no tag" },
  toSessions,
  { look: text("All repositories"), say: "On Sessions the title row says All repositories" },
  { press: place("Cockpit"), say: "Back to the Cockpit, where the picker is" },
  { press: text("All repositories"), say: "Pick one repository" },
  { press: role("menuitem", "storefront"), say: "storefront" },
  toSessions,
  { look: text("storefront"), say: "On Sessions the title row names storefront" },
  { press: place("Jobs"), say: "Jobs" },
  { look: text("storefront"), say: "It names storefront there too" },
]);

export { titleBarRepository as "title-bar-repository" };
