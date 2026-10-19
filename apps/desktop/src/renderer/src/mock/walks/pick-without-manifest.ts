// A repository with no Manifest is picked like any other: its Cockpit, no Setup, and n opens Dispatch with it offered.
// Over `nothing-set-up`, two repositories served and neither set up.

import { button, inside, place, role, walk } from "../walk";

const frame = role("region", "Dispatch", { exact: true });

const pickWithoutManifest = walk("nothing-set-up", [
  { press: button("Repository"), say: "Pick one of the two repositories" },
  { press: role("menuitem", "scratch"), say: "scratch has no Manifest" },
  { look: place("Cockpit"), say: "Its Cockpit, not Setup" },
  { key: "n", on: place("Cockpit"), say: "Press n" },
  { look: frame, say: "Dispatch opens" },
  { look: inside(frame, role("radio", "Repository scratch")), say: "scratch is offered, and so is notes" },
]);

export { pickWithoutManifest as "pick-without-manifest" };
