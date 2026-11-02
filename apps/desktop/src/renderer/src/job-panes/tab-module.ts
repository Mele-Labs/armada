// A tab mod's code, loaded, and an instrument mod's `instrument.js`, which is the same kind of module. `tab.js` is an ES module whose default export is a React component, run in this renderer as part of Bridge, handed `host`. **It is not sandboxed:**
// a tab can do what Armada's own code can, so it reaches the whole Fleet API through `host.armada`, and a module in an isolated view would have had to be given the same by proxy.
// It is run from the text `validate_mod` passed and no other. The module is loaded from a `blob:`, which the window's policy admits for scripts, so it has no path to `import` from.

import type { ComponentType } from "react";
import type { LucideIcon } from "lucide-react";
import type * as React from "react";

import type { TabUi } from "./tab-ui";

/** What a tab is handed, in place of an `import`. */
export type TabHost = {
  React: typeof React;
  /** The whole Fleet API, as Bridge's own code has it. */
  armada: Window["armada"];
  /** Bridge's primitives, by name (`tab-ui.ts`). */
  ui: TabUi;
  /** Bridge's icons, by their name in the registry. */
  glyphs: Readonly<Record<string, LucideIcon>>;
  /** The Job the tab is open in, and the tab. */
  job: {
    id: string;
  };
  tab: {
    id: string;
  };
};

export type TabComponent<Host = TabHost> = ComponentType<{
  host: Host;
}>;

/** Runs `code` as a module and answers what it exports. Swapped for another in a test, which has no `blob:` to import. */
export type Importer = (code: string) => Promise<{
  default?: unknown;
}>;

export const importBlob: Importer = async (code) => {
  const url = URL.createObjectURL(
    new Blob(
      [
        code,
      ],
      {
        type: "text/javascript",
      },
    ),
  );
  try {
    return (await import(/* @vite-ignore */ url)) as {
      default?: unknown;
    };
  } finally {
    URL.revokeObjectURL(url);
  }
};

/** The module's component, or the reason it could not be had, naming its `file`. Every load is a new module, so a tab the Mod Job rewrote is a new component. */
export async function loadTabModule<Host = TabHost>(
  code: string,
  run: Importer = importBlob,
  file = "tab.js",
): Promise<TabComponent<Host>> {
  let loaded: {
    default?: unknown;
  };
  try {
    loaded = await run(code);
  } catch (why) {
    throw new Error(`${file} did not load: ${why instanceof Error ? why.message : String(why)}`);
  }
  if (typeof loaded.default !== "function")
    throw new Error(`${file} has no component as its default export`);
  return loaded.default as TabComponent<Host>;
}
