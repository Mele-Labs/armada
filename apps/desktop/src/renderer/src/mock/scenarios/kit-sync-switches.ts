// Kit's Plugins with every kind off in What syncs: a plugin is switched on by itself from its detail
// and added to the home, then Sync all turns the whole kind on. The walk `kit-sync-switches` plays it.

import type { KitInventory } from "@armada/protocol";
import type { SharedRow } from "@armada/components";

import { KIT_INVENTORY, manifesting } from "../manifest-fake";
import type { Scenario } from "../moment";

const PLUGINS = "/Users/user/.agent/plugins/cache";

const INVENTORY: KitInventory = {
  ...KIT_INVENTORY,
  harness: "Claude",
  kinds: KIT_INVENTORY.kinds.map((one) =>
    one.kind !== "plugins"
      ? one
      : {
          kind: "plugins",
          read: {
            what: "read",
            items: [
              { name: "impeccable@impeccable", says: "version 4.7.0", source: `${PLUGINS}/impeccable/impeccable/4.7.0` },
              { name: "github@official", says: "version 1.0.0", source: `${PLUGINS}/official/github/1.0.0` },
              { name: "code-simplifier@official", says: "version 1.0.0", source: `${PLUGINS}/code-simplifier/1.0.0` },
            ],
            unreadable: [],
          },
        },
  ),
};

/** Every plugin row as Fleet would read it with Plugins on; What syncs narrows what is drawn. */
const ROWS: SharedRow[] = [
  { kind: "plugins", name: "impeccable@impeccable", state: "here" },
  { kind: "plugins", name: "github@official", state: "here" },
  { kind: "plugins", name: "code-simplifier@official", state: "same" },
];

function build(): Scenario {
  const base = manifesting({ kitInventory: INVENTORY });
  return {
    ...base,
    name: "kit-sync-switches",
    says: "Kit's Plugins with every kind off in What syncs: one plugin switched on by itself, then the whole kind",
    draft: { ...(base.draft ?? {}), sharedHome: { account: "nickmele", repository: "armada-home", rows: ROWS, syncs: { kinds: [], items: [] } } },
  };
}

export const kitSyncSwitches: Scenario = build();
