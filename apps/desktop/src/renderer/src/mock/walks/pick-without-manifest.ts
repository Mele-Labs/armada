// A repository with no Manifest is picked like any other: its Cockpit, no Setup, and Dispatch opens with it offered.
// Over `nothing-set-up`, two repositories served and neither set up.

import { button, inside, place, role, walk } from "../walk";

const picker = role("button", "Repository", { exact: true });
const frame = role("region", "Dispatch", { exact: true });

const pickWithoutManifest = walk("nothing-set-up", [
  { press: picker, say: "The repository picker opens" },
  { press: role("menuitem", /^scratch/), say: "scratch has no Manifest" },
  { look: place("Cockpit"), say: "Its Cockpit, not Setup" },
  { press: inside(role("region", "Dispatch controls"), button("Dispatch")), say: "Dispatch" },
  { look: frame, say: "Dispatch opens" },
  { look: inside(frame, role("radio", "Repository scratch")), say: "scratch is offered, and so is notes" },
]);

export { pickWithoutManifest as "pick-without-manifest" };
