// The type, spacing, shadow and motion foundations in six pairings, `?type=system|inter|manrope|instrument|dm|sora`, over the Cockpit.
// Over `cockpit-list-by-repository`. A design mock: nothing here asserts a face or a size.

import { button, inside, region, role, text, walk } from "../walk";

const list = region("Jobs and Sessions");

const typeTakes = walk("cockpit-list-by-repository", [
  { look: inside(list, role("heading", "armada")), say: "Panel headers are small tracked caps" },
  { hover: inside(list, text("The retry loop")), say: "A row under the pointer rises one level" },
  { hover: button("Repository"), say: "A button under the pointer deepens its shadow" },
  { press: button("Repository"), say: "A menu opens with a short rise" },
  { look: role("menuitem", "storefront"), say: "The menu sits at the overlay level" },
]);

export { typeTakes as "type-takes" };
