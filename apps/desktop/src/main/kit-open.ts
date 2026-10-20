// Open an item of Kit's inventory in the editor, or show it in Finder.
//
// **The renderer names a path, and main acts on it only if a fresh inventory from Fleet names it.**
// A string from a click handler going to the OS would make everything the sandbox holds back
// reachable through one button; the inventory is the set the string has to be a member of, as
// `session-file.ts` does for a Session's ledger. The editor is `open.ts`'s question.

import { shell } from "electron";
import type { KitFileOpened } from "@armada/manifest/api";
import type { KitInventoryRead } from "@armada/screens/src/manifest-kit";

import type { BridgeState } from "../shared/bridge";
import { namesKitPath } from "./kit-names";
import { inEditor } from "./open";

export { namesKitPath };

export async function openKitFile(
  state: BridgeState,
  read: KitInventoryRead,
  path: unknown,
  how: unknown,
): Promise<KitFileOpened> {
  if (typeof path !== "string" || (how !== "editor" && how !== "finder")) return { ok: false, detail: "Not understood." };
  if (!read.ok) return { ok: false, detail: "Fleet did not answer." };
  if (!namesKitPath(read.setup, path)) return { ok: false, detail: "Kit does not list that." };
  if (how === "finder") {
    shell.showItemInFolder(path);
    return { ok: true };
  }
  const refused = await inEditor(state, path);
  return refused === null ? { ok: true } : { ok: false, detail: refused };
}
