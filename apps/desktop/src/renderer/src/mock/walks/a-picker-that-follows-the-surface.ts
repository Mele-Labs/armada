// The title row's picker says where it applies: a repository's surface keeps
// it live, a surface about this machine greys it and names the machine. Kit is half
// each, so it keeps the picker live.

import { button, place, toSystemPage, walk } from "../walk";

export const aPickerThatFollowsTheSurface = walk("default", [
  { look: button("All repositories"), say: "On a repository's surface the picker is live" },
  { press: button("Settings"), say: "Settings is this machine's, not a repository's" },
  { look: button("This machine"), say: "The picker says so, and is greyed: a pick changes nothing here" },
  { press: place("Cockpit"), say: "Back on the Cockpit" },
  { look: button("All repositories"), say: "The picker is live again, on the pick it had" },
  ...toSystemPage("Kit", "Kit hands a Drone servers per repository, so it reads the pick"),
  { look: button("All repositories"), say: "The picker stays live on Kit" },
]);
