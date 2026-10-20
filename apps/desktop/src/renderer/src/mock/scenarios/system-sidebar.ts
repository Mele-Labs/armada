// System as one sidebar page: Kit with plugin-provided skills, sub agents and commands, Kit's own
// servers, and a theme mod on this machine. The walk `systemSidebar` plays it. Mod state is mock-only
// (`../themes.ts`).

import { KIT_SERVERS, manifesting, PLUGIN_KIT_INVENTORY } from "../manifest-fake";
import type { Scenario } from "../moment";
import { DUSK_MOD, mockThemes } from "../themes";

function build(): Scenario {
  const base = manifesting({ kitInventory: PLUGIN_KIT_INVENTORY, kitServers: KIT_SERVERS });
  return {
    ...base,
    name: "system-sidebar",
    says: "System: one sidebar for Kit, Fleet and Bridge, Kit's items by kind, and a theme mod under Theme",
    behaves: (handle) => {
      mockThemes.reset();
      mockThemes.install(DUSK_MOD);
      return base.behaves?.(handle) ?? {};
    },
  };
}

export const systemSidebar: Scenario = build();
