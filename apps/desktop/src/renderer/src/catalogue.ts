// The themes Bridge ships, from `packages/tokens/themes/`: an index read up front (a title, a tone
// and an id each) and one CSS file per theme read when it is chosen, so no theme's block is in the
// initial stylesheet. `more/` holds the rest of a collection a machine generated with
// `generate.mjs --all`; it is ignored by git and read the same way when it is there.

import type { CatalogueTheme } from "@armada/settings";

type Entry = { id: string; title: string; tone: "dark" | "light"; source: string; swatch?: string[] };

// Typed by hand, as `mock/scenario.ts` does: Vite's client types are not loaded here.
type Globbing = ImportMeta & {
  glob(pattern: string[], options: { eager?: boolean; query?: string; import?: string }): Record<string, unknown>;
};

const INDEXES = (import.meta as Globbing).glob(["../../../../../packages/tokens/themes/index.json", "../../../../../packages/tokens/themes/more/index.json"], { eager: true, import: "default" }) as Record<string, Entry[]>;
const FILES = (import.meta as Globbing).glob(["../../../../../packages/tokens/themes/css/*.css", "../../../../../packages/tokens/themes/more/css/*.css"], { query: "?raw", import: "default" }) as Record<string, () => Promise<string>>;

/** The file for a theme, by the slug its id ends in. */
const fileOf = (id: string) => Object.entries(FILES).find(([path]) => path.endsWith(`/css/${id.slice("catalogue:".length)}.css`))?.[1];

export const CATALOGUE: readonly CatalogueTheme[] = Object.values(INDEXES)
  .flat()
  .filter((one, at, all) => all.findIndex((other) => other.id === one.id) === at)
  .flatMap((one) => {
    const file = fileOf(one.id);
    return file === undefined ? [] : [{ id: one.id, title: one.title, tone: one.tone, ...(one.swatch === undefined ? {} : { swatch: one.swatch }), load: file }];
  });
