// Kit's Plugins with every plugin off in What syncs: impeccable is switched on by itself from its detail,
// shows it is only on this machine, and is added to the home; Sync all then turns on every plugin. Over
// `kit-sync-switches`.

import { button, inside, region, role, toSystemPage, walk } from "../walk";

const PLUGINS = region("Plugins");
const DETAIL = region("Detail");

const kitSyncSwitches = walk("kit-sync-switches", [
  ...toSystemPage("Plugins", "Plugins in Kit"),
  { look: inside(PLUGINS, role("switch", "Sync all")), say: "Sync all is off for Plugins, so no plugin goes to the home" },
  { press: inside(PLUGINS, button(/^impeccable/)), say: "impeccable" },
  { press: inside(DETAIL, role("switch", "Sync to home")), say: "Sync impeccable on its own" },
  { look: inside(PLUGINS, role("img", "Only on this machine", { exact: true })), say: "impeccable is only on this machine" },
  { press: inside(PLUGINS, button("Add to the home impeccable@impeccable")), say: "Add it to the home" },
  { look: inside(PLUGINS, role("img", "Same as the home", { exact: true })), say: "impeccable is in the home" },
  { press: inside(PLUGINS, role("switch", "Sync all")), say: "Sync all turns on every plugin" },
  { look: inside(DETAIL, role("switch", "Sync to home")), say: "impeccable's own switch is on and fixed while Sync all is on" },
]);

export { kitSyncSwitches as "kit-sync-switches" };
