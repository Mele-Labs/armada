// The Rules page over what `teach-anything` leaves: a Jira key and a batched Slot behind main notice taught in the first Job, and
// the three repositories' Jira sites known. Each teaching is a rules mod, switched on or off in Mods. Mock only. The walk
// `teach-mods` plays it; `mount.tsx` seeds the teachings.

import type { Scenario } from "../moment";
import { teachAnything } from "./teach-anything";

export const teachMods: Scenario = {
  ...teachAnything,
  name: "teach-mods",
  says: "What was taught is kept as rules mods: opened from the Logbook, read in Mods, switched off and on",
};
