// Add and Remove for skills and plugins, as a mock Fleet would answer them before the real one runs any: an install
// puts the source's rows into the inventory, a skill removed goes to a pretend Trash that Undo takes it back out of,
// and a plugin removed takes the skills it brought with it. The walk `kit-add-and-remove` plays it.

import type { InstallKitPiece, KitInventory, RemoveKitPiece, RestoreKitPiece, SetupItem, SetupKind } from "@armada/protocol";
import type { KitInstallRead, KitRemoveRead, KitRestoreRead } from "@armada/screens/src/manifest-kit";
import { kitPieceCommands } from "@armada/screens/src/manifest-kit";

/** What the fake's installs come to. A source in `failing` fails its first command with that text. */
export type KitPiecesFake = {
  failing?: Readonly<Record<string, string>>;
  /** Pressed Add, with what was sent. */
  onInstall?: (body: InstallKitPiece) => void;
};

const nameOf = (source: string): string => source.replace(/\.git$/, "").split("/").filter((one) => one !== "").pop() ?? source;

function itemsOf(inventory: KitInventory, kind: SetupKind): SetupItem[] {
  const found = inventory.kinds.find((one) => one.kind === kind);
  return found?.read.what === "read" ? found.read.items : [];
}

function withItems(inventory: KitInventory, kind: SetupKind, items: SetupItem[]): KitInventory {
  return {
    ...inventory,
    kinds: inventory.kinds.map((one) =>
      one.kind === kind && one.read.what === "read" ? { kind, read: { ...one.read, items } } : one,
    ),
  };
}

/** The three acts, over the inventory the rest of the fake reads. */
export function kitPieces(
  options: KitPiecesFake,
  inventory: { get: () => KitInventory; set: (next: KitInventory) => void },
): {
  installKitPiece: (body: InstallKitPiece) => Promise<KitInstallRead>;
  removeKitPiece: (body: RemoveKitPiece) => Promise<KitRemoveRead>;
  restoreKitPiece: (body: RestoreKitPiece) => Promise<KitRestoreRead>;
} {
  // Where each skill sat, so Undo puts the row back where it was.
  const trash = new Map<string, { item: SetupItem; index: number }>();
  return {
    installKitPiece: async (body) => {
      options.onInstall?.(body);
      const error = options.failing?.[body.source];
      if (error !== undefined) {
        const [command = ""] = kitPieceCommands(body.source, body.kind);
        return { ok: true, installed: { state: "failed", command, exit_code: 1, error } };
      }
      const name = nameOf(body.source);
      let next = inventory.get();
      const home = next.home;
      if (body.kind === "skill") {
        const skill: SetupItem = { name, source: `${home}/skills/${name}` };
        next = withItems(next, "skills", [...itemsOf(next, "skills").filter((one) => one.name !== name), skill]);
      } else {
        const folder = `${home}/plugins/cache/${name}/1.0.0`;
        const plugin: SetupItem = { name: `${name}@${name}`, says: "version 1.0.0", source: folder };
        const skill: SetupItem = { name, source: `${folder}/skills/${name}`, plugin: name };
        next = withItems(next, "plugins", [...itemsOf(next, "plugins").filter((one) => one.name !== plugin.name), plugin]);
        next = withItems(next, "skills", [...itemsOf(next, "skills").filter((one) => one.source !== skill.source), skill]);
      }
      inventory.set(next);
      return { ok: true, installed: { state: "installed", setup: next } };
    },
    removeKitPiece: async (body) => {
      let next = inventory.get();
      if (body.kind === "skill") {
        const gone = itemsOf(next, "skills").find((one) => one.name === body.name && one.plugin === undefined);
        if (gone === undefined) return { ok: false, outcome: { ok: false, why: "not_connected" } };
        const at = `/Users/user/.Trash/${body.name}`;
        trash.set(at, { item: gone, index: itemsOf(next, "skills").indexOf(gone) });
        next = withItems(next, "skills", itemsOf(next, "skills").filter((one) => one !== gone));
        inventory.set(next);
        return { ok: true, removed: { setup: next, trashed: at } };
      }
      // A plugin takes every item it brought with it.
      const provider = body.name.split("@")[0] ?? body.name;
      next = withItems(next, "plugins", itemsOf(next, "plugins").filter((one) => one.name !== body.name));
      for (const kind of ["skills", "sub_agents", "commands", "output_styles"] as const) {
        next = withItems(next, kind, itemsOf(next, kind).filter((one) => one.plugin !== provider));
      }
      inventory.set(next);
      return { ok: true, removed: { setup: next, trashed: null } };
    },
    restoreKitPiece: async (body) => {
      const back = trash.get(body.trashed);
      if (back === undefined) return { ok: false, outcome: { ok: false, why: "not_connected" } };
      trash.delete(body.trashed);
      const skills = [...itemsOf(inventory.get(), "skills")];
      skills.splice(back.index, 0, back.item);
      const next = withItems(inventory.get(), "skills", skills);
      inventory.set(next);
      return { ok: true, setup: next };
    },
  };
}
