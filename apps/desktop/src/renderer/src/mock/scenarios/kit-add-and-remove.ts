// Add and Remove on Kit's Skills and Plugins pages, before Fleet runs either: a plugin or a skill added from a source
// with the commands shown first, one whose command fails, a skill to the Trash and back, and a plugin removed with what
// it brought. The walk `kit-add-and-remove` plays it.

import { manifesting, PLUGIN_KIT_INVENTORY } from "../manifest-fake";
import type { Scenario } from "../moment";

/** A source whose marketplace cannot be cloned, and what the command prints. */
export const MISSING_SOURCE = "example/no-such-plugin";

export const kitAddAndRemove: Scenario = {
  ...manifesting({
    kitInventory: PLUGIN_KIT_INVENTORY,
    kitPieces: {
      failing: {
        [MISSING_SOURCE]: `Cloning into '/Users/user/.agent/plugins/marketplaces/no-such-plugin'...\nfatal: repository 'https://github.com/${MISSING_SOURCE}.git/' not found\nError: Failed to add marketplace: git clone exited 128`,
      },
    },
  }),
  name: "kit-add-and-remove",
  says: "Kit: Add a plugin or a skill from a source, one that fails, and Remove on skills and plugins",
};
