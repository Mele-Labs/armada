// The Mods page over what `teach-anything` leaves: a Jira key and a batched Slot behind main notice taught in the first Job, the three
// repositories' Jira sites known, and the second repository's Job asking the person something, so a row needs him. Teaching on Bridge
// itself is played from there: a row made louder, a panel hidden. Mock only. The walk `teach-mods` plays it; `mount.tsx` seeds the teachings.

import type { Scenario } from "../moment";
import { build } from "./teach-anything";

export const teachMods: Scenario = {
  ...build(true),
  name: "teach-mods",
  says: "Mods split by what they change: words, Fleet, theme tweaks, layout, and teaching Bridge itself",
};
