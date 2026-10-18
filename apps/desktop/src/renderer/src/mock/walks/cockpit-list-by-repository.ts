// The flight deck's Sessions list on All repositories is headed by repository, in the listing's order, with
// Sessions in several states under each. One repository picked draws no heading. Over `cockpit-list-by-repository`.

import { button, inside, region, role, text, walk } from "../walk";

const list = region("Jobs", { exact: true });

const cockpitListByRepository = walk("cockpit-list-by-repository", [
  { look: inside(list, role("heading", "armada")), say: "On All, armada is the first heading, in the repository listing's order" },
  { look: inside(list, text("Draft the migration notes")), say: "Its Jobs are under it" },
  { look: inside(list, role("heading", "storefront")), say: "storefront is the next heading, with its own rows" },
  { press: button("Repository"), say: "Pick one repository" },
  { press: role("menuitem", "storefront"), say: "storefront" },
  { look: inside(list, text("Round the cart totals")), say: "The list has no heading" },
]);

export { cockpitListByRepository as "cockpit-list-by-repository" };
