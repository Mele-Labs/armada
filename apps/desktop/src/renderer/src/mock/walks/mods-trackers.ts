// Tracker mods in the Mods list beside a theme and a layout: one tried and on, one edited since its try, one refused. Over `mods-trackers`.

import { button, place, role, systemPage, walk } from "../walk";

const modsTrackers = walk("mods-trackers", [
  { press: place("System"), say: "System" },
  { press: systemPage("All"), say: "Mods, every kind in one list" },
  { look: role("group", "linear"), say: "Tried as it stands, valid and on: a Manifest naming it uses it" },
  { look: role("group", "shortcut"), say: "Edited since its try: stale, not used until tried again" },
  { look: role("group", "plane"), say: "Refused, with Fleet's reason" },
  { press: button("Try shortcut"), say: "Try it again" },
  { press: systemPage("Tracker"), say: "Tracker, the same rows on their own" },
]);

export { modsTrackers as "mods-trackers" };
