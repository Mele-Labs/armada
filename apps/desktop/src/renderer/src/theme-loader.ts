// Puts a theme on the document: `data-theme` on <html>, and for a mod's or the catalogue's theme its
// CSS block.
//
// **A constructable stylesheet, not a `<style>` element.** The window's CSP is `default-src 'self'`,
// which refuses an inline `<style>`, and loosening it for mods is a security review rather than a
// local choice (`docs/practices/bridge.md`, Security posture). `adoptedStyleSheets` is CSSOM, so the
// same CSS applies without touching the policy. The sheet's cascade position is the end of the
// document, after every token, which is where a theme must sit to win at equal specificity.
//
// Nothing here throws. A theme that cannot be applied is Dark, and Bridge keeps running.

const DARK = "dark";
const BUILT_IN = new Set(["dark", "light"]);

/**
 * CSS a theme may ship: style rules only, fetching nothing. **A rule is the theme's own**, scoped
 * to its `[data-theme]`, or a bare `:root` or `[data-theme]` block, which is what Fleet's checks
 * let a mod write (`docs/concepts/mods.md`). A bare block cannot restyle Bridge whichever theme is
 * in force, because the sheet is adopted only while this theme is, and leaves with it. Anything
 * else, such as a bare `body { … }`, is refused.
 */
function sheetOf(doc: Document, name: string, css: string): CSSStyleSheet {
  if (/@import/i.test(css)) throw new Error("a theme may not @import");
  const sheet = new (doc.defaultView ?? window).CSSStyleSheet();
  sheet.replaceSync(css);
  if (sheet.cssRules.length === 0) throw new Error("a theme with no rules");
  const own = `[data-theme="${name}"]`;
  const scoped = (selector: string) => selector.split(",").every((part) => BARE.has(part.trim()) || part.includes(own));
  for (const rule of Array.from(sheet.cssRules)) {
    if (!(rule instanceof CSSStyleRule) || !scoped(rule.selectorText)) throw new Error(`a theme rule outside ${own}`);
  }
  return sheet;
}

const BARE = new Set([":root", "[data-theme]", ":root[data-theme]"]);

export type ThemeLoader = {
  /**
   * Put `id` on the document, with the CSS that defines it: none for Dark and Light, whose tokens
   * are already loaded, and a mod's or the catalogue's block otherwise. Returns the theme in force:
   * `id`, or Dark when the CSS is missing, refused, or throws.
   */
  apply(id: string, css: string | null): string;
  /** Leave the document as it was found. */
  clear(): void;
};

export function createThemeLoader(doc: Document): ThemeLoader {
  let mine: CSSStyleSheet | null = null;

  const drop = () => {
    if (mine !== null) doc.adoptedStyleSheets = doc.adoptedStyleSheets.filter((one) => one !== mine);
    mine = null;
  };
  const show = (id: string) => {
    doc.documentElement.dataset["theme"] = id;
    return id;
  };
  const fallBack = (why: unknown) => {
    console.warn("theme not applied, using Dark", why);
    drop();
    return show(DARK);
  };

  return {
    apply: (id, css) => {
      try {
        if (BUILT_IN.has(id)) {
          drop();
          return show(id);
        }
        if (css === null) return fallBack(`no theme named ${id}`);
        const next = sheetOf(doc, id, css);
        drop();
        mine = next;
        doc.adoptedStyleSheets = [...doc.adoptedStyleSheets, next];
        return show(id);
      } catch (why) {
        return fallBack(why);
      }
    },
    clear: () => {
      drop();
      delete doc.documentElement.dataset["theme"];
    },
  };
}

/**
 * Safe mode: `?nomods` on the page, which main sets when Bridge starts with `--no-mods`. The
 * mock reads the same query, so a walk or a test can ask for it.
 */
export const skipsMods = (search: string = window.location.search): boolean => {
  const query = new URLSearchParams(search);
  return query.has("nomods") || query.has("no-mods");
};
