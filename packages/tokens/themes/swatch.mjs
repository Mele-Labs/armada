// The strip of colours a theme is previewed by in the Theme field: its ground, a surface, its text, its
// accent and four status hues, read out of the theme's own tokens. One definition, used by the
// generator for the catalogue, by Bridge for a mod's checked CSS, and for Dark and Light from the
// files that define them, so a preview cannot drift from the theme it previews.

/** In the order the strip is drawn. Changing a name here changes what every theme is previewed by. */
export const SWATCH_TOKENS = [
  "--bg-base",
  "--bg-raised",
  "--fg-default",
  "--accent",
  "--status-completed-success",
  "--status-completed-failed",
  "--status-awaiting-review",
  "--status-running",
];

/** A colour that can be painted as it stands: no `var()`, nothing that fetches, nothing that ends a declaration. */
const PLAIN = /^(#[0-9a-f]{3,8}|(rgb|rgba|hsl|hsla|oklch|oklab|lab|lch)\([0-9a-z.,%+\-/ ]*\))$/i;

/** Every `--name: value` a stylesheet declares, later ones over earlier ones, comments ignored. */
export function declaredIn(css) {
  const declared = {};
  const bare = css.replace(/\/\*[\s\S]*?\*\//g, "");
  for (const found of bare.matchAll(/(--[\w-]+)\s*:\s*([^;{}]+);/g)) declared[found[1]] = found[2].trim();
  return declared;
}

/**
 * The strip for `css`. A token the sheet does not declare, or declares as something other than a
 * plain colour, takes `base`'s value: a mod re-skins the Dark defaults, and Light overrides a Dark
 * that is not written out again. Throws where a token has no colour from either.
 */
export function swatchOf(css, base = {}) {
  const declared = declaredIn(css);
  return SWATCH_TOKENS.map((name) => {
    for (const value of [declared[name], base[name]]) if (value !== undefined && PLAIN.test(value)) return value;
    throw new Error(`no colour for ${name}`);
  });
}
