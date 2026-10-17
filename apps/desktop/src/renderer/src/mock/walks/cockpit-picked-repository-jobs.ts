// With one repository picked the flight deck's list drops the other repositories' Jobs, as it does their Sessions.
// Over the `cockpit-list-by-repository` scenario.

import { button, inside, region, role, text, walk } from "../walk";

const list = region("Work", { exact: true });

const cockpitPickedRepositoryJobs = walk("cockpit-list-by-repository", [
  { look: inside(list, text("Trim the feed")), say: "On All, armada's Jobs are listed" },
  { look: inside(list, text("Resize the product images")), say: "And storefront's" },
  { press: button("Repository"), say: "Pick one repository" },
  { press: role("menuitem", "storefront"), say: "storefront" },
  { look: inside(list, text("Resize the product images")), say: "Only storefront's Jobs are listed now; armada's have dropped out" },
]);

export { cockpitPickedRepositoryJobs as "cockpit-picked-repository-jobs" };
