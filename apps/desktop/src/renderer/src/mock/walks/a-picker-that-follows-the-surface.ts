// The title row's picker is live on the Cockpit, on Kit and back: a repository's surface keeps it, and the pick it had.

import { button, place, toSystemPage, walk } from "../walk";

export const aPickerThatFollowsTheSurface = walk("default", [
  { look: button("Repository", { exact: true }), say: "On the Cockpit the picker is live" },
  ...toSystemPage("Kit", "Kit hands a Drone servers per repository, so it reads the pick"),
  { look: button("Repository", { exact: true }), say: "The picker stays live on Kit" },
  { press: place("Cockpit"), say: "Back on the Cockpit" },
  { look: button("Repository", { exact: true }), say: "The picker is live again, on the pick it had" },
]);
