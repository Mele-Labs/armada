// Interface sounds: a tick as j and k move through the Sessions list, a click on a tab or a switch, and silence once Interface sounds is off.
// Which take plays is `?sounds=tick|tap|knock|pebble|wood|glass`. Over the `cockpit-sessions-list` scenario.

import { place, region, role, toSystemPage, walk } from "../walk";

const list = region("Jobs", { exact: true });

const uiSounds = walk("cockpit-sessions-list", [
  { key: "j", on: list, say: "j moves down the list and ticks" },
  { key: "j", on: list, say: "Again" },
  { key: "k", on: list, say: "k moves up and ticks" },
  ...toSystemPage("Settings", "Pressing a tab or a button clicks, a touch lower than the tick"),
  { press: role("tab", "Features"), say: "Features, a tab" },
  { press: role("switch", /^Interface sounds/), say: "Interface sounds off. This press is the last sound" },
  { press: place("Cockpit"), say: "Back to the Cockpit. Silent" },
  { key: "j", on: list, say: "j moves the selection and nothing sounds" },
]);

export { uiSounds as "ui-sounds" };
