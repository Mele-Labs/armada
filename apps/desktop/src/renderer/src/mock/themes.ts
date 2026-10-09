// Mock-only: the mods on this machine, held in memory until Fleet serves the mod folder; the
// catalogue is the real one. A mod is a `theme.css` as it would ship: one `[data-theme="<name>"]`
// block over the tokens.

import { createThemeSource } from "@armada/settings";
import type { ThemeMod } from "@armada/settings";

import { CATALOGUE } from "../catalogue";
import { BUILT_IN_SWATCHES, modSwatch } from "../swatches";

// Typed by hand, as `catalogue.ts` does: Vite's client types are not loaded here.
const FILES = (
  import.meta as ImportMeta & {
    glob(pattern: string, options: { eager: true; query: string; import: string }): Record<string, string>;
  }
).glob("../../../../../../packages/tokens/themes/example-mod/dusk/theme.css", { eager: true, query: "?raw", import: "default" });

/** The theme a Session writes into the mod folder in the `mods-themes` scenario. */
const DUSK_CSS = Object.values(FILES)[0] ?? "";
export const DUSK_MOD: ThemeMod = { name: "dusk", title: "Dusk", enabled: true, swatch: modSwatch(DUSK_CSS), load: () => Promise.resolve(DUSK_CSS) };

/** The catalogue and no mods, Dark in force: where every mock window starts. */
export const mockThemes = createThemeSource(() => ({ mods: [], catalogue: CATALOGUE, active: "dark", swatches: BUILT_IN_SWATCHES }));
