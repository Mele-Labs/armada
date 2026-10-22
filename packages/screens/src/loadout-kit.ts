// What Kit lets a Loadout's skill rows do, matched against Kit's inventory. A Loadout row carries no path, and the
// inventory names a plugin's skill without the plugin's prefix the Loadout spells it with, so a row is matched by
// name and plugin, and one the inventory names twice or not at all offers nothing.

import { kitSourcePath } from "@armada/protocol";
import type { KitInventory, RemoveKitPiece, SetupItem, SetupKind } from "@armada/protocol";
import type { LoadoutRef, LoadoutSkillActs } from "@armada/components";
import { loadoutKeyOf } from "@armada/components/src/loadout";

const itemsOf = (setup: KitInventory, kind: SetupKind): SetupItem[] => {
  const found = setup.kinds.find((one) => one.kind === kind);
  return found?.read.what === "read" ? found.read.items : [];
};

/** The plugin a Loadout row came with, and its name as the inventory spells it. */
function inventoryNameOf(item: LoadoutRef): { name: string; plugin?: string } | undefined {
  if (item.source === "kit") return { name: item.name };
  if (item.source !== "plugin") return undefined;
  const plugin = item.from ?? item.name.split(":")[0];
  if (plugin === undefined || plugin === "") return undefined;
  return { name: item.name.startsWith(`${plugin}:`) ? item.name.slice(plugin.length + 1) : item.name, plugin };
}

/** A skill row Kit holds: what it offers, and the body Remove sends. */
export type KitSkillMatch = { acts: LoadoutSkillActs; remove?: RemoveKitPiece };

function matchOf(item: LoadoutRef, setup: KitInventory): KitSkillMatch | undefined {
  if (item.kind !== "skill") return undefined;
  const named = inventoryNameOf(item);
  if (named === undefined) return undefined;
  const found = itemsOf(setup, "skills").filter((one) => one.name === named.name && one.plugin === named.plugin);
  const path = found.length === 1 ? kitSourcePath(found[0]!.source, setup.home) : null;
  if (path === null) return undefined;
  const edits = `${path}/SKILL.md`;
  // A plugin's own skill goes with its plugin and never alone.
  if (named.plugin !== undefined) return { acts: { edits } };
  return { acts: { edits, removes: true }, remove: { kind: "skill", name: named.name } };
}

/** What each skill row offers, by `loadoutKeyOf`. A row Kit does not hold is absent. */
export function skillActsByKit(items: readonly LoadoutRef[], setup: KitInventory): Record<string, KitSkillMatch> {
  const acts: Record<string, KitSkillMatch> = {};
  for (const item of items) {
    const one = matchOf(item, setup);
    if (one !== undefined) acts[loadoutKeyOf(item)] = one;
  }
  return acts;
}
