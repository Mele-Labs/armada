// The Cockpit's map draws only the repository picked. On All it draws every territory; one picked, only that one, and the
// camera eases to fit it. Over the `cockpit-list-by-repository` scenario, two repositories served.

import { button, inside, role, walk } from "../walk";

const map = role("listbox", "Map");

const cockpitMapOneRepository = walk("cockpit-list-by-repository", [
  { look: inside(map, role("option", "Trim the feed")), say: "On All, the map draws armada's drones" },
  { look: inside(map, role("option", "Resize the product images")), say: "And storefront's, on its own territory" },
  { press: button("Repository"), say: "Pick one repository" },
  { press: role("menuitem", "storefront"), say: "storefront" },
  { look: inside(map, role("option", "Resize the product images")), say: "The map draws only storefront's territory, and the camera has eased to fill the pane with it" },
  { press: button("Repository"), say: "Switch to the other" },
  { press: role("menuitem", "armada"), say: "armada" },
  { look: inside(map, role("option", "Trim the feed")), say: "The camera eases to armada's territory; storefront's is gone from the map" },
  { press: button("Repository"), say: "Clear the selection" },
  { press: role("menuitem", "All repositories"), say: "All repositories" },
  { look: inside(map, role("option", "Resize the product images")), say: "Both territories are drawn again, as before" },
]);

export { cockpitMapOneRepository as "cockpit-map-one-repository" };
