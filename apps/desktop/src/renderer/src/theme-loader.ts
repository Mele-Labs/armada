// Puts a theme on the document: `data-theme` on <html>, and for a mod's theme its `theme.css`.
//
// **A constructable stylesheet, not a `<style>` element.** The window's CSP is `default-src 'self'`,
// which refuses an inline `<style>`, and loosening it for mods is a security review rather than a
// local choice (`docs/practices/bridge.md`, Security posture). `adoptedStyleSheets` is CSSOM, so the
// same CSS applies without touching the policy. The sheet's cascade position is the end of the
// document, after every token, which is where a theme must sit to win at equal specificity.
//
// Nothing here throws. A theme that cannot be applied is Dark, and Bridge keeps running.

import type { ThemeState } from "@armada/settings";

const DARK = "dark";
const BUILT_IN = new Set(["dark", "light"]);

/**
 * CSS a mod may ship: style rules only, each scoped to the mod's own `[data-theme]`, fetching
 * nothing. Scoping is what keeps a mod to its theme: a bare `body { … }` would restyle Bridge
 * whichever theme is in force.
 */
function sheetOf(doc: Document, name: string, css: string): CSSStyleSheet {
  if (/@import/i.test(css)) throw new Error("a theme may not @import");
  const sheet = new (doc.defaultView ?? window).CSSStyleSheet();
  sheet.replaceSync(css);
  if (sheet.cssRules.length === 0) throw new Error("a theme with no rules");
  const own = `[data-theme="${name}"]`;
  for (const rule of Array.from(sheet.cssRules)) {
    if (!(rule instanceof CSSStyleRule) || !rule.selectorText.includes(own)) throw new Error(`a theme rule outside ${own}`);
  }
  return sheet;
}

export type ThemeLoader = {
  /** Apply what the state asks for and return the theme in force: the asked-for id, or Dark. */
  apply(state: ThemeState): string;
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
    apply: (state) => {
      try {
        const { active } = state;
        if (BUILT_IN.has(active)) {
          drop();
          return show(active);
        }
        const mod = state.mods.find((one) => one.name === active && one.enabled);
        if (mod === undefined) return fallBack(`no enabled mod named ${active}`);
        const next = sheetOf(doc, mod.name, mod.css);
        drop();
        mine = next;
        doc.adoptedStyleSheets = [...doc.adoptedStyleSheets, next];
        return show(active);
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
