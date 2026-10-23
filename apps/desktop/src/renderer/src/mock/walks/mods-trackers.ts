// Tracker mods on the Mods page, beside a theme and a layout: one tried and on, one stale, one refused. Over `mods-trackers`.

import { button, role, toSystemPage, walk } from "../walk";

const modsTrackers = walk("mods-trackers", [
  ...toSystemPage("Mods", "Mods, with trackers in the list"),
  { look: role("group", "linear"), say: "Tried as it stands, valid and on: a Manifest naming it uses it" },
  { look: role("group", "shortcut"), say: "Edited since its try: stale, so it is not used until tried again" },
  { look: role("group", "plane"), say: "Refused, with Fleet's reason" },
  { press: button("Try shortcut"), say: "Try it again" },
]);

export { modsTrackers as "mods-trackers" };
