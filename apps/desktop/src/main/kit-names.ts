// Whether Kit's inventory names a path. Out of `kit-open.ts` so the file panel can ask without loading Electron.

import { kitSourcePath } from "@armada/protocol";
import type { KitInventory } from "@armada/protocol";

/** The file a skill's folder opens: the one the harness reads it from. */
export const SKILL_FILE = "SKILL.md";

/**
 * Whether the inventory names `path` as where an item of its own is. The allowlist names commands, not files.
 * **A skill is a folder, so its `SKILL.md` is named too**: the folder is what the inventory lists and that file is what opens.
 */
export function namesKitPath(setup: KitInventory, path: string): boolean {
  return setup.kinds.some(
    (one) =>
      one.kind !== "allowlist" &&
      one.read.what === "read" &&
      one.read.items.some((item) => {
        const source = kitSourcePath(item.source, setup.home);
        return source !== null && (source === path || (one.kind === "skills" && `${source}/${SKILL_FILE}` === path));
      }),
  );
}
