// System as one sidebar page: Kit with plugin-provided skills, sub agents and commands, Kit's own
// servers, and mods on this machine. The walk `systemSidebar` plays it. The mods are installed
// by `mount.tsx` (`../system-mods.ts`).

import { KIT_SERVERS, manifesting, PLUGIN_KIT_INVENTORY } from "../manifest-fake";
import type { Scenario } from "../moment";

function build(): Scenario {
  const base = manifesting({ kitInventory: PLUGIN_KIT_INVENTORY, kitServers: KIT_SERVERS });
  return {
    ...base,
    name: "system-sidebar",
    says: "System: one sidebar for Kit, Fleet and Bridge, Kit's items by kind, and mods under Theme and Layout",
  };
}

export const systemSidebar: Scenario = build();
