// Kit's Open on a skill and a sub agent shows the file in the side panel's editor; a plugin is a folder with no
// single file and keeps Show in Finder. Over the `system-sidebar` scenario's Kit. The walk `kitOpensInPanel` plays it.

import { KIT_SERVERS, manifesting, PLUGIN_KIT_INVENTORY } from "../manifest-fake";
import type { Scenario } from "../moment";

export const kitOpensInPanel: Scenario = {
  ...manifesting({ kitInventory: PLUGIN_KIT_INVENTORY, kitServers: KIT_SERVERS }),
  name: "kit-opens-in-panel",
  says: "Kit: Open shows a skill's SKILL.md or a sub agent's file in the side panel, and a plugin offers Show in Finder",
};
