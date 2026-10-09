// The preview colours of the themes that are no catalogue entry: Dark and Light from
// `packages/tokens/themes/built-in.json`, which the generator writes from the token set and
// `light.css`, and a mod from the CSS Fleet checked, over Dark's values. `swatch.mjs` says which
// tokens, so every theme is previewed by the same eight.

import BUILT_IN from "@armada/tokens/themes/built-in.json";
import TOKENS from "@armada/tokens/tokens.json";
import { swatchOf } from "@armada/tokens/themes/swatch.mjs";

/** What the token set says every token is before a theme moves it. */
const DARK_TOKENS: Record<string, string> = Object.fromEntries(TOKENS.tokens.map((token) => [token.name, token.value]));

export const BUILT_IN_SWATCHES: Readonly<Record<string, readonly string[]>> = { dark: BUILT_IN.dark, light: BUILT_IN.light };

/** A mod's strip, from the stylesheet Fleet returned. A sheet that names no colour for a token leaves Dark's. */
export const modSwatch = (css: string): readonly string[] => swatchOf(css, DARK_TOKENS);
