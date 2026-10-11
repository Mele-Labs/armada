// The title row's picker says where it applies: a repository's surface keeps
// it live, a surface about this machine greys it and names the machine.

import { button, walk } from "../walk";

export const aPickerThatFollowsTheSurface = walk("every-state", [
  { look: button("All repositories"), say: "On a repository's surface the picker is live" },
  { press: button("Kit"), say: "Kit is what this machine has, not a repository's" },
  { look: button("This machine"), say: "The picker says so, and is greyed: a pick changes nothing here" },
  { press: button("Cockpit"), say: "Back on the Cockpit" },
  { look: button("All repositories"), say: "The picker is live again, on the pick it had" },
]);
