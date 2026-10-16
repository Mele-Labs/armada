// Kit on a machine with output styles: the owner's own active, a plugin's, a repository's, a built-in
// one, and Armada's session rules under them. The walk `output-styles` plays it.

import { KIT_INVENTORY, manifesting } from "../manifest-fake";
import type { Scenario } from "../moment";

function build(): Scenario {
  const base = manifesting({ kitInventory: KIT_INVENTORY });
  return {
    ...base,
    name: "output-styles",
    says: "Kit's Output styles: where each style came from, the active one marked, and Armada's session rules set apart",
  };
}

export const outputStyles: Scenario = build();
