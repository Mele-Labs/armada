// A Kit item's file is the detail: a skill's SKILL.md or a sub agent's file is shown whole in the editor, edited and
// saved where it is read, and a plugin, a folder, offers Show in Finder. The walk `kitEditsInPlace` plays it.

import { KIT_SERVERS, manifesting, PLUGIN_KIT_INVENTORY } from "../manifest-fake";
import type { Scenario } from "../moment";

export const kitEditsInPlace: Scenario = {
  ...manifesting({ kitInventory: PLUGIN_KIT_INVENTORY, kitServers: KIT_SERVERS }),
  name: "kit-edits-in-place",
  says: "Kit: a skill's SKILL.md or a sub agent's file is edited in the detail, and a plugin offers Show in Finder",
};
