// Add and Remove for skills and plugins, as a mock Fleet would answer them before the real one runs any: a look finds
// what a scenario says a source is, an install puts its rows into the inventory, a skill removed goes to a pretend Trash that Undo takes it back out of,
// and a plugin removed takes the skills it brought with it. The walk `kit-add-and-remove` plays it.

import type { InstallKitPiece, KitInventory, KitSourceFound, LookAtKitSource, RemoveKitPiece, RestoreKitPiece, SetupItem, SetupKind } from "@armada/protocol";
import type { KitInstallRead, KitLookRead, KitRemoveRead, KitRestoreRead } from "@armada/screens/src/manifest-kit";

type Placed = Exclude<KitSourceFound, { found: "unknown" }>;

/** What the fake's looks and installs come to. */
export type KitPiecesFake = {
  /** What a look finds, by the source as pasted. Any other source is found unknown. */
  found?: Readonly<Record<string, KitSourceFound>>;
  /** A repository whose install fails, with what the failing command printed. `at` is which command fails, the first by default; `takenBack` is what Fleet undid. */
  failing?: Readonly<Record<string, { error: string; at?: number; takenBack?: string[] }>>;
  /** Pressed Add, with what was sent. */
  onInstall?: (body: InstallKitPiece) => void;
};

/** The harness's own spelling, as the fake's Fleet answers for it. */
const uninstallOf = (plugin: string): string => `claude plugin uninstall ${plugin}`;

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
  lookAtKitSource: (body: LookAtKitSource) => Promise<KitLookRead>;
  installKitPiece: (body: InstallKitPiece) => Promise<KitInstallRead>;
  removeKitPiece: (body: RemoveKitPiece) => Promise<KitRemoveRead>;
  restoreKitPiece: (body: RestoreKitPiece) => Promise<KitRestoreRead>;
} {
  // Where each skill sat, so Undo puts the row back where it was.
  const trash = new Map<string, { item: SetupItem; index: number }>();
  const placed = (repository: string, kind: string): Placed | undefined =>
    Object.values(options.found ?? {}).find((one): one is Placed => one.found === kind && "repository" in one && one.repository === repository);
  return {
    lookAtKitSource: async (body) => ({
      ok: true,
      found: options.found?.[body.source.trim()] ?? { found: "unknown", why: "Neither a plugin marketplace nor a skill was found at this source." },
    }),
    installKitPiece: async (body) => {
      options.onInstall?.(body);
      const what = placed(body.source, body.kind);
      if (what === undefined) return { ok: false, outcome: { ok: false, why: "not_connected" } };
      const fails = options.failing?.[body.source];
      if (fails !== undefined) {
        const command = what.commands[fails.at ?? 0] ?? "";
        return { ok: true, installed: { state: "failed", command, exit_code: 1, error: fails.error, taken_back: fails.takenBack ?? [] } };
      }
      const name = what.name;
      let next = inventory.get();
      const home = next.home;
      if (body.kind === "skill") {
        const skill: SetupItem = { name, source: `${home}/skills/${name}` };
        next = withItems(next, "skills", [...itemsOf(next, "skills").filter((one) => one.name !== name), skill]);
      } else {
        const folder = `${home}/plugins/cache/${name}/1.0.0`;
        const full = `${name}@${what.found === "plugin" ? what.marketplace : name}`;
        const plugin: SetupItem = { name: full, says: "version 1.0.0", source: folder, uninstall: uninstallOf(full) };
        const skills = (what.found === "plugin" ? what.skills : []).map((one): SetupItem => ({ name: one, source: `${folder}/skills/${one}`, plugin: name }));
        next = withItems(next, "plugins", [...itemsOf(next, "plugins").filter((one) => one.name !== plugin.name), plugin]);
        const fresh = new Set(skills.map((one) => one.source));
        next = withItems(next, "skills", [...itemsOf(next, "skills").filter((one) => !fresh.has(one.source)), ...skills]);
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
