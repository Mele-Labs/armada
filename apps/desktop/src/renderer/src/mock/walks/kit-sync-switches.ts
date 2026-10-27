// Kit's Plugins with every plugin off in What syncs: impeccable is switched on by itself from its detail and
// goes to the home at once; Sync all then sends every plugin. Over `kit-sync-switches`.

import { button, inside, region, role, toSystemPage, walk } from "../walk";

const PLUGINS = region("Plugins");
const DETAIL = region("Detail");

const kitSyncSwitches = walk("kit-sync-switches", [
  ...toSystemPage("Plugins", "Plugins in Kit"),
  { look: inside(PLUGINS, role("switch", "Sync all")), say: "Sync all is off for Plugins, so no plugin goes to the home" },
  { press: inside(PLUGINS, button(/^impeccable/)), say: "impeccable" },
  { press: inside(DETAIL, role("switch", "Sync to home")), say: "Sync impeccable on its own: it goes to the home now" },
  { look: inside(PLUGINS, role("img", "Same as the home", { exact: true })), say: "impeccable is in the home" },
  { press: inside(PLUGINS, button(/^github/)), say: "github is not synced, so it has no mark" },
  { press: inside(PLUGINS, role("switch", "Sync all")), say: "Sync all sends every plugin to the home" },
  { look: inside(DETAIL, role("switch", "Sync to home")), say: "github's own switch is on and fixed while Sync all is on" },
]);

export { kitSyncSwitches as "kit-sync-switches" };
